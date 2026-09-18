use std::{
	borrow::Cow,
	pin::Pin,
	sync::Arc,
	time::Duration,
};

use crate::{
	JValue,
	LspResult,
	SERVER_NAME,
	lsp::{
		custom::{
			EphemeralDocument,
			QuickPickRequest,
			ephemera::{self, EphemeralChange},
		},
		lenses::PatchLensArgs,
	},
	util::err::is_caused_by,
	wss::NoClientsError,
};
use anyhow::{Context as _, Result, anyhow, bail, ensure};
use const_format::formatc;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use smol_str::SmolStr;
use tokio::time;
use tower_lsp_server::{
	jsonrpc,
	ls_types::{
		ExecuteCommandOptions,
		ExecuteCommandParams,
		MessageType,
		ShowDocumentParams,
		WorkDoneProgressBegin,
		WorkDoneProgressOptions,
		request::ShowDocument,
	},
};
use tracing::{debug, info, warn};
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
		desc: "Set the log level of the server",
		user_visible: true,
		func: super::Server::set_log_level_cmd,
	},
	"progress_test" => CommandDescriptor {
		desc: "Test the progress reporting",
		user_visible: true,
		func: super::Server::progress_test_cmd,
	},
	"ephemera_test" => CommandDescriptor {
		desc: "Create a test epehemeral document",
		user_visible: true,
		func: super::Server::ephemera_test_cmd,
	},
	"open_patch_helper" => CommandDescriptor {
		desc: "Open a patch in the patch helper",
		user_visible: false,
		func: super::Server::open_patch_helper_cmd,
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

	fn progress_test_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			let handle = Self::start_progress(
				self.client.clone(),
				WorkDoneProgressBegin {
					title: "Test Progress".into(),
					message: Some(String::from("Test Progress Message")),
					..Default::default()
				},
			);
			info!("starting progress test");
			for i in 1..=20 {
				handle.step(Some(i * 5), format!("Step {i} of 20"));
				time::sleep(Duration::SECOND).await;
			}
			info!("progress test complete");
			Ok(None)
		})
	}

	fn ephemera_test_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			let uri = ephemera::uri("/test-ephemeral-doc.txt").unwrap();
			self.create_ephemeral_document(EphemeralDocument {
				uri: uri.clone(),
				content: Arc::from("This is a test ephemeral document"),
			});
			self.client
				.send_request::<ShowDocument>(ShowDocumentParams {
					uri: uri.clone(),
					take_focus: Some(true),
					external: None,
					selection: None,
				})
				.await
				.context("Failed to show document")?
				.success
				.then_some(())
				.context("Client reported failing to show document")?;
			for i in 1..=10 {
				time::sleep(Duration::from_secs(5)).await;
				debug!("updating ephemeral document");
				self.update_ephemeral_document(EphemeralChange {
					uri: uri.clone(),
					content: Some(
						format!(
							"This is a test ephemeral document, updated {i} times"
						)
						.into(),
					),
					deleted: None,
				})
				.context("Failed to update ephemeral document")?;
			}
			self.delete_ephemeral_document(uri)
				.context("Failed to delete ephemeral document")?;
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
}
