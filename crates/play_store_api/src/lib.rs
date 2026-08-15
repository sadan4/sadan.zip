pub mod constants;
mod mapping;
pub mod diag;
pub mod model;
mod parse_json;
// endpoints
pub mod app;
pub mod list;
pub mod search;

#[cfg(test)]
pub(crate) fn cc() -> reqwest::Client {
	reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::limited(constants::MAX_REDIRECTS))
		.build()
		.unwrap()
}
