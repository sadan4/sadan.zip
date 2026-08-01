use std::time::Instant;

use serenity::{
	all::{
		Context,
		Event,
		EventHandler,
		FullEvent,
		Interaction,
		Message,
		Ready,
	},
	async_trait,
};
use tracing::error;

use crate::util::{MESSAGE_RECEIVE_TIME, REFERENCED_USER, get_ref_user};

#[allow(clippy::multiple_inherent_impl)]
impl super::CommandFramework {
	async fn message(&self, ctx: &Context, msg: &Message) -> () {
		let handler_timestamp = Instant::now();
		let ref_user = get_ref_user(msg);
		let fut = self.execute_command(ctx, msg);
		REFERENCED_USER
			.scope(ref_user, MESSAGE_RECEIVE_TIME.scope(handler_timestamp, fut))
			.await;
	}

	async fn ready(&self, ctx: &Context, _ready: &Ready) -> () {
		if let Err(e) = self.register_slash_commands(ctx).await {
			error!("Failed to register slash commands: {:?}", e);
		}
		if let Err(e) = self
			.preload_eager_commands(self.root_cmd)
			.await
		{
			error!("Failed to preload eager commands: {:?}", e);
		}
	}

	async fn interaction_create(
		&self,
		ctx: &Context,
		interaction: &Interaction,
	) -> () {
		let Interaction::Command(command) = interaction else {
			return;
		};
		let handler_timestamp = Instant::now();
		let fut = self.handle_interaction(ctx, command);
		// a slash command has no referenced message, so `FROM_REPLY`-style
		// defaults have nothing to resolve against
		REFERENCED_USER
			.scope(None, MESSAGE_RECEIVE_TIME.scope(handler_timestamp, fut))
			.await;
	}
}

#[async_trait]
impl EventHandler for super::CommandFramework {
	fn filter_event(
		&self,
		_context: &serenity::prelude::Context,
		event: Box<Event>,
	) -> Option<Box<Event>> {
		#[allow(clippy::match_like_matches_macro)]
		let should_handle = match event.as_ref() {
			Event::MessageCreate(_)
			| Event::Ready(_)
			| Event::InteractionCreate(_) => true,
			_ => false,
		};
		if should_handle { Some(event) } else { None }
	}
	async fn dispatch(&self, ctx: &Context, e: &FullEvent) {
		match e {
			FullEvent::Message { new_message } => {
				self.message(ctx, new_message).await;
			}
			FullEvent::Ready { data_about_bot } => {
				self.ready(ctx, data_about_bot).await;
			}
			FullEvent::InteractionCreate { interaction } => {
				self.interaction_create(ctx, interaction)
					.await;
			}
			// `filter_event` gates the raw gateway events we handle, but
			// serenity still emits synthetic `FullEvent`s (e.g. `CacheReady`)
			// that reach `dispatch`; ignore anything we don't explicitly handle.
			_ => {}
		}
	}
}
