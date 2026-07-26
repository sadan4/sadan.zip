use std::{mem, time::Duration};

use anyhow::{Context as _, Result, bail};
use macros::command;
use serenity::{
	all::{
		ActionRow,
		ActionRowComponent,
		ButtonKind,
		ButtonStyle,
		Context,
		CreateActionRow,
		CreateAllowedMentions,
		CreateButton,
		CreateInteractionResponse,
		CreateInteractionResponseMessage,
		CreateMessage,
		Message,
		MessageReference,
	},
	futures::StreamExt as _,
};
use tokio::{select, time::sleep};
use tracing::info;

use crate::fw::{Command, CommandCtx, CommandFramework};

const STYLE_ON: ButtonStyle = ButtonStyle::Success;
const STYLE_OFF: ButtonStyle = ButtonStyle::Secondary;

fn mk_btn(r: u8, c: u8, e: &bot_config::EmojiDef) -> CreateButton {
	let id = format!("{r}\0{c}");
	CreateButton::new(id)
		.style(STYLE_OFF)
		.emoji(e.clone())
}

/// A board is is `row\0col`, where row and col are 0-4.
fn parse_board_id(id: &str) -> Result<(u8, u8)> {
	let bts = id.as_bytes();
	let [r @ b'0'..=b'4', b'\0', c @ b'0'..=b'4'] = bts else {
		bail!("invalid board id: {id:?}");
	};
	Ok((r - b'0', c - b'0'))
}

fn flip_btn(c: &mut ActionRowComponent) -> Result<()> {
	let ActionRowComponent::Button(btn) = c else {
		bail!("component is not a button. got: {c:?}");
	};
	let ButtonKind::NonLink { ref mut style, .. } = btn.data else {
		bail!("button is not a custom button. got: {btn:?}");
	};
	*style = if *style == STYLE_OFF {
		STYLE_ON
	} else {
		STYLE_OFF
	};
	Ok(())
}

fn convert_rows(rows: Vec<ActionRow>) -> Vec<CreateActionRow> {
	rows.into_iter()
		.map(|r| {
			CreateActionRow::Buttons(
				r.components
					.into_iter()
					.map(|c| {
						let ActionRowComponent::Button(btn) = c else {
							panic!("component is not a button. got: {c:?}");
						};
						CreateButton::from(btn)
					})
					.collect(),
			)
		})
		.collect()
}

#[command]
async fn board(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	_: &Command,
	fw: &CommandFramework,
) -> Result<()> {
	let mut rows = Vec::with_capacity(5);
	let emoji = &fw.config.emojis.empty;
	for r in 0..5 {
		let mut row = Vec::with_capacity(5);
		for c in 0..5 {
			row.push(mk_btn(r, c, emoji));
		}
		rows.push(CreateActionRow::Buttons(row));
	}
	let i_msg = match cctx {
		CommandCtx::Prefix { msg } => {
			let cm = CreateMessage::new()
				.components(rows)
				.reference_message(MessageReference::from(*msg))
				.allowed_mentions(
					CreateAllowedMentions::new().replied_user(true),
				);

			let res = msg
				.channel_id
				.send_message(ctx, cm)
				.await;
			let res: Result<Message> =
				res.context("Failed to send board message");
			res?
		}
		CommandCtx::Application { interaction } => {
			let cr = CreateInteractionResponse::Message(
				CreateInteractionResponseMessage::new().components(rows),
			);
			interaction
				.create_response(ctx, cr)
				.await
				.context("Failed to create board interaction response")?;
			interaction
				.get_response(ctx)
				.await
				.context("Failed to get board interaction response")?
		}
	};

	let mut events = i_msg
		.await_component_interactions(ctx)
		.stream();

	const TIMEOUT_DUR: Duration = Duration::from_mins(1);

	while let Some(mut i) = select! {
		e = events.next() => e,
		() = sleep(TIMEOUT_DUR) => None,
	} {
		let (r, c) = parse_board_id(&i.data.custom_id)
			.context("Failed to get board row/col")?;
		let mut rows = mem::take(&mut i.message.components);
		flip_btn(&mut rows[r as usize].components[c as usize])
			.context("Failed to flip button")?;
		i.create_response(
			ctx,
			CreateInteractionResponse::UpdateMessage(
				CreateInteractionResponseMessage::new()
					.components(convert_rows(rows)),
			),
		)
		.await
		.context("Failed to update interation response")?;
	}

	info!("Board interaction timed out after {TIMEOUT_DUR:?}");
	Ok(())
}
