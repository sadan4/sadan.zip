use companion_lsp::{
	ReloadHandle,
	lsp::{self, custom::ephemera::EphemeralQuery},
};
use miette_ui::install_miette_hook;
use std::{env, io::Write as _, process, thread, time::Duration};
use tokio::io;
use tower_lsp_server::{LspService, Server, ls_types::request::Request};

fn main() {
	#[cfg(debug_assertions)]
	{
		// SAFETY: this just allows us to be traced by a debugger, without needing to set yama.ptrace_scope = 0
		// SEE: PR_SET_PTRACER.2const
		#[cfg(target_os = "linux")]
		unsafe {
			libc::prctl(libc::PR_SET_PTRACER, libc::PR_SET_PTRACER_ANY);
		};
		wait_for_debugger(());
	};
	setup_backtrace();
	tokio_main();
}

#[tokio::main]
async fn tokio_main() {
	install_miette_hook(false);
	let reload_handle = init_tracing();
	let builder = lsp::ServerBuilder::new()
		.expect("Failed to create LSP ServerBuilder")
		.with_reload_handle(reload_handle);
	let (service, socket) = LspService::build(|client| builder.build(client))
		.custom_method(
			EphemeralQuery::METHOD,
			async |client: &lsp::Server, params| {
				client.query_ephemeral_document(&params)
			},
		)
		.finish();
	Server::new(io::stdin(), io::stdout(), socket)
		.serve(service)
		.await;
}
/// Spin until a debugger attaches, when `COMPANION_LSP_WAIT_DEBUGGER` is set and debug assertions are enabled.
///
/// Generic so the assertions is only ever run when it's called
#[cfg(target_os = "linux")]
#[cfg_attr(not(debug_assertions), allow(unused))]
fn wait_for_debugger<T>(_: T) {
	const {
		assert!(
			cfg!(debug_assertions),
			"wait_for_debugger should only be called in debug builds"
		);
	};
	if env::var_os("COMPANION_LSP_WAIT_DEBUGGER").is_none() {
		return;
	}
	// stdout is the LSP transport, so this has to go to stderr.
	let mut stderr = std::io::stderr();
	let _ = writeln!(
		stderr,
		"companion_lsp: pid {} waiting for a debugger",
		process::id()
	);
	while !traced() {
		thread::sleep(Duration::from_millis(100));
	}
	let _ = writeln!(stderr, "companion_lsp: debugger attached, continuing");

	/// Whether a debugger currently has us under `ptrace`.
	///
	/// Always `false` where `/proc/self/status` is unreadable.
	fn traced() -> bool {
		std::fs::read_to_string("/proc/self/status")
			.ok()
			.and_then(|status| {
				status
					.lines()
					.find_map(|line| line.strip_prefix("TracerPid:"))
					.map(|pid| pid.trim() != "0")
			})
			.unwrap_or(false)
	}
}

#[cfg(all(not(target_os = "linux"), debug_assertions))]
fn wait_for_debugger() {
	compile_error!("TODO: implement wait_for_debugger for non-Linux platforms");
}

fn setup_backtrace() {
	use std::env;
	if env::var_os("RUST_BACKTRACE").is_none() {
		// SAFETY: no other threads are running yet
		unsafe {
			env::set_var("RUST_BACKTRACE", "1");
		}
	}
}

fn init_tracing() -> ReloadHandle {
	use tracing_subscriber::{
		EnvFilter,
		fmt,
		layer::SubscriberExt as _,
		registry,
		reload,
		util::SubscriberInitExt as _,
	};

	let filter =
		EnvFilter::try_from_env("COMPANION_LSP_LOG").unwrap_or_else(|_| {
			if cfg!(debug_assertions) {
				EnvFilter::new("debug")
			} else {
				EnvFilter::new("info")
			}
		});

	let (reload_layer, reload_handle) = reload::Layer::new(filter);

	let fmt_layer = fmt::layer()
		.with_writer(std::io::stderr)
		.with_ansi_sanitization(false)
		.with_ansi(false);

	registry()
		.with(reload_layer)
		.with(fmt_layer)
		.init();
	reload_handle
}
