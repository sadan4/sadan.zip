use std::{borrow::Cow, io, time::Duration};

use anyhow::{Context as _, Result};
use derive_more::{Deref, DerefMut, From};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressFinish, ProgressStyle};
use serde::de::DeserializeOwned;

pub fn read_struct<T: DeserializeOwned>(from: impl io::Read) -> Result<T> {
    let mpk_raw_data = zstd::decode_all(from).context("failed to decompress struct")?;
    let data: T = rmp_serde::from_slice(&mpk_raw_data).context("failed to deserialize struct")?;
    Ok(data)
}

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
        let bar = bar.with_prefix(msg).with_finish(ProgressFinish::AndLeave);
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
    pub fn msg(&self, msg: impl Into<Cow<'static, str>>) {
        self.0.set_message(msg);
    }
}
