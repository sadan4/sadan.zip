use anyhow::{Context as _, Result, bail};
use macros::command;
use serenity::all::Context;
use skia_safe::{Data, Image, Point, surfaces};
use tokio::task::spawn_blocking;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{self, skia::mk_circle},
};

/// Clips image to a circle.
///
/// The returned image is always in [`ImageFormat::Webp`](util::ImageFormat::Webp).
fn clip_image(image: &util::Image) -> Result<util::Image> {
	// SAFETY: data does not escape this function
	let data = unsafe { Data::new_bytes(&image.bytes) };
	let img = Image::from_encoded(&data).context("Failed to decode image")?;
	let width = img.width();
	let height = img.height();
	if width != height {
		bail!("Image is not square ({width}x{height})");
	}
	let radius = width / 2;
	let center = (radius, radius);
	let info = img.image_info();
	let mut surface = surfaces::raster(info, None, None)
		.context("Failed to create surface")?;
	let c = surface.canvas();
	#[expect(clippy::cast_precision_loss)]
	let path = mk_circle(Point::from(center), radius as f32);
	c.clip_path(&path, None, None);
	c.draw_image(&img, (0, 0), None);
	util::Image::take_snapshot(&mut surface)
}

/// Clips the selected image to a circle
#[command]
async fn clip_to_circle(
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
		.context("Failed to get user image")?;
	let i = si.wait().await.clone();
	let i = spawn_blocking(move || clip_image(&i))
		.await?
		.context("Failed to clip image")?;
	fw.image_cache
		.update_user_entry(user, i.clone());
	cctx.followup_image(&ctx.http, &i)
		.await
		.context("Failed to send image followup")?;
	Ok(())
}
