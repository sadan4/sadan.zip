use std::borrow::Cow;

use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::{
	ButtonStyle,
	Context,
	CreateActionRow,
	CreateButton,
	CreateComponent,
	CreateContainer,
	CreateContainerComponent,
};

use crate::fw::CommandCtx;

const LOST_PARTS: &[&str] = &["┃", "┃╻", "┃┃", "┃_"];

const LOSS_BTN_ID: &[&str] =
	&["\x00LOSS_0", "\x00LOSS_1", "\x00LOSS_2", "\x00LOSS_3"];

macro_rules! loss_btn {
	($num:literal) => {
		CreateButton::new(LOSS_BTN_ID[$num])
			.label(LOST_PARTS[$num])
			.style(ButtonStyle::Secondary)
	};
}

/// Short description of the command.
#[command]
async fn loss(ctx: &Context, cctx: &CommandCtx<'_>) -> Result<()> {
	let row_1 =
		CreateActionRow::Buttons(Cow::Borrowed(&[loss_btn!(0), loss_btn!(1)]));
	let row_2 =
		CreateActionRow::Buttons(Cow::Borrowed(&[loss_btn!(2), loss_btn!(3)]));
	let container_cmpts = &[
		CreateContainerComponent::ActionRow(row_1),
		CreateContainerComponent::ActionRow(row_2),
	];
	let cmpts = &[CreateComponent::Container(CreateContainer::new(
		container_cmpts,
	))];
	cctx.reply_components(ctx, cmpts)
		.await
		.context("Failed to respond to interaction")?;
	Ok(())
}
