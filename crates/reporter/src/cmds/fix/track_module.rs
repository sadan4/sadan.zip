use explorer_types::ModuleId;

use crate::fetcher::ScrapedOutput;

pub struct ModuleTracker<'a> {
	prev_build: &'a ScrapedOutput,
	prev_target_id: ModuleId,
	next_build: &'a ScrapedOutput,
}

impl<'a> ModuleTracker<'a> {

}
