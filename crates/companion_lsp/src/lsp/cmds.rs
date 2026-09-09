use std::{borrow::Cow, pin::Pin};

use crate::{JValue, LspResult, SERVER_NAME, lsp::custom::QuickPickRequest};
use anyhow::Result;
use const_format::formatc;
use smol_str::SmolStr;
use tower_lsp::{
	jsonrpc,
	lsp_types::{
		ExecuteCommandOptions,
		ExecuteCommandParams,
		WorkDoneProgressOptions,
	},
};
use tracing::info;

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
	"hello_world" => CommandDescriptor {
		desc: "Prints 'Hello World' to the log",
		func: super::Server::hello_world_cmd,
	},
	"hello_world_2" => CommandDescriptor {
		desc: "Prints 'Hello World' to the log. again.",
		func: super::Server::hello_world_cmd,
	},
	"qp_test" => CommandDescriptor {
		desc: "Quick Pick Test",
		func: super::Server::echo_cmd,
	},
};

impl super::Server {
	const COMMAND_PREFIX: &str = formatc!("{SERVER_NAME}.");

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

	fn echo_cmd(
		&self,
		params: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		Box::pin(async move {
			let msg = self
				.quick_pick(QuickPickRequest {
					items: Vec::new(),
					placeholder: Some(SmolStr::new_static("Hello, World!")),
					allow_free_text: true,
				})
				.await?;
			info!("Quick Pick Result: {msg:?}");
			Ok(None)
		})
	}

	fn hello_world_cmd(
		&self,
		_: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		todo!()
	}
}
