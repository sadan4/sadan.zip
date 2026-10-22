//! Terminal progress bars shared between the reporter and the scraper
//! examples.
use std::{
	borrow::Cow,
	io,
	mem,
	sync::{Arc, Mutex},
	time::Duration,
};

use derive_more::{Debug, Deref, DerefMut, From};
use indicatif::{
	MultiProgress,
	ProgressBar,
	ProgressDrawTarget,
	ProgressFinish,
	ProgressStyle,
};

#[derive(Debug, From, Deref, DerefMut)]
pub struct Stage(pub ProgressBar);

pub struct StepGuard<'a> {
	this: &'a Stage,
}

impl Drop for StepGuard<'_> {
	fn drop(&mut self) {
		self.this.step();
	}
}

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
	pub fn hidden() -> Self {
		Self(ProgressBar::with_draw_target(
			None,
			ProgressDrawTarget::hidden(),
		))
	}
	#[must_use]
	pub fn and_attach(self, target: &MultiProgressWrapper) -> Self {
		target.add(self.0.clone());
		self
	}
	pub fn step(&self) {
		self.0.inc(1);
	}
	pub const fn step_guard(&self) -> StepGuard<'_> {
		StepGuard { this: self }
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
	/// A stderr writer that hides the bars while writing, so log lines
	/// don't get mangled by redraws.
	///
	/// Meant for `tracing_subscriber::fmt().with_writer(...)`
	pub fn writer(&self) -> MultiProgressWriter {
		MultiProgressWriter(self.inner.clone())
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

#[derive(Debug, Clone)]
pub struct MultiProgressWriter(MultiProgress);

impl io::Write for MultiProgressWriter {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.0
			.suspend(|| io::stderr().lock().write(buf))
	}

	// no need to flush stderr, rust doesn't either
	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}
