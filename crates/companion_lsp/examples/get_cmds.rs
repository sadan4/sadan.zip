use std::io;

use companion_lsp::{SERVER_NAME, lsp};
use serde::Serialize;

fn main() {
	let mut w = io::stdout();
	for (name, desc) in lsp::cmds::CMD_MAP.entries() {
		#[derive(Serialize)]
		struct O<'a> {
			command: String,
			title: &'a str,
		}
		serde_json::to_writer(
			&mut w,
			&O {
				command: format!("{SERVER_NAME}.{name}"),
				title: desc.desc,
			},
		)
		.expect("failed to serialize output");
		println!();
	}
}
