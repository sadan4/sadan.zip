//! Test the sandbox with seccomp installed
use qalc_sbox::Sandbox;
use std::assert_matches;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eval_works_under_seccomp() {
	let sbox = Sandbox::try_new().expect("spawn sandbox child");

	let out = sbox
		.eval("2+2".to_string())
		.await
		.expect("round-trip to sandbox");
	assert_eq!(out, Ok("4".to_string()), "basic arithmetic under filter");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eval_works_with_exec_worker() {
	// spawn the worker as a separate executable instead of forking
	let sbox = Sandbox::try_new_exec(env!("CARGO_BIN_EXE_qalc_sbox_worker"))
		.expect("spawn standalone worker");

	let out = sbox
		.eval("2+2".to_string())
		.await
		.expect("round-trip to sandbox");
	assert_eq!(out, Ok("4".to_string()), "basic arithmetic via exec worker");
}

mod blacklisted_functions {

	use super::*;

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn load_errors_under_seccomp() {
		let sbox = Sandbox::try_new().expect("spawn sandbox child");

		let out = sbox
			.eval("load(\"file\")".to_string())
			.await
			.expect("round-trip to sandbox");
		assert_eq!(
			out.as_ref().map_err(String::as_str),
			Err("Failed to evaluate expression: function load is disabled"),
		);
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn export_errors_under_seccomp() {
		let sbox = Sandbox::try_new().expect("spawn sandbox child");

		let out = sbox
			.eval("export([1 2], \"file\")".to_string())
			.await
			.expect("round-trip to sandbox");
		assert_eq!(
			out.as_ref().map_err(String::as_str),
			Err("Failed to evaluate expression: function export is disabled"),
		);
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn command_errors_under_seccomp() {
		let sbox = Sandbox::try_new().expect("spawn sandbox child");

		let out = sbox
			.eval("command(\"ls\")".to_string())
			.await
			.expect("round-trip to sandbox");
		assert_eq!(
			out.as_ref().map_err(String::as_str),
			Err("Failed to evaluate expression: function command is disabled"),
		);
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn plot_errors_under_seccomp() {
		let sbox = Sandbox::try_new().expect("spawn sandbox child");

		let out = sbox
			.eval("plot(sin(x))".to_string())
			.await
			.expect("round-trip to sandbox");

		assert_eq!(
			out.as_ref().map_err(String::as_str),
			Err("Failed to evaluate expression: function plot is disabled"),
		);
	}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn atom_works_under_seccomp() {
	let sbox = Sandbox::try_new().expect("spawn sandbox child");

	let out = sbox
		.eval("atom(Hg; weight) + atom(C; weight) * 4 to g".to_string())
		.await
		.expect("round-trip to sandbox");

	assert_eq!(out.unwrap(), "4.129e-22 g",);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn time_works_under_seccomp() {
	let sbox = Sandbox::try_new().expect("spawn sandbox child");

	let out = sbox
		.eval("time()".to_string())
		.await
		.expect("round-trip to sandbox");

	// we can't test the exact result because it's based on the time
	assert_matches!(out, Ok(_));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_usage_reports_live_worker() {
	let sbox = Sandbox::try_new().expect("spawn sandbox child");

	// evaluate something so the worker has actually done work
	sbox.eval("2+2".to_string())
		.await
		.expect("round-trip to sandbox")
		.expect("eval ok");

	let rss = sbox
		.memory_usage()
		.expect("read worker memory usage");

	assert!(rss > 0, "worker RSS should be non-zero, got {rss}");
	assert!(rss < 1024 * 1024 * 1024, "worker RSS is too large: {rss}");
}
