use anyhow::Result;
use itertools::Itertools;
use tokio::join;
use tower_lsp::lsp_types::{
	Hover,
	HoverContents,
	HoverParams,
	MarkupContent,
	MarkupKind,
};
use tracing::{error, warn};

use crate::{LspResult, lsp, util::range};

mod export_hover;
mod intl_hover;

fn map_hover_result_impl<T>(
	res: Result<Option<T>>,
	dbg_name: &str,
) -> Option<T> {
	match res {
		Err(e) => {
			error!("Error providing hover for {dbg_name}: {e}");
			None
		}
		Ok(r) => r,
	}
}

macro_rules! map_hover_result {
	($res:ident) => {
		let $res = map_hover_result_impl($res, stringify!($res));
	};
}

impl lsp::Server {
	pub(super) async fn provide_hover(
		&self,
		params: HoverParams,
	) -> LspResult<Option<Hover>> {
		let (intl_hover, export_hover) = join!(
			self.provide_intl_hover(&params),
			self.provide_export_hover(&params)
		);
		map_hover_result!(intl_hover);
		map_hover_result!(export_hover);
		let items = [intl_hover, export_hover]
			.into_iter()
			.flatten()
			.collect_vec();
		if items.is_empty() {
			return Ok(None);
		}
		let range = items
			.iter()
			.map(|(_, r)| Some(*r))
			.reduce(|r1, r2| range::intersect(r1?, r2?))
			.flatten();
		if range.is_none() {
			warn!("Hover ranges do not intersect: {items:#?}");
		}
		let md_str = items
			.into_iter()
			.map(|(s, _)| s)
			.join("\n\n---\n\n");
		Ok(Some(Hover {
			contents: HoverContents::Markup(MarkupContent {
				kind: MarkupKind::Markdown,
				value: md_str,
			}),
			range,
		}))
	}
}
