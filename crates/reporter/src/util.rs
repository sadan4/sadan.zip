use std::{
	borrow::Cow,
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
use tokio::sync::mpsc;

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
