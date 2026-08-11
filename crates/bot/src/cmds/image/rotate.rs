use anyhow::{Context as _, Result};
use clap::Parser;
use macros::{SlashArgs, command};
use serenity::all::Context;
use skia_safe::{Data, Image, Point, surfaces};
use tokio::task::spawn_blocking;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util,
};

#[derive(Parser, SlashArgs)]
struct RotateArgs {
	/// The number of degrees to rotate the image. Positive values rotate clockwise, negative values rotate counter-clockwise.
	#[arg()]
	deg: i16,
}

fn rotate_image(image: &util::Image, deg: i16) -> Result<util::Image> {
	// SAFETY: data does not escape this function
	let data = unsafe { Data::new_bytes(&image.bytes) };
	let img = Image::from_encoded(data).context("Failed to decode image")?;
	let mut surface = surfaces::raster(img.image_info(), None, None)
		.context("Failed to create surface")?;
	let c = surface.canvas();
	#[expect(clippy::cast_precision_loss)]
	let center = (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
	c.rotate(f32::from(deg), Some(Point::from(center)));
	c.draw_image(img, (0, 0), None);
	util::Image::take_snapshot(&mut surface)
}

/// Rotates an image by the specified number of degrees.
#[command]
#[arg_parser = RotateArgs]
#[slash_args]
async fn rotate(
	args: RotateArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(&ctx.http)
		.await
		.context("Failed to defer command")?;
	let user = cctx.author().id;
	let si = fw
		.image_cache
		.get_user_entry(user)
		.context("No image selected")?;
	let i = si.wait().await.clone();
	let i = spawn_blocking(move || rotate_image(&i, args.deg))
		.await?
		.context("Failed to rotate image")?;
	fw.image_cache
		.update_user_entry(user, i.clone());
	cctx.followup_image(&ctx.http, &i)
		.await
		.context("Failed to send image followup")?;
	Ok(())
}
