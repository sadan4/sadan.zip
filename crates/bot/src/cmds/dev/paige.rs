use std::borrow::Cow;

use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::{Context, CreateContainerComponent, CreateTextDisplay};

use crate::fw::{CommandCtx, Paigeinator, PaigeinatorFlags};

#[derive(Parser, SlashArgs)]
#[allow(clippy::struct_excessive_bools)]
struct PaigeParser {
	/// Number of pages to generate.
	#[arg(default_value_t = 5)]
	pages: u8,
	/// [`PaigeinatorFlags::FORK_ON_OTHER_INTERACTION`]: fork to an ephemeral message when a
	/// non-creator interacts.
	#[arg(long, default_value_t = true)]
	fork: bool,
	/// [`PaigeinatorFlags::LOOP`]: wrap from the last page back to the first (and vice versa).
	#[arg(long = "wrap", default_value_t = true)]
	wrap: bool,
	/// [`PaigeinatorFlags::HIDE_PAGE_NUMBERS`]: hide the page counter button.
	#[arg(long, default_value_t = false)]
	hide_page_numbers: bool,
	/// [`PaigeinatorFlags::PING_ON_REPLY`]: ping the user when replying (prefix/message source only).
	#[arg(long, default_value_t = false)]
	ping_on_reply: bool,
	/// [`PaigeinatorFlags::WAS_DEFERRED`]: defer first and follow up instead of responding directly
	/// (application/slash source only).
	#[arg(long = "defer", default_value_t = true)]
	defer: bool,
	/// Let anyone interact (no creator set); otherwise only the invoker can.
	#[arg(long, default_value_t = false)]
	everyone: bool,
}

/// Spin up a [`Paigeinator`] with `pages` throwaway text pages, to test
/// its paging/interaction handling. Each flag is toggleable via an option.
#[command]
#[arg_parser = PaigeParser]
#[slash_args]
async fn paige(
	args: PaigeParser,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
) -> Result<()> {
	let pages = usize::from(args.pages.max(1));

	let mut flags = PaigeinatorFlags::NONE;
	flags.set(PaigeinatorFlags::FORK_ON_OTHER_INTERACTION, args.fork);
	flags.set(PaigeinatorFlags::LOOP, args.wrap);
	flags.set(PaigeinatorFlags::HIDE_PAGE_NUMBERS, args.hide_page_numbers);
	flags.set(PaigeinatorFlags::PING_ON_REPLY, args.ping_on_reply);
	flags.set(PaigeinatorFlags::WAS_DEFERRED, args.defer);

	// the paigeinator follows up when WAS_DEFERRED is set, so the interaction
	// must already be deferred in that case.
	if args.defer {
		cctx.defer(ctx)
			.await
			.context("Failed to defer interaction")?;
	}

	// new() seeds PaigeinatorFlags::DEFAULT, so clear everything not in `flags`
	// to make the toggles authoritative.
	let mut p = Paigeinator::new()
		.with_flags(flags)
		.without_flags(!flags);
	if !args.everyone {
		p = p.with_creator(cctx.author().id);
	}

	for n in 0..pages {
		let content = format!("# Page {}/{pages}\nthis is a test page.", n + 1);
		let page: Vec<CreateContainerComponent> =
			vec![CreateContainerComponent::TextDisplay(
				CreateTextDisplay::new(content),
			)];
		p = p.add_page(Cow::Owned(page));
	}

	p.run(ctx, cctx)
		.await
		.context("Paigeinator run failed")?;
	Ok(())
}
