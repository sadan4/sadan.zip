mod cmds;
mod fw;

use serenity::{
	Client,
	all::{
		Context,
		EventHandler,
		GatewayIntents,
		Ready,
	},
	async_trait,
};
use tracing::{error, info};

struct Handler;

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
pub async fn run(token: &str) {
	let intents = GatewayIntents::all();
	let handler = Handler;
	let cmds = fw::CommandFramework::new(&cmds::ROOT_CMD).with_prefix(";");
	let mut client = Client::builder(token, intents)
		.event_handler(handler)
		.event_handler(cmds)
		.await
		.expect("Error creating client");
	if let Err(e) = client.start().await {
		error!("Client error: {:?}", e);
	}
}
