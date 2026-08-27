use std::borrow::Cow;

use anyhow::{Context as _, Result};
use arrayvec::ArrayVec;
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::{
	ButtonStyle,
	Context,
	CreateActionRow,
	CreateAllowedMentions,
	CreateButton,
	CreateComponent,
	CreateContainer,
	CreateContainerComponent,
	CreateInteractionResponseFollowup,
	CreateMessage,
	CreateSection,
	CreateSectionAccessory,
	CreateSectionComponent,
	CreateSeparator,
	CreateTextDisplay,
	CreateThumbnail,
	CreateUnfurledMediaItem,
	Mentionable,
	MessageFlags,
};

use crate::{
	fw::CommandCtx,
	util::{FROM_REPLY, UserArg},
};

#[derive(Parser, SlashArgs)]
struct UserInfoArgs {
	/// if true, gets the info of the guild user profile instead of the global user profile
	#[arg(short, long, default_value_t = false)]
	guild: bool,
	/// the user to get the info of
	#[arg(default_value = FROM_REPLY)]
	user: UserArg,
}

/// Get user info.
#[command]
#[arg_parser = UserInfoArgs]
#[slash_args]
async fn user_info(
	args: UserInfoArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let author = match ctx.http.get_user(*args.user).await {
		Ok(u) => u,
		Err(e) => {
			let err_txt = format!("Failed to get user info:\n```\n{e:?}\n```");
			cctx.followup_text(ctx, err_txt)
				.await
				.context("Failed to send err msg")?;
			return Ok(());
		}
	};
	let avatar_url = author.face();
	let banner_url = author.banner_url();
	let top_section_cmpts = &[CreateSectionComponent::TextDisplay(
		CreateTextDisplay::new(format!(
			"# {name}\n{ping} ({username})",
			name = author.display_name(),
			ping = author.mention(),
			username = author.name
		)),
	)];
	let top_section_accessory = CreateSectionAccessory::Thumbnail(
		CreateThumbnail::new(CreateUnfurledMediaItem::new(&avatar_url)),
	);
	let top_section =
		CreateSection::new(top_section_cmpts, top_section_accessory);
	let timestamps =
		CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
			"Account Creation: <t:{}>",
			author.id.created_at().unix_timestamp()
		)));
	let mut button_row_1: ArrayVec<CreateButton, 2> = ArrayVec::new();
	button_row_1.push(
		CreateButton::new_link(&avatar_url)
			.style(ButtonStyle::Secondary)
			.label("Avatar"),
	);
	if let Some(banner_url) = banner_url.as_deref() {
		button_row_1.push(
			CreateButton::new_link(banner_url)
				.style(ButtonStyle::Secondary)
				.label("Display Banner"),
		);
	}
	let link_row_1 = CreateContainerComponent::ActionRow(
		CreateActionRow::Buttons(Cow::Borrowed(&button_row_1)),
	);
	let container_cmpts = &[
		CreateContainerComponent::Section(top_section),
		timestamps,
		CreateContainerComponent::Separator(CreateSeparator::new()),
		link_row_1,
	];
	let cmpts = &[CreateComponent::Container(CreateContainer::new(
		container_cmpts,
	))];
	match cctx {
		CommandCtx::Prefix { msg } => {
			msg.channel_id
				.send_message(
					&ctx.http,
					CreateMessage::new()
						.components(cmpts)
						.flags(MessageFlags::IS_COMPONENTS_V2)
						.allowed_mentions(
							CreateAllowedMentions::new().replied_user(true),
						)
						.reference_message(*msg),
				)
				.await
				.context("Failed to send message")?;
		}
		CommandCtx::Application { interaction } => {
			interaction
				.create_followup(
					&ctx.http,
					CreateInteractionResponseFollowup::new()
						.components(cmpts)
						.flags(MessageFlags::IS_COMPONENTS_V2)
						.allowed_mentions(
							CreateAllowedMentions::new()
								.empty_users()
								.empty_roles(),
						),
				)
				.await
				.context("Failed to send interaction response")?;
		}
	}
	Ok(())
}
