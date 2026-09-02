use std::env;

use qalc_sbox::Sandbox;

/// `qalc_sbox_worker` sits next to this example binary's `target/<profile>/`
/// dir (examples build into `target/<profile>/examples/`), the same layout
/// `cargo` produces for every profile. `env!("CARGO_BIN_EXE_...")` isn't
/// available here — Cargo only injects those for integration tests/benches,
/// not examples — so resolve it relative to our own path instead.
fn worker_exe_path() -> std::path::PathBuf {
	let exe = env::current_exe().expect("resolve current_exe");
	exe.parent()
		.and_then(std::path::Path::parent)
		.expect("examples/ has a target/<profile>/ parent")
		.join("qalc_sbox_worker")
}

#[tokio::main]
/// To find the syscall that is missing, build both this example and the
/// worker bin, then run:
/// `cargo build -p qalc_sbox --example repl --bin qalc_sbox_worker && strace -ff -o trace target/debug/examples/repl "<expression>"`
/// then use `coredumpctl info` to show the PID of the killed process
/// the syscall will be the last one in `trace.<PID>`
async fn main() {
	let sbox = Sandbox::try_new_exec(worker_exe_path()).expect("new sandbox");
	let Some(arg) = env::args().nth(1) else {
		eprintln!(
			"Usage: {} <expression>",
			env::args().next().unwrap_or_default()
		);
		// SAFETY: guh
		unsafe { libc::_exit(1) };
	};

	eprintln!("Evaluating: {arg:?}");
	let out = sbox
		.eval(arg)
		.await
		.expect("post/recv msg");
	println!("Eval result: {out:?}");
}
