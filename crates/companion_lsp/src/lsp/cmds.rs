use std::pin::Pin;

use crate::{JValue, SERVER_NAME};
use tower_lsp::{
	jsonrpc::{self, Result},
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
};


impl super::Server {
	pub fn get_cmd_provider() -> ExecuteCommandOptions {
		let commands = CMD_MAP
			.keys()
			.map(|s| format!("{SERVER_NAME}.{s}"))
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
	) -> Result<Option<JValue>> {
		if let Some(cmd) = CMD_MAP.get(&params.command) {
			(cmd.func)(self, params).await
		} else {
			Err(jsonrpc::Error::method_not_found())
		}
	}

	fn hello_world_cmd(
		&self,
		params: ExecuteCommandParams,
	) -> Pin<Box<dyn Future<Output = Result<Option<JValue>>> + Send + '_>> {
		info!(?params, "Hello World");
		Box::pin(async move { Ok(None) })
	}
}
