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

type RegisteredCmd = (&'static str, CmdFunc);

static CMD_MAP: phf::Map<&'static str, CmdFunc> = phf::phf_map! {
	"hello_world" => super::Server::hello_world_cmd,
};

impl super::Server {
	pub(super) fn get_cmd_provider(&self) -> ExecuteCommandOptions {
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
		if let Some(cmd_func) = CMD_MAP.get(&params.command) {
			cmd_func(self, params).await
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
