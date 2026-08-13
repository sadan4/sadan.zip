use std::env;

fn main() {
	bot::install_tracing();
	reporter::install_miette_hook();
	let path = env::var("BOT_CONFIG")
		.unwrap_or_else(|_| ".bot.config.json".to_owned());
	let raw = std::fs::read_to_string(&path)
		.unwrap_or_else(|e| panic!("failed to read config `{path}`: {e}"));
	let config: bot_config::Config = serde_json::from_str(&raw)
		.unwrap_or_else(|e| panic!("failed to parse config `{path}`: {e}"));
	bot::run(config);
}
