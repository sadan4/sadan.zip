use std::{
	fmt::Debug,
	sync::{Arc, LazyLock},
};

#[cfg(feature = "highlight")]
use miette::highlighters::SyntectHighlighter;
use miette::{
	GraphicalReportHandler,
	GraphicalTheme,
	highlighters::{BlankHighlighter, Highlighter},
};
use terminal_size::{Width, terminal_size};

#[derive(Clone)]
// FIXME: don't arc and impl for &Self instead
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
	miette::set_hook(Box::new(move |_| {
		Box::new(create_report_handler(with_color))
	}))
	.expect("Failed to set miette hook");
}

fn blank_highlighter() -> ArcHl {
	static BLANK_HL: LazyLock<ArcHl> =
		LazyLock::new(|| ArcHl(Arc::new(BlankHighlighter)));
	BLANK_HL.clone()
}

#[cfg(feature = "highlight")]
fn get_highlighter(with_color: bool) -> ArcHl {
	if with_color {
		static HL: LazyLock<ArcHl> = LazyLock::new(|| {
			let bts: &[u8] = include_bytes!("./syntaxes.bin");
			let syntax_raw = zstd::decode_all(bts)
				.expect("failed to decompress syntax data");
			let syntax_set = bitcode::deserialize(&syntax_raw)
				.expect("failed to deserialize syntax data");
			let bts: &[u8] = include_bytes!("./theme.bin");
			let theme_raw =
				zstd::decode_all(bts).expect("failed to decompress theme data");
			let theme = bitcode::deserialize(&theme_raw)
				.expect("failed to deserialize theme data");
			ArcHl(Arc::new(SyntectHighlighter::new(syntax_set, theme, false)))
		});
		HL.clone()
	} else {
		blank_highlighter()
	}
}

/// Without the `highlight` feature there is no syntax highlighter to build, so
/// colored output falls back to the plain renderer.
#[cfg(not(feature = "highlight"))]
fn get_highlighter(_with_color: bool) -> ArcHl {
	blank_highlighter()
}

fn create_report_handler(with_color: bool) -> GraphicalReportHandler {
	make_handler(get_highlighter(with_color))
}

pub fn mk_test_handler() -> GraphicalReportHandler {
	make_handler(get_highlighter(false)).with_theme(GraphicalTheme::none())
}

fn make_handler(highlighter: ArcHl) -> GraphicalReportHandler {
	GraphicalReportHandler::new()
		.with_width(
			terminal_size().map_or(80, |(Width(width), _)| usize::from(width)),
		)
		.with_cause_chain()
		.with_syntax_highlighting(highlighter)
}
