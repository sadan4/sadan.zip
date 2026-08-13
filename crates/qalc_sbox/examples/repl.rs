use std::env;

use qalc_sbox::Sandbox;

#[tokio::main]
/// To find the syscall that is missing 
/// run `cargo build -p qalc_sbox --example repl && strace -ff -o trace target/debug/examples/repl "<expression>"`
/// then use `coredumpctl info` to show the PID of the killed process
/// the syscall will be the last one in `trace.<PID>`
async fn main() {
	let sbox = Sandbox::try_new().expect("new sandbox");
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
