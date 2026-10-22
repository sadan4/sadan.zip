use std::{borrow::Cow, sync::Mutex};

use explorer_types::Channel;
use term_progress::{MultiProgressWrapper, Stage};

pub trait ScrapeProgress: Send + Sync + 'static {
	fn set_stage(&self, _msg: &'static str) {}
	/// Called once, with the total number of chunks (js and css) that will
	/// be scraped, including any that already finished
	fn set_chunk_total(&self, _total: usize) {}
	fn chunk_finished(&self) {}
	fn make_stage(&self, msg: Cow<'static, str>, n: Option<usize>) -> Stage {
		_ = msg;
		_ = n;
		Stage::hidden()
	}
}

pub struct NoProgress;

impl ScrapeProgress for NoProgress {}

/// Renders scrape progress as terminal progress bars.
///
/// A spinner shows the current stage until the chunk total is known, then a
/// bar tracks the chunks.
pub struct BarProgress {
	bars: MultiProgressWrapper,
	channel: Channel,
	state: Mutex<BarState>,
}

enum BarState {
	/// chunks that finished before the total was known
	Pre {
		stage: Stage,
		finished: usize,
	},
	Chunks(Stage),
}

impl BarProgress {
	pub fn new(bars: MultiProgressWrapper, channel: Channel) -> Self {
		let stage =
			Stage::new(format!("[{channel:?}]: Scraping build data: "), None)
				.and_attach(&bars);
		Self {
			bars,
			channel,
			state: Mutex::new(BarState::Pre { stage, finished: 0 }),
		}
	}
}

impl ScrapeProgress for BarProgress {
	fn set_stage(&self, msg: &'static str) {
		if let BarState::Pre { stage, .. } = &*self.state.lock().unwrap() {
			stage.msg(msg);
		}
	}

	fn set_chunk_total(&self, total: usize) {
		let mut state = self.state.lock().unwrap();
		let BarState::Pre { finished, .. } = *state else {
			panic!("set_chunk_total called twice");
		};
		let bar = Stage::new(
			format!("[{:?}]: Scraping chunks: ", self.channel),
			Some(total),
		)
		.and_attach(&self.bars);
		bar.inc(finished as u64);
		// implicitly drops any spinners, finishing them
		*state = BarState::Chunks(bar);
	}

	fn chunk_finished(&self) {
		match &mut *self.state.lock().unwrap() {
			BarState::Pre { finished, .. } => *finished += 1,
			BarState::Chunks(bar) => bar.step(),
		}
	}
	fn make_stage(&self, msg: Cow<'static, str>, n: Option<usize>) -> Stage {
		Stage::new(msg, n).and_attach(&self.bars)
	}
}
