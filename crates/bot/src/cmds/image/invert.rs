use anyhow::{Context as _, Result, bail};
use macros::command;
use serenity::all::Context;
use skia_safe::{
	ColorType,
	Data,
	EncodedImageFormat,
	Image,
	Pixmap,
	image::CachingHint,
};
use tokio::task::spawn_blocking;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{self, ImageFormat},
};

fn invert_image(image: &util::Image) -> Result<util::Image> {
	// SAFETY: data does not escape this function
	let data = unsafe { Data::new_bytes(&image.bytes) };
	let img = Image::from_encoded(data).context("Failed to decode image")?;
	let info = img
		.image_info()
		.with_color_type(ColorType::RGBA8888);
	let mut buf = vec![0u8; info.min_row_bytes() * info.height() as usize];
	let pixels = img.read_pixels(
		&info,
		&mut buf,
		info.min_row_bytes(),
		(0, 0),
		CachingHint::Disallow,
	);
	if !pixels {
		bail!("Failed to read pixels from image");
	}
	debug_assert_eq!(buf.len() % 4, 0);
	let (chunks, _) = buf.as_chunks_mut();
	for [r, g, b, _a] in chunks {
		*r = !*r;
		*g = !*g;
		*b = !*b;
	}
	let pixmap = Pixmap::new(&info, &mut buf, info.min_row_bytes())
		.context("Failed to create pixmap")?;
	let bytes = pixmap
		.encode(EncodedImageFormat::WEBP, None)
		.context("Failed to encode image")?
		.into();
	Ok(util::Image {
		bytes,
		format: ImageFormat::Webp,
	})
}

/// Inverts an image.
#[command]
async fn invert(
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
	let i = spawn_blocking(move || invert_image(&i))
		.await?
		.context("Failed to invert image")?;
	fw.image_cache
		.update_user_entry(user, i.clone());
	cctx.followup_image(&ctx.http, &i)
		.await
		.context("Failed to send image followup")?;
	Ok(())
}
