//! The `download_modules` command: asks the connected client for every
//! webpack module it has, formats them, and dumps them into `.modules`.

use std::{
	fmt::Write as _,
	path::Path,
	sync::atomic::{AtomicUsize, Ordering},
	time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail};
use explorer_types::ModuleId;
use futures_util::{StreamExt as _, TryStreamExt as _, stream};
use pretty_printer::format_with_alloc;
use rayon::iter::{IntoParallelRefMutIterator as _, ParallelIterator as _};
use tokio::{fs, task};
use tower_lsp_server::ls_types::{MessageType, WorkDoneProgressBegin};
use tracing::{info, warn};
use webpack_ast_parser::WebpackAstParser;

use crate::{
	lsp::Server,
	util::progress::OwnedProgressHandle,
	wss::types::to_client::{ExtractMessage, FindQuery, LoadModulesMessage},
};

/// A module list takes the client a while to put together, far longer than a
/// normal response.
const MODULE_LIST_TIMEOUT: Duration = Duration::from_secs(120);

/// The indent the dumped modules are formatted with; `0` means tabs.
const INDENT: u8 = 2;

/// How many module downloads are kept in flight at once.
const DOWNLOAD_CONCURRENCY: usize = 32;

fn percent(done: usize, total: usize) -> u32 {
	if total == 0 {
		return 100;
	}
	(done as f32 * 100.0 / total as f32).floor() as u32
}

impl Server {
	/// Download every module the client has, format them, and write them to
	/// the module directory.
	///
	/// Refuses to run when that directory already exists; clearing it is the
	/// `clear_cache` command's job.
	pub(super) async fn download_modules(&self) -> Result<()> {
		let module_root = self.module_cache.module_dir_target()?;
		if fs::try_exists(&module_root)
			.await
			.unwrap_or(false)
		{
			bail!(
				"{} already exists, run the clear cache command first",
				module_root.display()
			);
		}
		let start = Instant::now();
		let ids = self.module_ids().await?;
		let mut modules = self.download_module_text(&ids).await?;
		let download_time = start.elapsed();
		let format_start = Instant::now();
		let unformatted = self.format_modules(&mut modules);
		let format_time = format_start.elapsed();
		let write_start = Instant::now();
		self.write_modules(&module_root, &modules)
			.await?;
		let write_time = write_start.elapsed();
		info!(
			modules = modules.len(),
			?download_time,
			?format_time,
			?write_time,
			total_time =? start.elapsed(),
			"Downloaded, formatted and wrote modules"
		);
		let mut msg = format!(
			"Wrote {} modules to {}",
			modules.len(),
			module_root.display()
		);
		if unformatted != 0 {
			write!(
				msg,
				" ({unformatted} could not be formatted, written as-is)"
			)
			.unwrap();
		}
		self.client
			.show_message(MessageType::INFO, msg)
			.await;
		Ok(())
	}

	/// The ids of every module the client has.
	async fn module_ids(&self) -> Result<Vec<ModuleId>> {
		let list = self
			.state
			.ws
			.send_msg_timeout(
				LoadModulesMessage::default(),
				MODULE_LIST_TIMEOUT,
			)
			.await
			.context("Failed to get the module list from the client")?;
		let mut ids = Vec::with_capacity(list.modules.len());
		for raw in &list.modules {
			match raw.parse().map(ModuleId) {
				Ok(id) => ids.push(id),
				Err(e) => {
					warn!(
						%raw,
						"Client sent a module id that is not a number, skipping: {e}"
					);
				}
			}
		}
		info!(modules = ids.len(), "Got the module list from the client");
		Ok(ids)
	}

	/// Ask the client for the source of each module in `ids`.
	async fn download_module_text(
		&self,
		ids: &[ModuleId],
	) -> Result<Vec<(ModuleId, String)>> {
		let handle = self.progress("Downloading modules");
		let total = ids.len();
		let done = AtomicUsize::new(0);
		let handle = &handle;
		let done = &done;
		stream::iter(ids.iter().copied())
			.map(|id| async move {
				let res = self
					.state
					.ws
					.send_msg(ExtractMessage {
						data: FindQuery::Id {
							id,
							use_patched: false,
						},
					})
					.await
					.with_context(|| {
						format!("Failed to download module {id}")
					})?;
				let done = done.fetch_add(1, Ordering::Relaxed) + 1;
				handle.step(
					percent(done, total),
					format!("Downloading module {id} ({done}/{total})"),
				);
				Ok((id, res.module))
			})
			.buffer_unordered(DOWNLOAD_CONCURRENCY)
			.try_collect()
			.await
	}

	/// Give each module its webpack header and pretty print it, in place.
	///
	/// Returns how many could not be formatted; those keep their downloaded
	/// source, which parses just as well as a formatted one.
	fn format_modules(&self, modules: &mut [(ModuleId, String)]) -> usize {
		let handle = self.progress("Formatting modules");
		let total = modules.len();
		let done = AtomicUsize::new(0);
		let failed = AtomicUsize::new(0);
		// cpu-bound AST parsing; block_in_place keeps rayon from starving the
		// tokio worker this runs on
		task::block_in_place(|| {
			modules
				.par_iter_mut()
				.for_each(|(id, text)| {
					let done = done.fetch_add(1, Ordering::Relaxed) + 1;
					handle.step(
						percent(done, total),
						format!("Formatting module {id} ({done}/{total})"),
					);
					WebpackAstParser::format_module_header(text, *id, false);

					let alloc = self.pool.get();
					let formatted = format_with_alloc(text, &alloc, INDENT)
						.map(|content| content.code);
					match formatted {
						Ok(code) => *text = code,
						Err(e) => {
							failed.fetch_add(1, Ordering::Relaxed);
							warn!(
								%id,
								"Failed to format module, writing it as-is: {e}"
							);
						}
					}
				});
		});
		let failed = failed.into_inner();
		if failed != 0 {
			warn!(failed, "Some modules could not be formatted");
		}
		failed
	}

	/// Write each module to `<module_root>/<id>.js`.
	async fn write_modules(
		&self,
		module_root: &Path,
		modules: &[(ModuleId, String)],
	) -> Result<()> {
		fs::create_dir_all(module_root)
			.await
			.with_context(|| {
				format!(
					"Failed to create the module directory at {}",
					module_root.display()
				)
			})?;
		let handle = self.progress("Writing modules");
		let total = modules.len();
		// TODO: concurrent writes?
		for (done, (id, text)) in modules.iter().enumerate() {
			handle.step(
				percent(done, total),
				format!("Writing module {id} ({}/{total})", done + 1),
			);
			// the id parsed as a number, so it cannot walk out of the module
			// directory
			let path = module_root.join(format!("{id}.js"));
			fs::write(&path, text)
				.await
				.with_context(|| {
					format!("Failed to write module {id} to {}", path.display())
				})?;
		}
		Ok(())
	}

	/// A progress bar titled `title`.
	fn progress(&self, title: &str) -> OwnedProgressHandle {
		Self::start_progress(
			self.client.clone(),
			WorkDoneProgressBegin {
				title: title.to_owned(),
				..Default::default()
			},
		)
	}
}
