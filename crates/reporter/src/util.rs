use std::{borrow::Cow, mem, str::Utf8Error, time::Duration};

use bytes::Bytes;
use derive_more::{Deref, DerefMut, From};
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
	#[allow(clippy::literal_string_with_formatting_args)]
	pub fn new(msg: &'static str, n: Option<usize>) -> Self {
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
	pub fn and_attach(self, target: &MultiProgress) -> Self {
		target.add(self.0.clone());
		self
	}
	pub fn step(&self) {
		self.0.inc(1);
	}
	pub const fn step_guard(&self) -> StageStepGuard<'_> {
		StageStepGuard(self)
	}
	pub fn msg(&self, msg: impl Into<Cow<'static, str>>) {
		self.0.set_message(msg);
	}
}

pub struct StageStepGuard<'a>(&'a Stage);

impl StageStepGuard<'_> {
	pub const fn forget(self) {
		mem::forget(self);
	}
}

impl Drop for StageStepGuard<'_> {
	fn drop(&mut self) {
		self.0.step();
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
