use std::{fs, path::Path};

use anyhow::Context as _;
use pyo3_stub_gen::Result;
use qalc_sbox_py::stub_info;

fn main() -> Result<()> {
	let stub = stub_info()?;
	stub.generate()?;

	// `pyo3-stub-gen` only sees the inventory-collected pyclasses; the
	// `tracing_subscriber` submodule is registered at runtime via
	// `add_submodule`, so its `.pyi` must be emitted separately. Write it into
	// the mixed-layout package tree next to the other stubs.
	let pkg = Path::new(env!("CARGO_MANIFEST_DIR")).join("python/qalc_sbox_py");
	pyo3_tracing_subscriber_build::write_stub_files(
		"qalc_sbox_py",
		"tracing_subscriber",
		&pkg.join("tracing_subscriber"),
	)?;

	// `stub_info` doesn't know about the runtime-registered submodule, so the
	// top-level stub omits it. Re-export it so type checkers see
	// `qalc_sbox_py.tracing_subscriber` as an attribute, matching runtime.
	reexport_tracing_subscriber(&pkg.join("__init__.pyi"))?;
	Ok(())
}

/// Append a `tracing_subscriber` re-export to the generated top-level stub.
/// Idempotent: does nothing if the re-export is already present.
fn reexport_tracing_subscriber(init_pyi: &Path) -> Result<()> {
	let mut contents = fs::read_to_string(init_pyi)
		.with_context(|| format!("reading {} to patch", init_pyi.display()))?;
	if contents.contains("tracing_subscriber") {
		return Ok(());
	}
	if !contents.ends_with('\n') {
		contents.push('\n');
	}
	contents.push_str(
		r#"# hack for typing for tracing_subscriber
from . import tracing_subscriber as tracing_subscriber
__all__ += ["tracing_subscriber"]"#,
	);
	fs::write(init_pyi, contents)
		.with_context(|| format!("writing patched {}", init_pyi.display()))?;
	Ok(())
}
