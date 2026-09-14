use std::{borrow::Cow, pin::Pin, time::Duration};

use crate::{JValue, LspResult, SERVER_NAME, lsp::custom::QuickPickRequest};
use anyhow::{Context as _, Result, anyhow, bail};
use const_format::formatc;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use smol_str::SmolStr;
use tokio::time;
use tower_lsp::{
	jsonrpc,
	lsp_types::{
		ExecuteCommandOptions,
		ExecuteCommandParams,
		WorkDoneProgressBegin,
		WorkDoneProgressOptions,
	},
};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

type CmdFunc = for<'fut> fn(
	&'fut super::Server,
	ExecuteCommandParams,
) -> Pin<
	Box<dyn Future<Output = Result<Option<JValue>>> + Send + 'fut>,
>;

pub struct CommandDescriptor {
	pub desc: &'static str,
	func: CmdFunc,
}

pub static CMD_MAP: phf::Map<&'static str, CommandDescriptor> = phf::phf_map! {
	"set_log_level" => CommandDescriptor {
		desc: "Set the log level of the server",
		func: super::Server::set_log_level_cmd,
	},
	"progress_test" => CommandDescriptor {
		desc: "Test the progress reporting",
		func: super::Server::progress_test_cmd,
	},
};

impl super::Server {
	const COMMAND_PREFIX: &str = formatc!("{SERVER_NAME}.");

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
			.map(|s| format!("{}{s}", Self::COMMAND_PREFIX))
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
			return (cmd.func)(self, params)
				.await
				.map_err(|e| jsonrpc::Error {
					message: Cow::Owned(format!("{e}")),
					..jsonrpc::Error::internal_error()
				});
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
}
