use anyhow::{Context as _, Result, bail};
use arrayvec::ArrayString;
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::{
	all::{
		ButtonStyle,
		ComponentInteractionCollector,
		ComponentInteractionData,
		ComponentInteractionDataKind,
		Context,
		CreateActionRow,
		CreateButton,
		CreateComponent,
		CreateInteractionResponse,
		CreateInteractionResponseMessage,
		CreateSelectMenu,
		CreateSelectMenuKind,
		CreateSelectMenuOption,
	},
	futures::StreamExt as _,
};
use std::{borrow::Cow, fmt::Write as _, time::Duration};
use tokio::{select, time::sleep};

use crate::fw::CommandCtx;

#[derive(Parser, SlashArgs)]
struct ChmodArgs {
	/// initial perms, in octal. Eg: 755, 644, 777, etc.
	#[arg()]
	perms: Option<String>,
}

const EXECUTE_MASK: u32 = 0o111;
const READ_MASK: u32 = 0o444;
const WRITE_MASK: u32 = 0o222;
const PERM_MASK: u32 = EXECUTE_MASK | READ_MASK | WRITE_MASK;
const USER_MASK: u32 = 0o700;
const GROUP_MASK: u32 = 0o070;
const OTHER_MASK: u32 = 0o007;
const STICKY_BIT: u32 = 0o1000;
const SETGID_BIT: u32 = 0o2000;
const SETUID_BIT: u32 = 0o4000;
const MISC_MASK: u32 = STICKY_BIT | SETGID_BIT | SETUID_BIT;
const ALL_BITS_MASK: u32 = MISC_MASK | PERM_MASK;

fn parse_perms(s: &str) -> Result<u32> {
	if s.is_empty() {
		return Ok(0);
	}
	let perms = u32::from_str_radix(s, 8)?;
	if (perms & !ALL_BITS_MASK) != 0 {
		anyhow::bail!("Unknown permission bits: {:#o}", perms & !ALL_BITS_MASK);
	}
	Ok(perms)
}

// 13 is max chars + 1 with leading 0
type OctalString = ArrayString<13>;

fn print_perms(perms: u32) -> OctalString {
	let mut s = OctalString::new();
	write!(s, "{:04o}", perms & ALL_BITS_MASK).unwrap();
	s
}

const DISPLAY_ID: &str = "\x00CHMOD_DISPLAY";
const HEADER_USER_ID: &str = "\x00CHMOD_HEADER_USER";
const HEADER_GROUP_ID: &str = "\x00CHMOD_HEADER_GROUP";
const HEADER_OTHER_ID: &str = "\x00CHMOD_HEADER_OTHER";
const COL_READ_ID: &str = "\x00CHMOD_COL_READ";
const COL_WRITE_ID: &str = "\x00CHMOD_COL_WRITE";
const COL_EXECUTE_ID: &str = "\x00CHMOD_COL_EXECUTE";
const SELECT_MISC_ID: &str = "\x00CHMOD_MISC_SELET";
const USER_READ_ID: &str = "\x00CHMOD_USER_READ";
const USER_WRITE_ID: &str = "\x00CHMOD_USER_WRITE";
const USER_EXECUTE_ID: &str = "\x00CHMOD_USER_EXECUTE";
const GROUP_READ_ID: &str = "\x00CHMOD_GROUP_READ";
const GROUP_WRITE_ID: &str = "\x00CHMOD_GROUP_WRITE";
const GROUP_EXECUTE_ID: &str = "\x00CHMOD_GROUP_EXECUTE";
const OTHER_READ_ID: &str = "\x00CHMOD_OTHER_READ";
const OTHER_WRITE_ID: &str = "\x00CHMOD_OTHER_WRITE";
const OTHER_EXECUTE_ID: &str = "\x00CHMOD_OTHER_EXECUTE";
const STICKY_ID: &str = "\x00CHMOD_STICKY";
const SETUID_ID: &str = "\x00CHMOD_SETUID";
const SETGID_ID: &str = "\x00CHMOD_SETGID";

