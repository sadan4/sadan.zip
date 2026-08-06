use pyo3::pymodule;

#[pymodule]
mod qalc_sbox_py {
	use pyo3::pymodule;

	#[pymodule]
	mod qalc_sandbox {
		use anyhow::{Context, anyhow};
		use pyo3::{PyResult, Python, pyclass, pymethods};
		use qalc_sbox::Sandbox;

		#[pyclass]
		#[derive(Debug)]
		/// A sandboxed wrapper around libqalculate's Calculator
		///
		/// It is safe to pass untrusted user input to this class
		pub struct Qalculator {
			/// The inner sandboxed calculator
			inner: Sandbox,
		}

		#[pymethods]
		impl Qalculator {
			/// `fork(2)` the current process to create the sandbox
			#[staticmethod]
			pub fn create_fork() -> PyResult<Self> {
				let inner =
					Sandbox::try_new().context("Failed to create sandbox")?;
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
