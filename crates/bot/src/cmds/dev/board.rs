use std::{borrow::Cow, mem, time::Duration};

use anyhow::{Context as _, Result, bail};
use macros::command;
use serenity::{
	all::{
		ActionRow,
		ActionRowComponent,
		ButtonKind,
		ButtonStyle,
		Component,
		ComponentInteractionCollector,
		Context,
		CreateActionRow,
		CreateAllowedMentions,
		CreateButton,
		CreateComponent,
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

use crate::fw::{CommandCtx, CommandFramework};

const STYLE_ON: ButtonStyle = ButtonStyle::Success;
const STYLE_OFF: ButtonStyle = ButtonStyle::Secondary;
#[allow(clippy::octal_escapes)]
static BUTTON_IDS: &[&[&str]] = &[
	&["0\00", "0\01", "0\02", "0\03", "0\04"],
	&["1\00", "1\01", "1\02", "1\03", "1\04"],
	&["2\00", "2\01", "2\02", "2\03", "2\04"],
	&["3\00", "3\01", "3\02", "3\03", "3\04"],
	&["4\00", "4\01", "4\02", "4\03", "4\04"],
];

fn mk_btn(r: u8, c: u8, e: &bot_config::EmojiDef) -> CreateButton<'static> {
	let id = BUTTON_IDS[r as usize][c as usize];
	CreateButton::new(Cow::Borrowed(id))
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

fn convert_rows(rows: Vec<ActionRow>) -> Vec<CreateComponent<'static>> {
	rows.into_iter()
		.map(|r| {
			let btns = r
				.components
				.into_iter()
				.map(|c| {
					let ActionRowComponent::Button(btn) = c else {
						panic!("component is not a button. got: {c:?}");
					};
					CreateButton::from(btn)
				})
				.collect::<Vec<_>>();
			CreateComponent::ActionRow(CreateActionRow::Buttons(btns.into()))
		})
		.collect()
}

#[command]
async fn board(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	const TIMEOUT_DUR: Duration = Duration::from_mins(1);

	let mut rows: Vec<CreateComponent> = Vec::with_capacity(5);

	let emoji = &fw.config.assets.emojis.empty;
	for r in 0..5 {
		let mut row = Vec::with_capacity(5);
		for c in 0..5 {
			row.push(mk_btn(r, c, emoji));
		}
		rows.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
			row.into(),
		)));
	}
	let i_msg = match cctx {
		CommandCtx::Prefix { msg } => {
			let cm = CreateMessage::new()
				.components(&*rows)
				.reference_message(MessageReference::from(*msg))
				.allowed_mentions(
					CreateAllowedMentions::new().replied_user(true),
				);

			let res = msg
				.channel_id
				.send_message(&ctx.http, cm)
				.await;
			let res: Result<Message> =
				res.context("Failed to send board message");
			res?
		}
		CommandCtx::Application { interaction } => {
			let cr = CreateInteractionResponse::Message(
				CreateInteractionResponseMessage::new().components(&*rows),
			);
			interaction
				.create_response(&ctx.http, cr)
				.await
				.context("Failed to create board interaction response")?;
			interaction
				.get_response(&ctx.http)
				.await
				.context("Failed to get board interaction response")?
		}
	};

	let mut events = ComponentInteractionCollector::new(ctx)
		.message_id(i_msg.id)
		.stream();

	// use a select instead of ComponentInteractionCollector::timeout
	// because we want to timeout after DUR of no interactions, not after we've been running for DUR
	while let Some(mut i) = select! {
		e = events.next() => e,
		() = sleep(TIMEOUT_DUR) => None,
	} {
		let (r, c) = parse_board_id(&i.data.custom_id)
			.context("Failed to get board row/col")?;
		let mut rows: Vec<ActionRow> = mem::take(&mut i.message.components)
			.into_iter()
			.filter_map(|comp| match comp {
				Component::ActionRow(ar) => Some(ar),
				_ => None,
			})
			.collect();
		let comps: &mut [ActionRowComponent] = &mut rows[r as usize].components;
		flip_btn(&mut comps[c as usize]).context("Failed to flip button")?;
		i.create_response(
			&ctx.http,
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
