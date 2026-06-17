use std::{borrow::Cow, mem, str::Utf8Error, time::Duration};

use bytes::Bytes;
use derive_more::{Debug, Deref, DerefMut, From};
use indicatif::{
	MultiProgress,
	ProgressBar,
	ProgressDrawTarget,
	ProgressFinish,
	ProgressStyle,
};

#[derive(From, Deref, DerefMut)]
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

#[derive(Default, Debug)]
pub struct MultiProgressWrapper {
	inner: MultiProgress,
	bars: std::sync::Mutex<Vec<ProgressBar>>,
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
}

#[derive(Debug)]
pub struct ByteStr(Bytes);

impl TryFrom<Bytes> for ByteStr {
	type Error = Utf8Error;

	fn try_from(value: Bytes) -> std::result::Result<Self, Self::Error> {
		if let Err(e) = str::from_utf8(&value) {
			Err(e)
		} else {
			Ok(Self(value))
		}
	}
}

impl AsRef<str> for ByteStr {
	fn as_ref(&self) -> &str {
		// SAFETY: ByteStr can only be constructed from valid UTF-8 bytes
		unsafe { str::from_utf8_unchecked(&self.0) }
	}
}

impl std::fmt::Display for ByteStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.as_ref().fmt(f)
	}
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
