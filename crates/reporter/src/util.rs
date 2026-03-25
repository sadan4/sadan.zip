use std::{borrow::Cow, io, time::Duration};

use anyhow::{Context as _, Result};
use derive_more::{Deref, DerefMut, From};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
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
    #[expect(clippy::literal_string_with_formatting_args)]
    pub fn new(msg: &'static str, n: Option<u32>) -> Self {
        let bar = n.map_or_else(
            || {
                let bar = ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden())
                    .with_prefix(msg)
                    .with_style(ProgressStyle::with_template("{spinner} {prefix} {msg}").unwrap());
                bar.enable_steady_tick(Duration::from_millis(1000 / 20));
                bar
            },
            |n| {
                ProgressBar::with_draw_target(Some(n.into()), ProgressDrawTarget::hidden())
                    .with_prefix(msg)
                    .with_style(
                        ProgressStyle::with_template(
                            "{spinner} {prefix} {msg} {bar} ({pos}/{len})",
                        )
                        .unwrap(),
                    )
            },
        );
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
