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
		.with(
			tracing_subscriber::fmt::layer()
				.with_ansi_sanitization(false)
				.with_ansi(true),
		)
		.with(filter_layer)
		.init();
}

fn main() {
	install_tracing();
	reporter::install_miette_hook();
	let path = env::var("BOT_CONFIG")
		.unwrap_or_else(|_| ".bot.config.json".to_owned());
	let raw = std::fs::read_to_string(&path)
		.unwrap_or_else(|e| panic!("failed to read config `{path}`: {e}"));
	let config: bot_config::Config = serde_json::from_str(&raw)
		.unwrap_or_else(|e| panic!("failed to parse config `{path}`: {e}"));
	bot::run(config);
}
