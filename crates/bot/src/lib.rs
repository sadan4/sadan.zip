mod cmds;
mod fw;
mod util;

use std::{fmt::Debug, sync::Arc};

use derive_more::Deref;
use serenity::{Client, all::GatewayIntents};
use tracing::error;
use typesize::{TypeSize, derive::TypeSize};

fn size_of_arc<T: TypeSize>(e: &Arc<T>) -> usize {
	std::mem::size_of::<Arc<()>>() + e.get_size()
}

#[derive(Deref, Default, Clone, TypeSize)]
pub struct BotConfig(#[typesize(with = size_of_arc)] Arc<bot_config::Config>);

impl Debug for BotConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		<bot_config::Config as Debug>::fmt(&self.0, f)
	}
}

const USER_AGENT: &str =
	concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main]
pub async fn run(config: bot_config::Config) {
	let intents = GatewayIntents::all();
	// when set, slash commands register to this guild (instant); otherwise
	// they are built for global registration but not auto-pushed
	let config = BotConfig(Arc::new(config));
	let guild = config.home_guild_id;
	let cmds = fw::CommandFramework::new(&cmds::ROOT_CMD, config.clone())
		.expect("Failed to make command framework");
	cmds.with_prefix(";").await;
	cmds.with_guild(guild);
	let mut client = Client::builder(
		config
			.token
			.parse()
			.expect("Invalid Token"),
		intents,
	)
	.event_handler(cmds.handler())
	.await
	.expect("Error creating client");

	if let Err(e) = client.start().await {
		error!("Client error: {:?}", e);
	}
}
