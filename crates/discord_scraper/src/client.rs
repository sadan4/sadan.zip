use std::sync::Arc;

use anyhow::Result;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

static USER_AGENT: &str =
	concat![env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")];

pub fn make_reqwest_client() -> Result<Arc<ClientWithMiddleware>> {
	let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
	let retry_middleware =
		RetryTransientMiddleware::new_with_policy(retry_policy);
	let client = reqwest::Client::builder()
		.user_agent(USER_AGENT)
		.build()?;
	let client = ClientBuilder::new(client)
		.with(retry_middleware)
		.build();
	Ok(Arc::new(client))
}
