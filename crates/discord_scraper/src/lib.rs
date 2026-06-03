pub mod bundle_parser;
mod client;
pub mod html_parser;
pub mod progress;
mod scrape;

pub use client::make_reqwest_client;
pub use progress::{NoProgress, ScrapeProgress};
pub use scrape::{ScrapedModules, scrape_full_bundle, scrape_modules};
