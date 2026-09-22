use std::{borrow::Cow, pin::Pin};

use crate::{
	JValue,
	LspResult,
	SERVER_NAME,
	lsp::{custom::QuickPickRequest, lenses::PatchLensArgs},
	util::err::is_caused_by,
	wss::NoClientsError,
};
use anyhow::{Context as _, Result, anyhow, bail, ensure};
use const_format::formatc;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use smol_str::SmolStr;
use tower_lsp_server::{
	jsonrpc,
	ls_types::{
		ExecuteCommandOptions,
		ExecuteCommandParams,
		MessageType,
		WorkDoneProgressOptions,
	},
};
use tracing::{debug, warn};
use tracing_subscriber::EnvFilter;

type CmdFunc = for<'fut> fn(
	&'fut super::Server,
	ExecuteCommandParams,
) -> Pin<
	Box<dyn Future<Output = Result<Option<JValue>>> + Send + 'fut>,
>;

#[derive(Clone, Copy)]
pub struct CommandDescriptor {
	pub desc: &'static str,
	/// Should the user be able to see and run this command
	pub user_visible: bool,
	pub func: CmdFunc,
}

pub static CMD_MAP: phf::Map<&'static str, CommandDescriptor> = phf::phf_map! {
	"set_log_level" => CommandDescriptor {
		desc: "Set Log Level",
		user_visible: true,
		func: super::Server::set_log_level_cmd,
	},
	"download_modules" => CommandDescriptor {
		desc: "Download Module Cache",
		user_visible: true,
		func: super::Server::download_modules_cmd,
	},
	"clear_cache" => CommandDescriptor {
		desc: "Purge Module Cache",
		user_visible: true,
		func: super::Server::clear_cache_cmd,
	},
	"clear_live_cache" => CommandDescriptor {
		desc: "Purge Live Module Cache",
		user_visible: true,
		func: super::Server::clear_live_cache_cmd,
	},
	"open_patch_helper" => CommandDescriptor {
		desc: "",
		user_visible: false,
		func: super::Server::open_patch_helper_cmd,
	},
	"test_patch" => CommandDescriptor {
		desc: "",
		user_visible: false,
		func: super::Server::test_patch_cmd,
	}
};

impl super::Server {
	const COMMAND_PREFIX: &str = formatc!("{SERVER_NAME}.");

	/// The name a client sees for the command registered under `key` in
	/// [`CMD_MAP`], i.e. `key` with [`Self::COMMAND_PREFIX`] prepended.
	pub(crate) fn cmd_name(key: &str) -> String {
		format!("{}{key}", Self::COMMAND_PREFIX)
	}

	pub(crate) fn copy_string_cmd_uri(s: &str) -> String {
		let input = serde_json::to_string(&[s]).unwrap();
		format!(
			"command:{}copy?{}",
			Self::COMMAND_PREFIX,
			percent_encode(input.as_bytes(), NON_ALPHANUMERIC)
		)
	}

	pub fn get_cmd_provider() -> ExecuteCommandOptions {
		let commands = CMD_MAP
			.keys()
			.map(|s| Self::cmd_name(s))
			.collect();
		ExecuteCommandOptions {
			commands,
			work_done_progress_options: WorkDoneProgressOptions {
				work_done_progress: Some(true),
			},
		}
	}

	pub async fn handle_cmd(
		&self,
		params: ExecuteCommandParams,
	) -> LspResult<Option<JValue>> {
		if let Some(cmd) = CMD_MAP.get(
			params
				.command
				.trim_prefix(Self::COMMAND_PREFIX),
		) {
			let command = params.command.clone();
			let ret = (cmd.func)(self, params).await;
			return match ret {
				Ok(v) => Ok(v),
				Err(e) => {
					if is_caused_by::<NoClientsError>(&*e) {
						self.client
							.show_message(MessageType::ERROR, NoClientsError)
							.await;
						Ok(None)
					} else {
						warn!("Error executing command {}: {e:?}", command);
						Err(jsonrpc::Error {
							message: Cow::Owned(format!("{e}")),
							..jsonrpc::Error::internal_error()
						})
					}
				}
			};
		}
		Err(jsonrpc::Error::method_not_found())
	}