macro_rules! header_buttons {
	($label:expr) => {
		[
			$label,
			CreateButton::new(HEADER_USER_ID)
				.label("Owner")
				.style(ButtonStyle::Secondary)
				.disabled(true),
			CreateButton::new(HEADER_GROUP_ID)
				.label("Group")
				.style(ButtonStyle::Secondary)
				.disabled(true),
			CreateButton::new(HEADER_OTHER_ID)
				.label("Others")
				.style(ButtonStyle::Secondary)
				.disabled(true),
		]
	};
}

const fn style(set: bool) -> ButtonStyle {
	if set {
		ButtonStyle::Success
	} else {
		ButtonStyle::Danger
	}
}

fn read_row(mut perms: u32) -> [CreateButton<'static>; 4] {
	perms &= READ_MASK;
	let user_read: bool = (perms & USER_MASK) != 0;
	let group_read: bool = (perms & GROUP_MASK) != 0;
	let other_read: bool = (perms & OTHER_MASK) != 0;
	[
		CreateButton::new(COL_READ_ID)
			.label("Read")
			.style(ButtonStyle::Secondary)
			.disabled(true),
		CreateButton::new(USER_READ_ID)
			.label("Owner")
			.style(style(user_read)),
		CreateButton::new(GROUP_READ_ID)
			.label("Group")
			.style(style(group_read)),
		CreateButton::new(OTHER_READ_ID)
			.label("Others")
			.style(style(other_read)),
	]
}
fn write_row(mut perms: u32) -> [CreateButton<'static>; 4] {
	perms &= WRITE_MASK;
	let user_write: bool = (perms & USER_MASK) != 0;
	let group_write: bool = (perms & GROUP_MASK) != 0;
	let other_write: bool = (perms & OTHER_MASK) != 0;
	[
		CreateButton::new(COL_WRITE_ID)
			.label("Write")
			.style(ButtonStyle::Secondary)
			.disabled(true),
		CreateButton::new(USER_WRITE_ID)
			.label("Owner")
			.style(style(user_write)),
		CreateButton::new(GROUP_WRITE_ID)
			.label("Group")
			.style(style(group_write)),
		CreateButton::new(OTHER_WRITE_ID)
			.label("Others")
			.style(style(other_write)),
	]
}

fn exec_row(mut perms: u32) -> [CreateButton<'static>; 4] {
	perms &= EXECUTE_MASK;
	let user_exec: bool = (perms & USER_MASK) != 0;
	let group_exec: bool = (perms & GROUP_MASK) != 0;
	let other_exec: bool = (perms & OTHER_MASK) != 0;
	[
		CreateButton::new(COL_EXECUTE_ID)
			.label("Exec")
			.style(ButtonStyle::Secondary)
			.disabled(true),
		CreateButton::new(USER_EXECUTE_ID)
			.label("Owner")
			.style(style(user_exec)),
		CreateButton::new(GROUP_EXECUTE_ID)
			.label("Group")
			.style(style(group_exec)),
		CreateButton::new(OTHER_EXECUTE_ID)
			.label("Others")
			.style(style(other_exec)),
	]
}

fn misc_bits(mut perms: u32) -> CreateSelectMenu<'static> {
	perms &= MISC_MASK;
	let sticky = (perms & STICKY_BIT) != 0;
	let setgid = (perms & SETGID_BIT) != 0;
	let setuid = (perms & SETUID_BIT) != 0;
	CreateSelectMenu::new(
		SELECT_MISC_ID,
		CreateSelectMenuKind::String {
			options: vec![
				CreateSelectMenuOption::new("Sticky", STICKY_ID)
					.default_selection(sticky),
				CreateSelectMenuOption::new("SetGID", SETGID_ID)
					.default_selection(setgid),
				CreateSelectMenuOption::new("SetUID", SETUID_ID)
					.default_selection(setuid),
			]
			.into(),
		},
	)
	.min_values(0)
	.max_values(3)
}

