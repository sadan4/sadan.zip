use std::{
	cmp::Reverse,
	collections::{HashMap, HashSet},
	fmt::Display,
	hash::BuildHasher,
};

use explorer_types::ModuleId;
use memchr::memmem::Finder;
use miette::miette;
use oxc_allocator::Allocator;
use rayon::prelude::*;
pub use term_progress::{MultiProgressWrapper, Stage};
use tokio::sync::mpsc;
use webpack_ast_parser::{WebpackAstParser, find::ScoredFindSequence};

pub fn sink_sender<T>(buffer: usize) -> mpsc::Sender<T>
where
	T: Send + 'static,
{
	let (tx, mut rx) = mpsc::channel(buffer);
	tokio::spawn(async move {
		let mut buf = Vec::with_capacity(buffer);
		loop {
			// returns 0 when channel is closed
			if rx.recv_many(&mut buf, buffer).await == 0 {
				break;
			}
		}
	});
	tx
}

pub async fn join_all<T>(
	futs: impl IntoIterator<Item = impl Future<Output = T>>,
) -> Vec<T> {
	let futs = futs.into_iter();
	let mut ret = Vec::with_capacity(futs.size_hint().0);
	for fut in futs {
		ret.push(fut.await);
	}
	ret
}

/// prints a debug url for a module
#[must_use = "This function returns a value that implements Display"]
pub fn debug_module_url(mid: ModuleId, hash: &str) -> impl Display + use<'_> {
	struct D<'a>(&'a str, ModuleId);
	impl Display for D<'_> {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "https://sadan.zip/e/view/{}/{}", self.0, self.1)
		}
	}
	D(hash, mid)
}

pub fn generate_unique_finds<S>(
	module_id: ModuleId,
	modules: &HashMap<ModuleId, String, S>,
	bars: &MultiProgressWrapper,
) -> miette::Result<Vec<ScoredFindSequence>>
where
	S: BuildHasher + Sync,
{
	let src = modules
		.get(&module_id)
		.ok_or_else(|| miette!("Module {} not found", module_id.0))?;

	// `src` as stored in the fetched build map may be missing the
	// `// Webpack Module` header (and the leading `0,` that turns a bare
	// `function(...) {}` into a parseable expression instead of an invalid,
	// unnamed function declaration). Add it if it's missing so parsing
	// doesn't fail on otherwise-valid modules.
	let mut owned_src;
	let src: &str = if WebpackAstParser::is_webpack_module(src) {
		src
	} else {
		owned_src = src.clone();
		WebpackAstParser::format_module_header(
			&mut owned_src,
			module_id,
			false,
		);
		&owned_src
	};

	let alloc = Allocator::new();
	let parser = WebpackAstParser::try_new(&alloc, src).map_err(|e| {
		miette!(
			"Failed to parse file: {}",
			pretty_printer::render_diag(e, src, &format!("{module_id}.js"))
		)
	})?;

	let finds = parser.generate_finds();

	let bar = Stage::new("Generating unique finds", Some(finds.len()))
		.and_attach(bars);

	let mut finds: Vec<_> = finds
		.into_par_iter()
		.filter(|find| {
			let ft = find.get_find(src);
			let finder = Finder::new(ft);

			let is_unique = !modules.par_iter().any(|(id, code)| {
				if *id == module_id {
					return false;
				}
				finder.find(code.as_bytes()).is_some()
			});

			bar.step();
			is_unique
		})
		.collect();

	finds.sort_by_key(|f| Reverse(f.score));

	// multiple non-overlapping token runs can resolve to the same source
	// substring (e.g. a repeated short expression); keep only the
	// highest-scoring occurrence of each distinct find string.
	let mut seen = HashSet::new();
	finds.retain(|f| seen.insert(f.get_find(src)));

	Ok(finds)
}
