pub trait ScrapeProgress: Send + Sync + 'static {
	fn set_stage(&self, _msg: &'static str) {}
	fn set_chunk_total(&self, _total: usize) {}
	fn chunk_finished(&self) {}
}

pub struct NoProgress;

impl ScrapeProgress for NoProgress {}
