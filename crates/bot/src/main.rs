use std::env;

use tracing_subscriber::{
	EnvFilter,
	layer::SubscriberExt as _,
	util::SubscriberInitExt as _,
};

fn install_tracing() {
	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| {
			EnvFilter::builder().parse(if cfg!(debug_assertions) {
				"debug,h2=info,hyper=info,rustls=info,reqwest::retry=debug,reqwest::connect=info"
			} else {
				"info"
			})
		})
		.unwrap();
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(filter_layer)
		.init();
}

fn main() {
	install_tracing();
	let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set");
	bot::run(&token);
}
