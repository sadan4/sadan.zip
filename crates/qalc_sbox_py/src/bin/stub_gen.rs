use pyo3_stub_gen::Result;
use qalc_sbox_py::stub_info;

fn main() -> Result<()> {
	let stub = stub_info()?;
	stub.generate()?;
	Ok(())
}
