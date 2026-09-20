use std::io;

use companion_lsp::{
	SERVER_NAME,
	lsp::{self, cmds::CommandDescriptor},
};
use serde::Serialize;

fn main() {
	let mut w = io::stdout();
	for (name, desc) in lsp::cmds::CMD_MAP.entries() {
		let CommandDescriptor {
			desc,
			user_visible,
			func: _,
		} = *desc;
		if !user_visible {
			continue;
		}
		#[derive(Serialize)]
		struct O<'a> {
			command: String,
			title: &'a str,
		}
		serde_json::to_writer(
			&mut w,
			&O {
				command: format!("{SERVER_NAME}.{name}"),
				title: desc,
			},
		)
		.expect("failed to serialize output");
		println!();
	}
}