fn build_components(perms: u32) -> [CreateComponent<'static>; 5] {
	let label = print_perms(perms);
	let header_buttons = header_buttons![
		CreateButton::new(DISPLAY_ID)
			.label(label.as_str().to_owned())
			.style(ButtonStyle::Primary)
			.disabled(true)
	];
	[
		CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(
			Vec::from(header_buttons),
		))),
		CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(
			Vec::from(read_row(perms)),
		))),
		CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(
			Vec::from(write_row(perms)),
		))),
		CreateComponent::ActionRow(CreateActionRow::Buttons(Cow::Owned(
			Vec::from(exec_row(perms)),
		))),
		CreateComponent::ActionRow(CreateActionRow::SelectMenu(misc_bits(
			perms,
		))),
	]
}

fn next_perms(mut perms: u32, i: &ComponentInteractionData) -> Result<u32> {
	let id = i.custom_id.as_str();
	match id {
		USER_READ_ID => perms ^= USER_MASK & READ_MASK,
		USER_WRITE_ID => perms ^= USER_MASK & WRITE_MASK,
		USER_EXECUTE_ID => perms ^= USER_MASK & EXECUTE_MASK,
		GROUP_READ_ID => perms ^= GROUP_MASK & READ_MASK,
		GROUP_WRITE_ID => perms ^= GROUP_MASK & WRITE_MASK,
		GROUP_EXECUTE_ID => perms ^= GROUP_MASK & EXECUTE_MASK,
		OTHER_READ_ID => perms ^= OTHER_MASK & READ_MASK,
		OTHER_WRITE_ID => perms ^= OTHER_MASK & WRITE_MASK,
		OTHER_EXECUTE_ID => perms ^= OTHER_MASK & EXECUTE_MASK,
		SELECT_MISC_ID => {
			// unset all misc bits
			perms &= !MISC_MASK;
			let data = match &i.kind {
				ComponentInteractionDataKind::StringSelect { values } => {
					values.as_slice()
				}
				other => bail!(
					"Expected string select interaction data, got {other:#?}"
				),
			};
			for value in data {
				match value.as_str() {
					STICKY_ID => perms |= STICKY_BIT,
					SETGID_ID => perms |= SETGID_BIT,
					SETUID_ID => perms |= SETUID_BIT,
					other => bail!("Unknown misc select value: {other}"),
				}
			}
		}
		_ => bail!("Unknown button ID: {id}"),
	}
	Ok(perms)
}

/// Interactive chmod viewer
#[command]
#[arg_parser = ChmodArgs]
#[slash_args]
async fn chmod(
	args: ChmodArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	let mut perms = parse_perms(&args.perms.unwrap_or_default())
		.context("Failed to parse perms")?;
	let cmpts = build_components(perms);
	let sent_id = cctx
		.reply_components(ctx, Vec::from(cmpts))
		.await?;
	let mut events = ComponentInteractionCollector::new(ctx)
		.message_id(sent_id)
		.stream();
	const TIMEOUT_DUR: Duration = Duration::from_mins(5);
	while let Some(e) = select! {
		e = events.next() => e,
		() = sleep(TIMEOUT_DUR) => None,
	} {
		// TODO: implement forking
		perms = next_perms(perms, &e.data).with_context(|| {
			format!("Failed to compute next perms. {:#?}", e.data)
		})?;
		let cmpts = build_components(perms);
		e.create_response(
			&ctx.http,
			CreateInteractionResponse::UpdateMessage(
				CreateInteractionResponseMessage::new()
					.components(Vec::from(cmpts)),
			),
		)
		.await
		.context("Failed to update chmod message")?;
	}
	Ok(())
}
