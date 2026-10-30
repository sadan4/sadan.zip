//! Shared ways of getting a discord bundle: the explorer server, scraping
//! discord directly, and an on-disk cache in front of both.

pub mod cache;
pub mod explorer;
pub mod scrape;
