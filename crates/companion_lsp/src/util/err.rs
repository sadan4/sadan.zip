mod anyhow_no_backtrace;

pub use anyhow_no_backtrace::display_no_backtrace;

use std::error::Error as StdError;

/// Checks if `err` is caused by an error of type `T`, recursively checking the source chain.
pub fn is_caused_by<T: StdError + 'static>(err: &(dyn StdError + 'static)) -> bool {
	err.is::<T>()
		|| err
			.source()
			.is_some_and(|cause| is_caused_by::<T>(cause))
}
