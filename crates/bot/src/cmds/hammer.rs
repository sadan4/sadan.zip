use std::time::Instant;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use gif::Repeat;
use macros::{SlashArgs, command};
use serenity::all::{
	Context,
	CreateAllowedMentions,
	CreateAttachment,
	CreateInteractionResponseFollowup,
	CreateMessage,
};
use skia_safe::{
	Data,
	ISize,
	Image,
	Paint,
	Rect,
	surfaces,
};
use tracing::debug;

use crate::{
	fw::{CommandCtx, CommandFramework, gif_templates::GifTemplate},
	util::{
		Image as BotImage,
		UserArg,
		avatar::download_avatar,
		skia::capture_gif_frame,
	},
};

#[derive(Parser, SlashArgs)]
struct HammerArgs {
	/// user to hammer
	#[arg()]
	target: UserArg,
}

fn gen_frames(avatar: &BotImage, tmpl: &GifTemplate) -> Result<Vec<Vec<u8>>> {
	let width = tmpl.config.width as i32;
	let height = tmpl.config.height as i32;
	let mut frames = Vec::with_capacity(tmpl.frames.len());
	let mut surface = surfaces::raster_n32_premul(ISize { width, height })
		.context("Failed to create surface")?;
	// SAFETY: avatar_data never escapes this function
	let avatar_data = unsafe { Data::new_bytes(&avatar.bytes) };
	let avatar_img = Image::from_encoded(avatar_data)
		.context("Failed to make avatar image from avatar bytes")?;
	let [inj] = tmpl.config.injection.as_slice() else {
		bail!(
			"hammer should only have one injection, got {}",
			tmpl.config.injection.len()
		);
	};
	#[expect(clippy::cast_precision_loss)]
	let injection_rect = Rect::from_xywh(
		inj.x as f32,
		inj.y as f32,
		inj.width as f32,
		inj.height as f32,
	);
	let injection_paint = Paint::default();
	for frame in &tmpl.frames {
		let c = surface.canvas();
		// SAFETY: frame_data never escapes this loop
		let frame_data = unsafe { Data::new_bytes(frame) };
		let img = Image::from_encoded(frame_data)
			.context("Failed to make image from frame data")?;
		c.draw_image(&img, (0, 0), None);
		c.draw_image_rect(&avatar_img, None, injection_rect, &injection_paint);

		let mut pix_buf = Vec::new();

		capture_gif_frame(&mut surface, &mut pix_buf)
			.context("Failed to capture gif frame")?;
		frames.push(pix_buf);
	}
	Ok(frames)
}

fn gen_hammer(avatar: &BotImage, tmpl: &GifTemplate) -> Result<Vec<u8>> {
	let rasterize_start = Instant::now();
	let gen_frames =
		gen_frames(avatar, tmpl).context("Failed to generate frames")?;
	debug!(
		"rasterized {} frame(s) in {:.2?}",
		gen_frames.len(),
		rasterize_start.elapsed()
	);
	let width = tmpl.config.width as u16;
	let height = tmpl.config.height as u16;

	let encode_start = Instant::now();
	let mut gif = gif::Encoder::new(Vec::new(), width, height, &[])
		.context("Failed to create gif encoder")?;

	for mut frame in gen_frames {
		let mut f = gif::Frame::from_rgba_speed(
			width,
			height,
			&mut frame,
			i32::from(tmpl.config.gif_quality),
		);
		f.delay = (tmpl.config.delay / 10) as u16;
		gif.write_frame(&f)
			.context("Failed to write frame to gif")?;
	}

	gif.set_repeat(Repeat::Infinite)
		.context("Failed to set gif repeat")?;

	let out = gif
		.into_inner()
		.context("Failed to finish writing gif")?;
	debug!("encoded gif in {:.2?}", encode_start.elapsed());
	Ok(out)
}

/// Short description of the command.
#[command]
#[arg_parser = HammerArgs]
#[slash_args]
async fn hammer(
	args: HammerArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let total_start = Instant::now();
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let target = ctx
		.http
		.get_user(*args.target)
		.await
		.context("Failed to get hammer target user")?;
	let pfp = target
		.avatar_url()
		.context("Failed to get user avatar url")?;
	let download_start = Instant::now();
	let avatar = download_avatar(&pfp, fw)
		.await
		.context("Failed to download avatar")?;
	debug!("downloaded avatar in {:.2?}", download_start.elapsed());
	let tmpl = &fw
		.get_gif_templates()
		.await
		.context("Failed to get gif templates")?
		.hammer;
	debug_assert_eq!(
		tmpl.config.injection.len(),
		1,
		"hammer should only have one injection"
	);
	let gen_start = Instant::now();
	let gif = tokio::task::block_in_place(|| gen_hammer(&avatar, tmpl))
		.context("Failed to generate hammer gif")?;
	debug!("generated gif in {:.2?}", gen_start.elapsed());
	let file =
		CreateAttachment::bytes(gif, format!("hammer_{}.gif", target.name));
	debug!("total time {:.2?}", total_start.elapsed());
	match cctx {
		CommandCtx::Prefix { msg } => {
			let cm = CreateMessage::new()
				.add_file(file)
				.reference_message(*msg)
				.allowed_mentions(
					CreateAllowedMentions::new().replied_user(true),
				);
			msg.channel_id
				.send_message(&ctx.http, cm)
				.await
				.context("Failed to reply with hammer gif")?;
		}
		CommandCtx::Application { interaction } => {
			let cm = CreateInteractionResponseFollowup::new().add_file(file);
			interaction
				.create_followup(&ctx.http, cm)
				.await
				.context(
					"Failed to send interaction followup with hammer gif",
				)?;
		}
	}
	Ok(())
}
