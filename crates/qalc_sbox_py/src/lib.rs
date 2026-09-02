use pyo3::pymodule;
use pyo3_stub_gen::define_stub_info_gatherer;

#[pymodule]
mod qalc_sbox_py {
	use pyo3::{Bound, PyResult, pymodule, types::PyModule};

	/// Register the `qalc_sbox_py.tracing_subscriber` submodule so Python can
	/// drive the Rust `tracing` subscriber via its `Tracing` context manager.
	#[pymodule_init]
	fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
		pyo3_tracing_subscriber::add_submodule(
			"qalc_sbox_py",
			"tracing_subscriber",
			m.py(),
			m,
		)?;
		Ok(())
	}

	#[pymodule]
	mod qalc_sandbox {
		use std::path::{Path, PathBuf};

		use anyhow::{Context, anyhow};
		use pyo3::{PyResult, Python, pyclass, pymethods, types::PyAnyMethods};
		use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
		use qalc_sbox::Sandbox;

		/// Path of `qalc_sbox_worker`, bundled by the wheel build next to
		/// this package's compiled extension (`qalc_sbox_worker`, a sibling
		/// of `qalc_sbox_py/__init__<ext>`).
		fn bundled_worker_path(py: Python<'_>) -> PyResult<PathBuf> {
			let module_file: String = py
				.import("qalc_sbox_py")?
				.getattr("__file__")?
				.extract()?;
			let dir = Path::new(&module_file)
				.parent()
				.with_context(|| {
					format!(
						"qalc_sbox_py module path {module_file:?} has no parent directory"
					)
				})?;
			let worker = dir.join("qalc_sbox_worker");
			ensure_executable(&worker)?;
			Ok(worker)
		}

		fn ensure_executable(path: &Path) -> PyResult<()> {
			use std::os::unix::fs::PermissionsExt as _;
			let mut perms = std::fs::metadata(path)
				.with_context(|| format!("failed to stat {}", path.display()))?
				.permissions();
			if perms.mode() & 0o111 == 0 {
				perms.set_mode(perms.mode() | 0o755);
				std::fs::set_permissions(path, perms).with_context(|| {
					format!("failed to make {} executable", path.display())
				})?;
			}
			Ok(())
		}

		#[gen_stub_pyclass]
		#[pyclass(module = "qalc_sbox_py.qalc_sandbox")]
		#[derive(Debug)]
		/// A sandboxed wrapper around libqalculate's Calculator
		///
		/// It is safe to pass untrusted user input to this class
		pub struct Qalculator {
			/// The inner sandboxed calculator
			inner: Sandbox,
		}

		#[gen_stub_pymethods]
		#[pymethods]
		impl Qalculator {
			/// `fork(2)` the current process to create the sandbox
			///
			/// # Warning
			///
			/// This is only safe to call while no other threads are alive in
			/// the current process. [`Sandbox::try_new_fork`]'s Rust docs for why).
			///
			/// Prefer [`Self::create_worker_exe`], which is always safe.
			#[staticmethod]
			pub fn create_fork() -> PyResult<Self> {
				// SAFETY: caller assumes deadlock risk;
				let inner = unsafe { Sandbox::try_new_fork() }
					.context("Failed to create sandbox")?;
				Ok(Self { inner })
			}

			#[staticmethod]
			/// Create the sandbox from a worker executable
			///
			/// This is preferable over [`Self::create_fork`]; however, unlike [`Self::create_fork`]
			/// it requires a worker executable
			pub fn create_worker_exe(path: &str) -> PyResult<Self> {
				let inner = Sandbox::try_new_exec(path)
					.context("Failed to create sandbox")?;
				Ok(Self { inner })
			}

			/// Create the sandbox using the `qalc_sbox_worker` binary bundled
			/// alongside this package
			#[staticmethod]
			pub fn create(py: Python<'_>) -> PyResult<Self> {
				let inner = Sandbox::try_new_exec(bundled_worker_path(py)?)
					.context("Failed to create sandbox")?;
				Ok(Self { inner })
			}

			/// send a string to the calculator sandbox over IPC and return the result
			///
			/// This method **WILL** block the calling thread; however, the GIL will be released
			///
			/// It is reccomended to use this method from a separate python thread
			pub fn calculate(
				&self,
				py: Python<'_>,
				content: &str,
			) -> PyResult<String> {
				let c = String::from(content);
				py.detach(|| {
					let res = self
						.inner
						.eval_blocking(c)
						.context(
							"Sandbox Error: Failed to evaluate expression",
						)?
						.map_err(|e| {
							anyhow!(
								"Qalculate Error: Failed to evaluate expression: {e}"
							)
						})?;
					Ok(res)
				})
			}

			/// Get the memory usage of the sandbox process in bytes
			///
			/// if the sandbox was created via [`Self::create_fork`] it will
			/// also include the memory usage of the parent process;
			/// however, these are copy-on-write pages and do
			/// not consume any additional memory
			pub fn memory_usage(&self) -> PyResult<u64> {
				Ok(self
					.inner
					.memory_usage()
					.context("Failed to get memory usage")?)
			}
		}
	}
}

define_stub_info_gatherer!(stub_info);
