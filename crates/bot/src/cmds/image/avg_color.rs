use anyhow::{Context as _, Result, bail};
use macros::command;
use serenity::all::{Context, CreateAttachment};
use skia_safe::{
	Color4f,
	ColorType,
	Data,
	Image,
	Paint,
	Rect,
	image::CachingHint,
	surfaces::raster_n32_premul,
};
use tokio::task::spawn_blocking;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{self},
};

fn calc_avg_color(image: &util::Image) -> Result<((u8, u8, u8), util::Image)> {
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
	let mut ar: usize = 0;
	let mut ag: usize = 0;
	let mut ab: usize = 0;
	for [r, g, b, _a] in &mut *chunks {
		ar += *r as usize;
		ag += *g as usize;
		ab += *b as usize;
	}
	let len = chunks.len();
	let avg_color = ((ar / len) as u8, (ag / len) as u8, (ab / len) as u8);
	let mut s =
		raster_n32_premul((100, 100)).context("Failed to create canvas")?;
	let c = s.canvas();
	c.draw_rect(
		Rect::from_iwh(100, 100),
		&Paint::new(
			Color4f {
				r: avg_color.0 as f32 / 255.0,
				g: avg_color.1 as f32 / 255.0,
				b: avg_color.2 as f32 / 255.0,
				a: 1.0,
			},
			None,
		),
	);
	let img = util::Image::take_snapshot(&mut s)
		.context("Failed to take snapshot")?;
	Ok((avg_color, img))
}

/// Calculates the average color of an image.
#[command]
async fn avg_color(
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
	let (avg_color, i) = spawn_blocking(move || calc_avg_color(&i))
		.await?
		.context("Failed to calculate average color")?;
	let file = CreateAttachment::bytes(i.bytes, i.format.generic_file_name());
	let text = format!(
		"Average color: #{:02X}{:02X}{:02X}",
		avg_color.0, avg_color.1, avg_color.2
	);
	cctx.followup_file(&ctx.http, file, Some(text))
		.await
		.context("Failed to send image followup")?;
	Ok(())
}
