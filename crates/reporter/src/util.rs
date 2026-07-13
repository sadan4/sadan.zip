use std::{
	borrow::Cow,
	cmp::Reverse,
	collections::HashMap,
	fmt::Display,
	hash::BuildHasher,
	mem,
	sync::{Arc, Mutex},
	time::Duration,
};

use derive_more::{Debug, Deref, DerefMut, From};
use explorer_types::ModuleId;
use indicatif::{
	MultiProgress,
	ProgressBar,
	ProgressDrawTarget,
	ProgressFinish,
	ProgressStyle,
};
use memchr::memmem::Finder;
use miette::miette;
use oxc_allocator::Allocator;
use rayon::prelude::*;
use tokio::sync::mpsc;
use webpack_ast_parser::{WebpackAstParser, find::ScoredFindSequence};

#[derive(Debug, From, Deref, DerefMut)]
pub struct Stage(pub ProgressBar);

impl Drop for Stage {
	fn drop(&mut self) {
		self.0.finish();
	}
}

impl Stage {
	#[expect(clippy::literal_string_with_formatting_args)]
	pub fn new(msg: impl Into<Cow<'static, str>>, n: Option<usize>) -> Self {
		let bar = n.map_or_else(
            || {
                ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden()).with_style(
                    ProgressStyle::with_template("{spinner:.green} {prefix} {msg} [{elapsed:.yellow}]")
                        .unwrap(),
                )
            },
            |n| {
                ProgressBar::with_draw_target(Some(n as _), ProgressDrawTarget::hidden())
                    .with_style(
                        ProgressStyle::with_template(
                            "{spinner:.green} {prefix} {msg} {bar:40.cyan/red} ({pos:.green}/{len:.green}) [{elapsed:.yellow}]",
                        )
                        .unwrap(),
                    )
            },
        );
		let bar = bar
			.with_prefix(msg)
			.with_finish(ProgressFinish::AndLeave);
		bar.enable_steady_tick(Duration::from_millis(1000 / 20));
		Self(bar)
	}
	#[must_use]
	pub fn and_attach(self, target: &MultiProgressWrapper) -> Self {
		target.add(self.0.clone());
		self
	}
	pub fn step(&self) {
		self.0.inc(1);
	}
	pub fn msg(&self, msg: impl Into<Cow<'static, str>>) {
		self.0.set_message(msg);
	}
}

#[derive(Default, Debug, Clone)]
pub struct MultiProgressWrapper {
	inner: MultiProgress,
	bars: Arc<Mutex<Vec<ProgressBar>>>,
}

impl MultiProgressWrapper {
	pub const fn inner_(&self) -> &MultiProgress {
		&self.inner
	}
	fn add(&self, bar: ProgressBar) {
		self.inner.add(bar.clone());
		self.bars.lock().unwrap().push(bar);
	}
	pub fn clear(&self) {
		let bars = mem::take(&mut *self.bars.lock().unwrap());
		for bar in bars {
			self.inner.remove(&bar);
		}
	}
	pub fn suspend<R>(&self, f: impl FnOnce() -> R) -> R {
		self.inner.suspend(f)
	}
	/// Create a progress bar for testing.
	/// will never print anything
	pub fn null_bar() -> Self {
		Self {
			inner: MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
			bars: Arc::new(Mutex::new(Vec::new())),
		}
	}
}

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
		WebpackAstParser::format_module_header(&mut owned_src, module_id, false);
		&owned_src
	};

	let alloc = Allocator::new();
	let parser = WebpackAstParser::try_new(&alloc, src)
		.map_err(|e| miette!("Failed to parse file: {e:?}"))?;

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

	Ok(finds)
}
