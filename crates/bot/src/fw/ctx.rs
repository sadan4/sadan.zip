//! The unified invocation context handed to command handlers and checks.
//!
//! A command can be invoked two ways: as prefix text (an ordinary [`Message`])
//! or as a Discord application (slash) command (a [`CommandInteraction`]).
//! [`CommandCtx`] abstracts over the two so a single handler serves both, while
//! [`CommandCtx::as_message`] / [`CommandCtx::as_interaction`] provide escape
//! hatches for handlers that are deliberately restricted to one mode.

use anyhow::{Context as _, Result};
use serenity::all::{
	CacheHttp,
	CommandInteraction,
	CreateAllowedMentions,
	CreateAttachment,
	CreateEmbed,
	CreateInteractionResponse,
	CreateInteractionResponseFollowup,
	CreateInteractionResponseMessage,
	CreateMessage,
	EditInteractionResponse,
	EditMessage,
	GenericChannelId,
	Message,
	User,
};

/// How a command was invoked.
pub enum CommandCtx<'a> {
	/// Invoked as prefix text; wraps the triggering message.
	Prefix { msg: &'a Message },
	/// Invoked as a Discord application (slash) command.
	Application { interaction: &'a CommandInteraction },
}

/// A handle to the reply a command produced, so it can be edited afterwards
/// (e.g. to fill in a value that was not known when the reply was first sent).
pub enum ReplyHandle<'a> {
	/// The bot's own reply message, for a prefix invocation. Boxed because a
	/// [`Message`] is far larger than the interaction variant's reference.
	Prefix { msg: Box<Message> },
	/// The initial interaction response, for a slash invocation.
	Application { interaction: &'a CommandInteraction },
}

impl<'a> CommandCtx<'a> {
	pub async fn defer(&self, c: impl CacheHttp) -> Result<()> {
		match self {
			CommandCtx::Prefix { msg } => {
				msg.react(c.http(), '💭')
					.await
					.context("Failed to react to message")?;
			}
			CommandCtx::Application { interaction } => {
				interaction
					.defer(c.http())
					.await
					.context("Failed to defer interaction")?;
			}
		}
		Ok(())
	}

	/// The user who invoked the command.
	pub const fn author(&self) -> &User {
		match self {
			Self::Prefix { msg } => &msg.author,
			Self::Application { interaction } => &interaction.user,
		}
	}

	/// The channel the command was invoked in.
	pub const fn channel_id(&self) -> GenericChannelId {
		match self {
			Self::Prefix { msg } => msg.channel_id,
			Self::Application { interaction } => interaction.channel_id,
		}
	}

	/// The triggering message, if this was a prefix invocation.
	pub const fn as_message(&self) -> Option<&Message> {
		match self {
			Self::Prefix { msg } => Some(msg),
			Self::Application { .. } => None,
		}
	}

	/// The triggering interaction, if this was a slash invocation.
	pub const fn as_interaction(&self) -> Option<&CommandInteraction> {
		match self {
			Self::Application { interaction } => Some(interaction),
			Self::Prefix { .. } => None,
		}
	}

	/// Send the command's initial reply, returning a handle that can edit it.
	///
	/// For a slash invocation this is the interaction's initial response, so it
	/// must be sent within Discord's 3-second window; long-running commands
	/// should be reworked to defer first (not yet supported).
	pub async fn reply(
		&self,
		c: impl CacheHttp,
		content: impl Into<String>,
	) -> Result<ReplyHandle<'a>> {
		let content: String = content.into();
		match self {
			Self::Prefix { msg } => {
				let sent = msg
					.reply_ping(c.http(), content)
					.await?;
				Ok(ReplyHandle::Prefix {
					msg: Box::new(sent),
				})
			}
			Self::Application { interaction } => {
				interaction
					.create_response(
						c.http(),
						CreateInteractionResponse::Message(
							CreateInteractionResponseMessage::new()
								.content(content),
						),
					)
					.await?;
				Ok(ReplyHandle::Application { interaction })
			}
		}
	}

	/// Send the command's initial reply as one or more embeds.
	pub async fn reply_embed<'b>(
		&self,
		c: impl CacheHttp,
		embeds: impl IntoIterator<Item = CreateEmbed<'b>>,
	) -> Result<ReplyHandle<'a>> {
		let embeds: Vec<CreateEmbed> = embeds.into_iter().collect();
		match self {
			CommandCtx::Prefix { msg } => {
				let cm = CreateMessage::new()
					.embeds(embeds)
					.reference_message(*msg)
					.allowed_mentions(
						CreateAllowedMentions::new().replied_user(true),
					);
				let sent = msg
					.channel_id
					.send_message(c.http(), cm)
					.await?;
				Ok(ReplyHandle::Prefix {
					msg: Box::new(sent),
				})
			}
			CommandCtx::Application { interaction } => {
				let cm = CreateInteractionResponseMessage::new().embeds(embeds);
				interaction
					.create_response(
						c.http(),
						CreateInteractionResponse::Message(cm),
					)
					.await?;
				Ok(ReplyHandle::Application { interaction })
			}
		}
	}

	pub async fn followup_text(
		&self,
		c: impl CacheHttp,
		txt: impl Into<String>,
	) -> Result<ReplyHandle<'a>> {
		let txt: String = txt.into();
		match self {
			CommandCtx::Prefix { msg } => {
				let sent = msg.reply_ping(c.http(), txt).await?;
				Ok(ReplyHandle::Prefix {
					msg: Box::new(sent),
				})
			}
			CommandCtx::Application { interaction } => {
				interaction
					.create_followup(
						c.http(),
						CreateInteractionResponseFollowup::new().content(txt),
					)
					.await?;
				Ok(ReplyHandle::Application { interaction })
			}
		}
	}

	/// Send embeds as the reply to a command that has already [`defer`]red,
	/// one embed per message.
	///
	/// Discord caps the combined size of all embeds in a single message at
	/// 6000 chars, so each embed is sent in its own message. After [`defer`]
	/// the interaction is already acknowledged: the first embed edits the
	/// deferred response and the rest are sent as followups.
	///
	/// [`defer`]: CommandCtx::defer
	pub async fn followup_embed<'b>(
		&self,
		c: impl CacheHttp,
		embeds: impl IntoIterator<Item = CreateEmbed<'b>>,
	) -> Result<ReplyHandle<'a>> {
		let mut embeds = embeds.into_iter();
		let Some(first) = embeds.next() else {
			return Ok(match self {
				Self::Prefix { .. } => unreachable!(),
				Self::Application { interaction } => {
					ReplyHandle::Application { interaction }
				}
			});
		};
		match self {
			CommandCtx::Prefix { msg } => {
				let mk = |e: CreateEmbed<'b>| -> CreateMessage<'b> {
					CreateMessage::new()
						.embed(e)
						.reference_message(*msg)
						.allowed_mentions(
							CreateAllowedMentions::new().replied_user(true),
						)
				};
				let sent = msg
					.channel_id
					.send_message(c.http(), mk(first))
					.await?;
				for e in embeds {
					msg.channel_id
						.send_message(c.http(), mk(e))
						.await?;
				}
				Ok(ReplyHandle::Prefix {
					msg: Box::new(sent),
				})
			}
			CommandCtx::Application { interaction } => {
				interaction
					.edit_response(
						c.http(),
						EditInteractionResponse::new().embed(first),
					)
					.await?;
				for e in embeds {
					interaction
						.create_followup(
							c.http(),
							CreateInteractionResponseFollowup::new().embed(e),
						)
						.await?;
				}
				Ok(ReplyHandle::Application { interaction })
			}
		}
	}

	pub async fn reply_file<'b>(
		&self,
		c: impl CacheHttp,
		file: CreateAttachment<'b>,
		txt: impl Into<Option<String>>,
	) -> Result<ReplyHandle<'a>> {
		match self {
			CommandCtx::Prefix { msg } => {
				let mut cm = CreateMessage::new()
					.add_file(file)
					.reference_message(*msg)
					.allowed_mentions(
						CreateAllowedMentions::new().replied_user(true),
					);
				if let Some(txt) = txt.into() {
					cm = cm.content(txt);
				}
				let msg = msg
					.channel_id
					.send_message(c.http(), cm)
					.await?;
				Ok(ReplyHandle::Prefix { msg: Box::new(msg) })
			}
			CommandCtx::Application { interaction } => {
				let mut cm =
					CreateInteractionResponseMessage::new().add_file(file);
				if let Some(txt) = txt.into() {
					cm = cm.content(txt);
				}
				interaction
					.create_response(
						c.http(),
						CreateInteractionResponse::Message(cm),
					)
					.await?;
				Ok(ReplyHandle::Application { interaction })
			}
		}
	}
}

impl ReplyHandle<'_> {
	/// Replace the content of the reply produced by [`CommandCtx::reply`].
	pub async fn edit(
		&mut self,
		c: impl CacheHttp,
		content: impl Into<String>,
	) -> Result<()> {
		let content: String = content.into();
		match self {
			Self::Prefix { msg } => {
				msg.edit(c, EditMessage::new().content(content))
					.await
					.context("failed to edit reply message")?;
			}
			Self::Application { interaction } => {
				interaction
					.edit_response(
						c.http(),
						EditInteractionResponse::new().content(content),
					)
					.await
					.context("failed to edit interaction response")?;
			}
		}
		Ok(())
	}
}
