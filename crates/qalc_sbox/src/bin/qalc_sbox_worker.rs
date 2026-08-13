//! Standalone qalc sandbox worker.
//!
//! Point [`qalc_sbox::Sandbox::try_new_exec`] at this binary to spawn the
//! worker as a fresh process (no fork of the parent's heap). The parent passes
//! the IPC bootstrap name as the first CLI argument, which
//! `worker_main_from_args` reads.
fn main() -> ! {
	qalc_sbox::worker_main_from_args()
}
