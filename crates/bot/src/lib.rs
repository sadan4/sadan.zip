mod cmds;
mod fw;

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use derive_more::Deref;
use serenity::{
	Client,
	all::{
		Context,
		EventHandler,
		GatewayIntents,
		Ready,
		ShardId,
		ShardRunnerInfo,
		prelude::TypeMapKey,
	},
	async_trait,
};
use tokio::sync::Mutex;
use tracing::{error, info};

struct Handler;

struct ShardInfo(Arc<Mutex<HashMap<ShardId, ShardRunnerInfo>>>);

#[derive(Deref, Clone)]
struct BotConfig(Arc<bot_config::Config>);

impl Debug for BotConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		<bot_config::Config as Debug>::fmt(&self.0, f)
	}
}

impl TypeMapKey for BotConfig {
	type Value = Self;
}

impl TypeMapKey for ShardInfo {
	type Value = Self;
}

#[async_trait]
impl EventHandler for Handler {
	// Set a handler to be called on the `ready` event. This is called when a shard is booted, and
	// a READY payload is sent by Discord. This payload contains data like the current user's guild
	// Ids, current user data, private channels, and more.
	//
	// In this case, just print what the current user's username is.
	async fn ready(&self, _: Context, ready: Ready) {
		info!("{} is connected!", ready.user.name);
	}
}

#[tokio::main]
pub async fn run(config: bot_config::Config) {
	let intents = GatewayIntents::all();
	let handler = Handler;
	// when set, slash commands register to this guild (instant); otherwise
	// they are built for global registration but not auto-pushed
	let guild = config.home_guild_id;
	let cmds = fw::CommandFramework::new(&cmds::ROOT_CMD);
	cmds.with_prefix(";");
	cmds.with_guild(guild);
	let mut client = Client::builder(&config.token, intents)
		.event_handler(handler)
		.event_handler(cmds.clone())
		.await
		.expect("Error creating client");
	cmds.init_data(client.data.clone());
	let runners = Arc::clone(&client.shard_manager.runners);
	client
		.data
		.write()
		.await
		.insert::<ShardInfo>(ShardInfo(runners));
	client
		.data
		.write()
		.await
		.insert::<BotConfig>(BotConfig(Arc::new(config)));

	if let Err(e) = client.start().await {
		error!("Client error: {:?}", e);
	}
}
mod util;
