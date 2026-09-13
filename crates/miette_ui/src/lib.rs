use std::{
	fmt::Debug,
	sync::Arc,
};

use miette::{
	GraphicalReportHandler,
	highlighters::{BlankHighlighter, Highlighter, SyntectHighlighter},
};
use terminal_size::{Width, terminal_size};

#[derive(Clone)]
struct ArcHl(Arc<dyn Highlighter + Send + Sync + 'static>);

impl Debug for ArcHl {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ArcHl(...)")
	}
}

impl Highlighter for ArcHl {
	fn start_highlighter_state<'h>(
		&'h self,
		source: &dyn miette::SpanContents<'_>,
	) -> Box<dyn miette::highlighters::HighlighterState + 'h> {
		let inner = self.0.as_ref();
		inner.start_highlighter_state(source)
	}
}

pub fn install_miette_hook(with_color: bool) {
	let highlighter = ArcHl(if with_color {
		let bts: &[u8] = include_bytes!("./syntaxes.bin");
		let syntax_raw =
			zstd::decode_all(bts).expect("failed to decompress syntax data");
		let syntax_set = bitcode::deserialize(&syntax_raw)
			.expect("failed to deserialize syntax data");
		let bts: &[u8] = include_bytes!("./theme.bin");
		let theme_raw =
			zstd::decode_all(bts).expect("failed to decompress theme data");
		let theme = bitcode::deserialize(&theme_raw)
			.expect("failed to deserialize theme data");
		Arc::new(SyntectHighlighter::new(syntax_set, theme, false))
	} else {
		Arc::new(BlankHighlighter)
	});
	miette::set_hook(Box::new(move |_| {
		Box::new(
			GraphicalReportHandler::new()
				.with_width(
					terminal_size()
						.map_or(80, |(Width(width), _)| usize::from(width)),
				)
				.with_cause_chain()
				.with_syntax_highlighting(highlighter.clone()),
		)
	}))
	.expect("Failed to set miette hook");
}