	fn set_log_level_cmd(
		&self,
		params: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			let handle = self
				.log_reload_handle
				.as_ref()
				.context("Internal Error: log reload handle not set")?;
			let filter_str = match params.arguments.first() {
				Some(JValue::String(s)) => s.clone(),
				Some(other) => {
					bail!(
						"expected first argument to be a string, got {other:?}"
					);
				}
				None => {
					debug!("no argument provided, asking user");

					self.quick_pick(QuickPickRequest {
						items: vec![
							SmolStr::new_static("info"),
							SmolStr::new_static("debug"),
							SmolStr::new_static("trace"),
							SmolStr::new_static("warn"),
							SmolStr::new_static("error"),
						],
						placeholder: Some(SmolStr::new_static(
							"Enter a log filter string",
						)),
						allow_free_text: true,
					})
					.await
					.map_err(|e| {
						anyhow!("failed to get log filter from user: {e}")
					})?
					.context("user cancelled log filter prompt")?
				}
			};
			handle
				.reload(
					EnvFilter::try_new(&filter_str)
						.context("failed to parse log filter string")?,
				)
				.context("failed to reload log filter")?;
			Ok(None)
		})
	}

	fn download_modules_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			self.download_modules().await?;
			Ok(None)
		})
	}

	fn clear_cache_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			const CONFIRM: &str = "Delete";

			let answer = self
				.quick_pick(QuickPickRequest {
					// cancel first so the default selection is the safe one
					items: vec![
						SmolStr::new_static("Cancel"),
						SmolStr::new_static(CONFIRM),
					],
					placeholder: Some(SmolStr::new_static(
						"Delete the dumped modules and their caches?",
					)),
					allow_free_text: false,
				})
				.await
				.map_err(|e| {
					anyhow!("failed to get confirmation from user: {e}")
				})?;
			if answer.as_deref() != Some(CONFIRM) {
				debug!("user cancelled clearing the module cache");
				return Ok(None);
			}
			let removed = self.module_cache.clear().await?;
			self.client
				.show_message(
					MessageType::INFO,
					format!(
						"Cleared the module cache at {}",
						removed.display()
					),
				)
				.await;
			Ok(None)
		})
	}

	/// Drop the modules fetched from the running client.
	///
	/// No confirmation, unlike [`Self::clear_cache_cmd`]: nothing on disk goes
	/// away and every module is re-fetched the next time it is asked for.
	fn clear_live_cache_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			let dropped = self.module_cache.clear_live();
			self.client
				.show_message(
					MessageType::INFO,
					format!("Cleared {dropped} live modules"),
				)
				.await;
			Ok(None)
		})
	}

	fn open_patch_helper_cmd(
		&self,
		mut params: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			ensure!(
				params.arguments.len() == 1,
				"expected exactly one argument for open_patch_helper"
			);
			let args: PatchLensArgs = match params.arguments.swap_remove(0) {
				JValue::Object(map) => serde_json::from_value(JValue::Object(
					map,
				))
				.context("Failed to parse arguments for open_patch_helper")?,
				other => {
					bail!(
						"expected first argument to be an object, got {other:?}"
					);
				}
			};
			self.open_patch_helper(args).await?;
			Ok(None)
		})
	}

	fn test_patch_cmd(
		&self,
		mut params: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			ensure!(
				params.arguments.len() == 1,
				"expected exactly one argument for test_patch"
			);
			let args: PatchLensArgs = match params.arguments.swap_remove(0) {
				JValue::Object(map) => {
					serde_json::from_value(JValue::Object(map))
						.context("Failed to parse arguments for test_patch")?
				}
				other => {
					bail!(
						"expected first argument to be an object, got {other:?}"
					);
				}
			};
			self.test_patch(args).await?;
			Ok(None)
		})
	}
}
