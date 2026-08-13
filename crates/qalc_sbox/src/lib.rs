use std::{
	collections::BTreeMap,
	env,
	ffi::OsStr,
	fs,
	io,
	os::unix::fs::PermissionsExt as _,
	path::{Path, PathBuf},
	process,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
		mpsc,
	},
	thread,
};

use anyhow::Context as _;
use ipc_channel::{
	IpcError,
	ipc::{self, IpcOneShotServer, IpcReceiver, IpcSender},
};
use landlock::{
	ABI,
	Access,
	AccessFs,
	Ruleset,
	RulesetAttr,
	RulesetCreatedAttr,
	RulesetStatus,
	path_beneath_rules,
};
use nix::{
	errno::Errno,
	fcntl::{OFlag, open},
	mount::{MntFlags, MsFlags, mount, umount2},
	sched::{CloneFlags, unshare},
	sys::stat::Mode,
	unistd::{close, pivot_root},
};
use qalc::ffi::Qalculator;
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, SeccompRule};
use serde::{Deserialize, Serialize};
use tracing::warn;

pub use ::ipc_channel;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Failed to fork process: {0}")]
	Fork(#[source] io::Error),
	#[error("Failed to spawn worker executable: {0}")]
	Spawn(#[source] io::Error),
	#[error("Failed to create initial IPC oneshot server: {0}")]
	CreateInitIpcServer(#[source] io::Error),
	#[error("Failed to accept initial IPC connection: {0}")]
	AcceptInitIpcServer(#[source] IpcError),
	#[error("Failed to send message to child: {0}")]
	#[expect(private_interfaces)]
	Send(#[source] mpsc::SendError<SandboxMessage>),
	#[error("Failed to receive message from worker thread: {0}")]
	OneshotRecv(#[source] oneshot::RecvError),
	#[error("Failed to join blocking task: {0}")]
	Join(
		#[source]
		#[from]
		tokio::task::JoinError,
	),
}

#[derive(Serialize, Deserialize, Debug)]
enum MsgToChild {
	Eval {
		id: u64,
		content: String,
	},
	/// Ask the child to exit cleanly. Sent when every `Sandbox` handle has
	/// been dropped so the internal channel closed.
	Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
enum MsgToParent {
	EvalResult {
		id: u64,
		result: Result<String, String>,
	},
	/// Sent when the child process has encountered a fatal error and is about to exit
	FatalError { msg: String },
}

enum SandboxMessage {
	Eval {
		content: String,
		tx: oneshot::Sender<Result<String, String>>,
	},
}

#[derive(Debug, Clone)]
/// O(1) clone; refcounted
pub struct Sandbox(Arc<Inner>);

#[derive(Debug)]
struct Inner {
	tx: mpsc::SyncSender<SandboxMessage>,
	/// pid of the forked worker in the host pid namespace (no `CLONE_NEWPID` is
	/// used), so `/proc/<child_pid>/` is readable from the parent.
	child_pid: libc::pid_t,
}

type InitPacket = (IpcSender<MsgToChild>, IpcReceiver<MsgToParent>);

const M_NONE: Option<&'static str> = None;
const PERM_MASK: libc::mode_t = 0o777;

fn bind_allowed_paths(paths: &[&str], jail_dir: &Path) -> anyhow::Result<()> {
	for path in paths {
		assert!(
			path.starts_with('/'),
			"whitelisted path must be absolute: {path}"
		);
		match fs::metadata(path) {
			Ok(m) if m.is_dir() => {
				let mut pb = PathBuf::from(jail_dir);
				let without_leading_slash = &path[1..];
				pb.push(without_leading_slash);
				fs::create_dir_all(&pb).with_context(|| {
					format!(
						"Failed to create dir for whitelisted path {}",
						pb.display()
					)
				})?;
				let orig_perms = m.permissions();
				fs::set_permissions(&pb, orig_perms).with_context(|| {
					format!(
						"Failed to set permissions for whitelisted path {}",
						pb.display()
					)
				})?;
				mount(Some(*path), &pb, M_NONE, MsFlags::MS_BIND, M_NONE)
					.with_context(|| {
						format!(
							"Failed to bind whitelisted dir into jail root. dir: {path}"
						)
					})?;
			}
			Ok(m) if m.is_file() => {
				let mut pb = PathBuf::from(jail_dir);
				// PathBuf::push cannot concat if the path is absolute (starts with `/`)
				// so we need to strip it
				let without_leading_slash = &path[1..];
				pb.push(without_leading_slash);
				let parent = pb.parent().unwrap();
				fs::create_dir_all(parent).with_context(|| {
					format!(
						"Failed to create dir for whitelisted path {}",
						parent.display()
					)
				})?;
				let orig_perms = m.permissions().mode() & PERM_MASK;
				// We can't use std::fs::File::create because it does a permission check
				// (outside of the syscall), which fails because uid = nobody
				// even though we can create the file
				close(
					open(
						&pb,
						OFlag::O_CREAT,
						Mode::from_bits_truncate(orig_perms),
					)
					.with_context(|| {
						format!(
							"Failed to create file for whitelisted path {}",
							pb.display()
						)
					})?,
				)
				.with_context(|| {
					format!(
						"Failed to close file for whitelisted path {}",
						pb.display()
					)
				})?;
				mount(Some(*path), &pb, M_NONE, MsFlags::MS_BIND, M_NONE)
					.with_context(|| -> String {
						format!(
							"Failed to bind whitelisted file into jail root. file: {path}"
						)
					})?;
			}
			Ok(_) => {
				eprintln!(
					"Warning: whitelisted path {path} is not a file or dir, skipping"
				);
			}
			Err(_) => todo!(),
		}
	}
	Ok(())
}

/// Confine the current process's filesystem view before the seccomp filter is
/// installed.
///
/// Prefers a mount-namespace jail (`unshare` + `pivot_root`) which hides
/// everything except the whitelisted paths. If the namespace unshare is denied
/// with `EPERM` (e.g. Docker's default seccomp profile blocks
/// `unshare(CLONE_NEWUSER)`), falls back to a Landlock ruleset that restricts
/// filesystem access to the same whitelist.
#[cfg(target_os = "linux")]
fn enter_sandbox(allow_paths: &[&str]) -> anyhow::Result<()> {
	// Detach ourselves from the parent mount namespace
	// we have to pass CLONE_NEWUSER because CLONE_NEWNS requires CAP_SYS_ADMIN, which we might not have
	// CLONE_FILES to close all fds

	match unshare(
		CloneFlags::CLONE_NEWNS
			| CloneFlags::CLONE_NEWUSER
			| CloneFlags::CLONE_FILES,
	) {
		Ok(()) => {
			// The user namespace was created, but on some hosts the namespace
			// has its capabilities stripped (e.g. GitHub Actions runners with
			// apparmor_restrict_unprivileged_userns=1), so the mount/pivot_root
			// jail setup fails with EACCES even though unshare(2) succeeded.
			// Fall back to the Landlock fs sandbox in that case, matching the
			// behaviour when unshare(2) itself is denied.
			let jail = configure_userns_mapping()
				.and_then(|()| enter_sandbox_namespaces(allow_paths));
			match jail {
				Ok(()) => Ok(()),
				Err(e) => {
					warn!(
						"user-namespace jail setup failed ({e:#}); falling back to Landlock fs sandbox"
					);
					enter_sandbox_landlock(allow_paths)
				}
			}
		}
		// Docker blocks unshare(2) vis seccomp, so fall back to landlock
		Err(Errno::EPERM) => {
			warn!(
				"unshare(CLONE_NEWNS | CLONE_NEWUSER | CLONE_FILES) denied (EPERM); falling back to Landlock fs sandbox"
			);
			enter_sandbox_landlock(allow_paths)
		}
		Err(e) => {
			Err(e).context("unshare(CLONE_NEWNS | CLONE_NEWUSER | CLONE_FILES)")
		}
	}
}

/// A freshly `unshare`d user namespace has no uid/gid mapping, so the process
/// runs as the overflow (nobody) id: `mount(2)` and path resolution then fail
/// with `EACCES`. Map the current uid/gid to root inside the namespace so the
/// bind mounts and `pivot_root` below are permitted. This is what
/// bubblewrap/crun do and is required whenever the job runs as an unprivileged
/// user (e.g. GitHub Actions runners run as uid 1001, not root).
#[cfg(target_os = "linux")]
fn configure_userns_mapping() -> anyhow::Result<()> {
	// SAFETY: `geteuid`/`getegid` are always-successful syscalls with no
	// preconditions and no memory safety implications.
	let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
	// `setgroups` must be denied before writing `gid_map` in an unprivileged
	// user namespace (kernel requirement since Linux 3.19).
	fs::write("/proc/self/setgroups", "deny")
		.context("write /proc/self/setgroups")?;
	fs::write("/proc/self/uid_map", format!("0 {uid} 1"))
		.context("write /proc/self/uid_map")?;
	fs::write("/proc/self/gid_map", format!("0 {gid} 1"))
		.context("write /proc/self/gid_map")?;
	Ok(())
}

/// sandbox via [`pivot_root`] + bind-mounts
#[cfg(target_os = "linux")]
fn enter_sandbox_namespaces(allow_paths: &[&str]) -> anyhow::Result<()> {
	// Mounts inherited into the new namespace are shared+locked. Recursively
	// make the whole tree private so our bind mounts don't propagate to the
	// host and so pivot_root's "new root and its parent must not be shared"
	// requirement holds. Without this, the bind mount / pivot_root below
	// intermittently fails with EACCES on hosts where / is mounted rshared
	// (e.g. GitHub Actions runners).
	mount(
		M_NONE,
		"/",
		M_NONE,
		MsFlags::MS_REC | MsFlags::MS_PRIVATE,
		M_NONE,
	)
	.context("make mount tree private")?;

	// use PID to create a unique dir and make it easy to find
	let mut jail_dir = env::temp_dir();
	let pid = process::id();
	jail_dir.push(format!("qalc-jail.{pid}"));

	fs::create_dir_all(&jail_dir)?;
	// bind-mount the jail dir so we can pivot_root into it
	mount(Some(&jail_dir), &jail_dir, M_NONE, MsFlags::MS_BIND, M_NONE)
		.context("mount bind jail dir")?;
	// pivot_root requires a dir to mount the old root
	let old_root_dir = jail_dir.join("old_root");
	fs::create_dir_all(&old_root_dir)?;
	env::set_current_dir(&jail_dir).context("Failed to chdir to jail root")?;

	// pivot_root requires that no mounts are shared so we bind-mount whitelisted files in
	bind_allowed_paths(allow_paths, &jail_dir)
		.context("Failed to bind whitelisted paths")?;

	// pivot_root and enter the jail
	pivot_root(c".", &old_root_dir).with_context(|| {
		format!("Failed to pivot root to {}", jail_dir.display())
	})?;

	env::set_current_dir("/").context("Failed to chdir to new root")?;
	// unmount and detach the old root so it can't be accessed anymore
	umount2(c"/old_root", MntFlags::MNT_DETACH)
		.context("Failed to umount old root")?;
	fs::remove_dir("/old_root").context("Failed to remove old root dir")?;
	Ok(())
}

/// Filesystem confinement via Landlock, used when the namespace unshare is
/// denied. Grants read-only access to the whitelisted paths and denies all
/// other filesystem access. Unlike the mount-namespace jail this does not
/// change the filesystem view, so unlisted paths remain visible but are
/// inaccessible.
///
/// Requires Landlock support in the running kernel (Linux 5.13+). Uses
/// best-effort ABI compatibility, but bails if the ruleset ends up entirely
/// unenforced rather than running the worker without a filesystem sandbox.
#[cfg(target_os = "linux")]
fn enter_sandbox_landlock(allow_paths: &[&str]) -> anyhow::Result<()> {
	let abi = ABI::V5;
	let status = Ruleset::default()
		.handle_access(AccessFs::from_all(abi))
		.context("landlock: handle_access")?
		.create()
		.context("landlock: create ruleset")?
		.add_rules(path_beneath_rules(allow_paths, AccessFs::from_read(abi)))
		.context("landlock: add read rules for whitelisted paths")?
		.restrict_self()
		.context("landlock: restrict_self")?;

	if status.ruleset == RulesetStatus::NotEnforced {
		anyhow::bail!(
			"Landlock is unsupported by the running kernel; refusing to run the worker without a filesystem sandbox"
		);
	}
	Ok(())
}

/// Entry point for a standalone worker executable spawned via
/// [`Sandbox::try_new_exec`]. Runs the sandbox worker loop and never returns.
///
/// `ipc_name` is the bootstrap server name the parent passes as the child's
/// first CLI argument.
pub fn worker_main(ipc_name: String) -> ! {
	child_proc(ipc_name)
}

/// Like [`worker_main`], but reads the bootstrap name from this process's first
/// CLI argument (the form [`Sandbox::try_new_exec`] spawns the worker with).
pub fn worker_main_from_args() -> ! {
	let Some(ipc_name) = env::args().nth(1) else {
		eprintln!("qalc_sbox worker: missing IPC bootstrap name argument");
		process::exit(2);
	};
	worker_main(ipc_name)
}

fn child_proc(ipc_name: String) -> ! {
	// We have to use expect here as there's no good form of error handling available here
	let (p_tx, rx) = ipc::channel().expect("Failed to create IPC channel");
	let (tx, p_rx) = ipc::channel().expect("Failed to create IPC channel");
	let init_packet: InitPacket = (p_tx, p_rx);
	let init_srv =
		IpcSender::connect(ipc_name).expect("Failed to connect to IPC server");
	init_srv
		.send(init_packet)
		.expect("Failed to send init packet");
	let mut calc = Qalculator::create();
	macro_rules! fatal_error {
		($($arg:tt)*) => {
			{
				let msg = format!($($arg)*);
				if let Err(e) = tx.send(MsgToParent::FatalError { msg: msg.clone() }) {
					eprintln!("Failed to send fatal error to parent: {e}");
				}
				eprintln!("sandbox child fatal error: {msg}");
				// We can't panic here or call exit because those both use non-whitelisted syscalls
				// SAFETY: exit_group terminates us, nothing happens after this
				// See exit_group(2) for details
				unsafe {
					libc::syscall(libc::SYS_exit_group, 1);
					std::hint::unreachable_unchecked()
				}
			}
		}
	}
	if calc.is_null() {
		fatal_error!("Failed to create Qalculator instance");
	}
	let mut calc = calc.as_mut().unwrap();
	calc.as_mut()
		.allow_impure_expressions(false);
	calc.as_mut().enable_sandboxing();
	if !calc.as_mut().load_exchange_rates() {
		fatal_error!("Failed to load exchange rates");
	}
	if !calc.as_mut().load_global_defs() {
		fatal_error!("Failed to load global defs");
	}
	if !calc.as_mut().load_local_defs() {
		fatal_error!("Failed to load local defs");
	}
	#[cfg(target_os = "linux")]
	{
		let qalc_data_dir = Qalculator::get_package_data_dir();
		eprintln!("Resolved qalc data dir: {qalc_data_dir}");
		let whitelisted_paths = ["/etc/localtime", &qalc_data_dir];
		if let Err(e) = enter_sandbox(&whitelisted_paths) {
			fatal_error!("Failed to enter sandbox: {e:?}");
		}
		// preload io message file before filter is installed
		let _ = io::Error::from_raw_os_error(libc::EPIPE).to_string();
		if let Err(e) = install_seccomp_filter() {
			fatal_error!("Failed to install seccomp filter: {e}");
		}
	}
	#[cfg(not(target_os = "linux"))]
	compile_error!("TODO: implement sandboxing on non-linux platforms");
	loop {
		let msg = match rx.recv() {
			Ok(m) => m,
			Err(e) => {
				fatal_error!("Failed to receive message from parent: {e}");
			}
		};

		match msg {
			MsgToChild::Eval { content, id } => {
				let result = calc
					.as_mut()
					.calculate_and_print(&content);
				let res_msg = match result {
					Ok(s) => {
						if cfg!(test) {
							eprintln!("Eval result: {s:?}");
						}
						MsgToParent::EvalResult { id, result: Ok(s) }
					}
					Err(e) => {
						let msg = format!("Failed to evaluate expression: {e}");
						MsgToParent::EvalResult {
							id,
							result: Err(msg),
						}
					}
				};
				if let Err(e) = tx.send(res_msg) {
					fatal_error!("Failed to send result to parent: {e}");
				}
			}
			MsgToChild::Shutdown => {
				// parent asked us to exit (all Sandbox handles dropped)
				// SAFETY: exit_group terminates us, nothing happens after this
				// See exit_group(2) for details
				unsafe {
					libc::syscall(libc::SYS_exit_group, 0);
					std::hint::unreachable_unchecked()
				}
			}
		}
	}
}

/// Reap the forked child so it doesn't linger as a zombie.
///
/// Retries on `EINTR`; any other error just means there's nothing to reap
/// (already reaped, or never a child of ours), which is fine.
fn reap_child(pid: libc::pid_t) {
	loop {
		let mut status = 0;
		// SAFETY: waitpid only writes through the provided status pointer
		let ret = unsafe { libc::waitpid(pid, &raw mut status, 0) };
		if ret == -1
			&& io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
		{
			continue;
		}
		break;
	}
}

#[cfg(target_os = "linux")]
fn install_seccomp_filter() -> Result<(), String> {
	// don't allow the child to do some tricks to gain new privs
	// SEE man 2const PR_SET_NO_NEW_PRIVS
	// SAFETY: doesn't deal with memory
	if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
		return Err(format!(
			"prctl(PR_SET_NO_NEW_PRIVS): {}",
			io::Error::last_os_error()
		));
	}

	// needed syscalls for qalc + ipc
	#[rustfmt::skip]
	const ALLOWED: &[libc::c_long] = &[
		// fd ops
		libc::SYS_read, libc::SYS_write, libc::SYS_close, libc::SYS_fcntl, libc::SYS_ppoll,
		// socket ops
		libc::SYS_recvmsg, libc::SYS_sendmsg,  
		// memory allocation
		libc::SYS_mmap, libc::SYS_munmap, libc::SYS_mremap,
		libc::SYS_mprotect, libc::SYS_brk, libc::SYS_madvise,
		// threading
		libc::SYS_clone, libc::SYS_clone3, libc::SYS_futex,
		libc::SYS_set_robust_list, libc::SYS_get_robust_list, libc::SYS_rseq,
		libc::SYS_set_tid_address, libc::SYS_gettid, libc::SYS_sched_yield,
		libc::SYS_sched_getaffinity, libc::SYS_sigaltstack,
		libc::SYS_rt_sigprocmask, libc::SYS_rt_sigaction,
		libc::SYS_rt_sigreturn, libc::SYS_tgkill, libc::SYS_membarrier,
		// clock/timer
		libc::SYS_clock_gettime, libc::SYS_clock_nanosleep,
		libc::SYS_nanosleep, libc::SYS_timer_create, libc::SYS_timer_settime,
		libc::SYS_timer_delete, libc::SYS_timer_gettime,
		libc::SYS_gettimeofday,
		// misc
		libc::SYS_getrandom, libc::SYS_getpid, libc::SYS_restart_syscall,
		libc::SYS_exit, libc::SYS_exit_group,
		// used to read /etc/localtime and files from qalc_data_dir
		libc::SYS_openat, libc::SYS_fstat, libc::SYS_lseek,
		// used for atom()
		libc::SYS_getcwd, libc::SYS_newfstatat, libc::SYS_dup
	];

	let rules: BTreeMap<i64, Vec<SeccompRule>> = ALLOWED
		.iter()
		.map(|&n| (n, Vec::new() /* no extra conditions */))
		.collect();

	// Kill in debug builds to make it easire to find missing syscalls
	let mismatch_action = if cfg!(debug_assertions) || cfg!(test) {
		SeccompAction::KillProcess
	} else {
		SeccompAction::Errno(libc::EPERM as u32)
	};

	let filter = SeccompFilter::new(
		rules,
		mismatch_action,      // not on the allowlist
		SeccompAction::Allow, // on the allowlist
		std::env::consts::ARCH
			.try_into()
			.map_err(|e| format!("unsupported seccomp arch: {e}"))?,
	)
	.map_err(|e| format!("build seccomp filter: {e}"))?;

	let prog: BpfProgram = filter
		.try_into()
		.map_err(|e| format!("compile seccomp filter: {e}"))?;

	seccompiler::apply_filter(&prog)
		.map_err(|e| format!("apply seccomp filter: {e}"))
}

fn mk_id() -> u64 {
	static NEXT_ID: AtomicU64 = AtomicU64::new(1);
	NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn parent_proc(
	(ctx, crx): InitPacket,
	irx: &mpsc::Receiver<SandboxMessage>,
	child_pid: libc::pid_t,
) {
	loop {
		let msg = match irx.recv() {
			Ok(m) => m,
			Err(e) => {
				warn!(
					"Sandbox internal channel closed, shutting down child process, error: {e}"
				);
				// tell the child to exit cleanly, then reap it. Ignore send errors:
				// the child may already be gone, in which case we still reap below.
				let _ = ctx.send(MsgToChild::Shutdown);
				reap_child(child_pid);
				return;
			}
		};
		match msg {
			SandboxMessage::Eval { content, tx } => {
				let id = mk_id();
				if let Err(e) = ctx.send(MsgToChild::Eval { id, content }) {
					warn!("Failed to send eval message to child: {e}");
					let _ = tx.send(Err(format!(
						"Failed to send message to sandbox: {e}"
					)));
					let _ = ctx.send(MsgToChild::Shutdown);
					reap_child(child_pid);
					return;
				}
				match crx.recv() {
					Ok(MsgToParent::EvalResult { id: rid, result }) => {
						if rid != id {
							warn!(
								"Eval result id mismatch: expected {id}, got {rid}"
							);
						}
						let _ = tx.send(result);
					}
					Ok(MsgToParent::FatalError { msg }) => {
						warn!("Child process reported fatal error: {msg}");
						let _ =
							tx.send(Err(format!("Sandbox fatal error: {msg}")));
						// child is exiting on its own after a fatal error; reap it
						reap_child(child_pid);
						return;
					}
					Err(e) => {
						warn!("Failed to receive eval result from child: {e}");
						let _ = tx.send(Err(format!(
							"Failed to receive result from sandbox: {e}"
						)));
						let _ = ctx.send(MsgToChild::Shutdown);
						reap_child(child_pid);
						return;
					}
				}
			}
		}
	}
}

impl Sandbox {
	pub const INTERNAL_CHANNEL_SIZE: usize = 1024;

	/// Spawn the worker by `fork()`ing the current process.
	///
	/// The child shares the parent's address space copy-on-write, so a large
	/// parent produces a child whose RSS is inflated by inherited (unused)
	/// pages. When that matters, use [`Sandbox::try_new_exec`] to launch a
	/// small standalone worker binary instead.
	pub fn try_new() -> Result<Self, Error> {
		#[cfg(target_os = "windows")]
		compile_error!("TODO: implement sandboxing on windows");
		let (init_srv, init_srv_name) = IpcOneShotServer::<InitPacket>::new()
			.map_err(Error::CreateInitIpcServer)?;
		// SAFETY: we are single-threaded in the child process so fork() is safe
		let ec = unsafe { libc::fork() };
		if ec == -1 {
			let err = io::Error::last_os_error();
			return Err(Error::Fork(err));
		}
		if ec == 0 {
			child_proc(init_srv_name);
		}
		assert!(ec > 0, "unreachable");
		Self::finish_setup(init_srv, ec)
	}

	/// Spawn the worker as a fresh child *process* running `exe`, rather than
	/// forking the current process. This avoids inheriting the parent's heap,
	/// keeping the worker's memory footprint minimal.
	///
	/// `exe` must be a program that, on startup, calls [`worker_main`] (or
	/// [`worker_main_from_args`]) with the IPC bootstrap name this passes as
	/// the child's first CLI argument.
	pub fn try_new_exec(exe: impl AsRef<OsStr>) -> Result<Self, Error> {
		let (init_srv, init_srv_name) = IpcOneShotServer::<InitPacket>::new()
			.map_err(Error::CreateInitIpcServer)?;
		let child = process::Command::new(exe)
			.arg(&init_srv_name)
			.spawn()
			.map_err(Error::Spawn)?;
		let child_pid = child.id() as libc::pid_t;
		// we manage the child's lifecycle (reaping) ourselves via `parent_proc`,
		// so don't let `Child`'s drop glue do anything with the pid.
		std::mem::forget(child);
		Self::finish_setup(init_srv, child_pid)
	}

	/// Accept the worker's bootstrap connection and start the parent-side
	/// message pump. Shared by [`Sandbox::try_new`] and
	/// [`Sandbox::try_new_exec`].
	fn finish_setup(
		init_srv: IpcOneShotServer<InitPacket>,
		child_pid: libc::pid_t,
	) -> Result<Self, Error> {
		let (_, (tx, rx)) = init_srv
			.accept()
			.map_err(Error::AcceptInitIpcServer)?;
		let (itx, irx) = mpsc::sync_channel(Self::INTERNAL_CHANNEL_SIZE);
		thread::spawn(move || {
			parent_proc((tx, rx), &irx, child_pid);
		});
		let inner = Arc::new(Inner { tx: itx, child_pid });
		Ok(Self(inner))
	}

	/// Resident set size (physical memory) of the sandboxed worker, in bytes.
	///
	/// Reads `/proc/<child_pid>/statm`. Returns an error if the worker has
	/// exited (its `/proc` entry is gone) or the file can't be parsed.
	#[cfg(target_os = "linux")]
	pub fn memory_usage(&self) -> io::Result<u64> {
		let pid = self.0.child_pid;
		let statm = fs::read_to_string(format!("/proc/{pid}/statm"))?;
		// statm fields are in pages: size resident shared text lib data dt
		let resident_pages: u64 = statm
			.split_ascii_whitespace()
			.nth(1)
			.and_then(|s| s.parse().ok())
			.ok_or_else(|| {
				io::Error::new(
					io::ErrorKind::InvalidData,
					format!("unexpected /proc/{pid}/statm contents: {statm:?}"),
				)
			})?;
		// SAFETY: sysconf just reads a system constant
		let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
		let page_size = u64::try_from(page_size).map_err(|_| {
			io::Error::other("sysconf(_SC_PAGESIZE) returned an invalid value")
		})?;
		Ok(resident_pages * page_size)
	}

	/// Evaluate an expression in the sandboxed child process.
	///
	/// returns Ok(Err(msg)) if qalculate returned an error
	pub async fn eval(
		&self,
		content: String,
	) -> Result<Result<String, String>, Error> {
		let this = self.clone();
		tokio::task::spawn_blocking(move || this.eval_blocking(content)).await?
	}

	pub fn eval_blocking(
		&self,
		content: String,
	) -> Result<Result<String, String>, Error> {
		let (tx, rx) = oneshot::channel();
		self.0
			.tx
			.send(SandboxMessage::Eval { content, tx })
			.map_err(Error::Send)?;
		rx.recv().map_err(Error::OneshotRecv)
	}
}

#[cfg(all(test, target_os = "linux"))]
mod landlock_tests {
	use std::{env, fs, io::ErrorKind, process};

	use super::enter_sandbox_landlock;

	/// Whether the running kernel exposes Landlock, gleaned from securityfs.
	/// If securityfs is not mounted we cannot tell, so the caller skips.
	fn landlock_available() -> Option<bool> {
		let lsm = fs::read_to_string("/sys/kernel/security/lsm").ok()?;
		Some(lsm.split(',').any(|m| m == "landlock"))
	}

	// Child exit codes for the forked probe.
	const OK: i32 = 0;
	const APPLY_FAILED: i32 = 12;
	const ALLOWED_READ_FAILED: i32 = 10;
	const DENIED_READ_LEAKED: i32 = 11;
	const DENIED_READ_WRONG_ERR: i32 = 13;

	fn child_probe(
		allow_dir: &str,
		allowed_file: &str,
		denied_file: &str,
	) -> ! {
		let code = if enter_sandbox_landlock(&[allow_dir]).is_err() {
			APPLY_FAILED
		} else if fs::read(allowed_file).is_err() {
			ALLOWED_READ_FAILED
		} else {
			match fs::read(denied_file) {
				Ok(_) => DENIED_READ_LEAKED,
				Err(e) if e.kind() == ErrorKind::PermissionDenied => OK,
				Err(_) => DENIED_READ_WRONG_ERR,
			}
		};
		// SAFETY: _exit terminates the child immediately without running
		// atexit handlers, which is required after fork(2).
		unsafe { libc::_exit(code) }
	}

	#[test]
	fn landlock_blocks_non_whitelisted_paths() {
		match landlock_available() {
			Some(true) => {}
			Some(false) => {
				eprintln!("skipping: kernel has no Landlock support");
				return;
			}
			None => {
				eprintln!("skipping: cannot determine Landlock support");
				return;
			}
		}

		let base =
			env::temp_dir().join(format!("landlock-test.{}", process::id()));
		let allow_dir = base.join("allowed");
		let deny_dir = base.join("denied");
		fs::create_dir_all(&allow_dir).unwrap();
		fs::create_dir_all(&deny_dir).unwrap();
		let allowed_file = allow_dir.join("ok.txt");
		let denied_file = deny_dir.join("secret.txt");
		fs::write(&allowed_file, b"ok").unwrap();
		fs::write(&denied_file, b"secret").unwrap();

		let allow_s = allow_dir.to_str().unwrap().to_owned();
		let allowed_s = allowed_file
			.to_str()
			.unwrap()
			.to_owned();
		let denied_s = denied_file.to_str().unwrap().to_owned();

		// Landlock restriction is per-process and irreversible, so run the
		// probe in a forked child to avoid confining the test runner.
		// SAFETY: fork(2); the child only performs Landlock syscalls,
		// read(2), and _exit(2). At test entry the process is effectively
		// single-threaded.
		let pid = unsafe { libc::fork() };
		assert!(pid >= 0, "fork failed");
		if pid == 0 {
			child_probe(&allow_s, &allowed_s, &denied_s);
		}

		let mut status: libc::c_int = 0;
		// SAFETY: valid pid from fork, valid status pointer.
		let w = unsafe { libc::waitpid(pid, &raw mut status, 0) };
		assert_eq!(w, pid, "waitpid failed");

		let _ = fs::remove_dir_all(&base);

		assert!(
			libc::WIFEXITED(status),
			"child killed by signal, raw status {status}"
		);
		let code = libc::WEXITSTATUS(status);
		assert_eq!(
			code, OK,
			"child probe failed with code {code} (10=allowed read blocked, \
			 11=denied read LEAKED, 12=landlock apply failed, 13=denied read \
			 failed with wrong error)"
		);
	}
}
