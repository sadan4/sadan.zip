use core::f32;
use std::time::Instant;

use anyhow::{Context as _, Result, anyhow};
use bytes::Bytes;
use macros::command;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serenity::all::{Context, CreateAttachment};
use skia_safe::{AlphaType, Color4f, Data, Image, Point, Surface, surfaces};
use tokio::task::spawn_blocking;
use tracing::debug;
use webp::{AnimEncoder, AnimFrame, WebPConfig};

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{self, skia::capture_frame},
};

const TRANSPARENT: Color4f = Color4f {
	r: 0.,
	g: 0.,
	b: 0.,
	a: 0.,
};

const NUM_FRAMES: u16 = 120;
const ROTATIONS: [u16; NUM_FRAMES as usize] = const {
	let mut arr = [0; _];
	let mut i = 0;
	while i < NUM_FRAMES {
		arr[i as usize] = i * (360 / NUM_FRAMES);
		i += 1;
	}
	arr
};
const TOTAL_DUR_MS: u16 = 1500;
/// Frame delay in milliseconds; 360 frames * 10ms = 3.6s per rotation.
const FRAME_DELAY_MS: i32 = (TOTAL_DUR_MS / NUM_FRAMES) as i32;
/// WEBP encode quality (0..=100) for the lossy encoder.
const WEBP_QUALITY: f32 = 90.0;
/// WEBP compression effort (0=fast/larger .. 6=slow/smaller). Method 2 is the
/// sweet spot here: ~3x faster than the default 4 for near-identical size.
const WEBP_METHOD: i32 = 2;

/// Renders `img` rotated `deg` degrees about `center` into `buf` as straight
/// (unpremultiplied) RGBA, keeping the full 8-bit alpha channel — no palette,
/// no 1-bit mask, so the anti-aliased edges stay smooth.
fn render_frame_rgba(
	surface: &mut Surface,
	img: &Image,
	center: (f32, f32),
	origin: (f32, f32),
	deg: f32,
	buf: &mut Vec<u8>,
) -> Result<()> {
	let c = surface.canvas();
	c.save();
	c.clear(TRANSPARENT);
	c.rotate(deg, Some(Point::from(center)));
	c.draw_image(img, Point::from(origin), None);
	c.restore();
	capture_frame(surface, buf, AlphaType::Unpremul)
		.context("Failed to capture frame")
}

/// Spins an image 360° into an animated (lossy) WEBP.
///
/// Keeps full 24-bit color and 8-bit alpha — no quantization, no 1-bit
/// transparency — so the rotating anti-aliased edges stay smooth. Frames are
/// rendered in parallel; the webp anim encoder itself is sequential.
pub fn spin_image(image: &util::Image) -> Result<Vec<u8>> {
	let total_start = Instant::now();
	// SAFETY: data does not escape this function
	let data = unsafe { Data::new_bytes(&image.bytes) };
	let img = Image::from_encoded(data).context("Failed to decode image")?;
	let width = img.width() as u16;
	let height = img.height() as u16;
	let center = (f32::from(width) / 2.0, f32::from(height) / 2.0);
	let (width, height) = (u32::from(width), u32::from(height));

	// Render every rotation in parallel as straight RGBA. All frames must stay
	// alive for the encoder (AnimFrame borrows the pixel data), hence the Vec.
	let render_start = Instant::now();
	let frames: Vec<Vec<u8>> = ROTATIONS
		.into_par_iter()
		.map_init(
			|| (surfaces::raster(img.image_info(), None, None), Vec::new()),
			|(surface, buf), deg| -> Result<Vec<u8>> {
				let surface = surface
					.as_mut()
					.context("Failed to create surface")?;
				render_frame_rgba(
					surface,
					&img,
					center,
					(0.0, 0.0),
					f32::from(deg),
					buf,
				)?;
				Ok(std::mem::take(buf))
			},
		)
		.collect::<Result<Vec<_>>>()?;
	debug!(elapsed = ?render_start.elapsed(), "rendered 360 frames");

	let encode_start = Instant::now();
	let mut config =
		WebPConfig::new().map_err(|()| anyhow!("Failed to init WebPConfig"))?;
	config.lossless = 0;
	config.quality = WEBP_QUALITY;
	config.method = WEBP_METHOD;
	config.thread_level = 1;
	let mut encoder = AnimEncoder::new(width, height, &config);
	encoder.set_loop_count(0); // infinite
	for (i, frame) in frames.iter().enumerate() {
		let timestamp = (i as i32 + 1) * FRAME_DELAY_MS;
		encoder
			.add_frame(AnimFrame::from_rgba(frame, width, height, timestamp));
	}
	let out = encoder
		.try_encode()
		.map_err(|e| anyhow!("Failed to encode webp: {e:?}"))?
		.to_vec();
	debug!(elapsed = ?encode_start.elapsed(), "encoded 360 frames");
	debug!(elapsed = ?total_start.elapsed(), bytes = out.len(), "spin_image done");
	Ok(out)
}

pub fn spin_image_no_clip(image: &util::Image) -> Result<Vec<u8>> {
	// SAFETY: data does not escape this function
	let data = unsafe { Data::new_bytes(&image.bytes) };
	let img = Image::from_encoded(data).context("Failed to decode image")?;
	let orig_w = img.width() as f32;
	let orig_h = img.height() as f32;
	// Enlarge the canvas by √2 so a rotated corner never clips.
	let width = (orig_w * f32::consts::SQRT_2).ceil();
	let height = (orig_h * f32::consts::SQRT_2).ceil();
	let center = (width / 2., height / 2.);
	// Offset that centers the original image inside the enlarged canvas.
	let origin = ((width - orig_w) / 2., (height - orig_h) / 2.);
	// The surface must match the encoder dimensions, otherwise the frame
	// buffer is smaller than what libwebp reads (out-of-bounds -> SEGV).
	let info = img
		.image_info()
		.with_dimensions((width as i32, height as i32));

	let frames: Vec<Vec<u8>> = ROTATIONS
		.into_par_iter()
		.map_init(
			|| surfaces::raster(&info, None, None),
			|surface, deg| -> Result<Vec<u8>> {
				let mut buf = Vec::new();
				let surface = surface
					.as_mut()
					.context("Failed to create surface")?;
				render_frame_rgba(
					surface,
					&img,
					center,
					origin,
					f32::from(deg),
					&mut buf,
				)?;
				Ok(buf)
			},
		)
		.collect::<Result<_>>()?;

	let mut config =
		WebPConfig::new().map_err(|()| anyhow!("Failed to init WebPConfig"))?;
	config.lossless = 0;
	config.quality = WEBP_QUALITY;
	config.method = WEBP_METHOD;
	config.thread_level = 1;
	let mut encoder = AnimEncoder::new(width as u32, height as u32, &config);
	encoder.set_loop_count(0); // infinite
	for (i, frame) in frames.iter().enumerate() {
		let timestamp = (i as i32 + 1) * FRAME_DELAY_MS;
		encoder.add_frame(AnimFrame::from_rgba(
			frame,
			width as u32,
			height as u32,
			timestamp,
		));
	}
	let out = encoder
		.try_encode()
		.map_err(|e| anyhow!("Failed to encode webp: {e:?}"))?;
	// TODO: i don't think there's a good reason for WebPMemory to be !send + !sync
	// we could wrap in an send+sync wrapper and convert directly to Bytes
	Ok(out.to_vec())
}

/// Spins an image in a circle, clipping the edges
#[command]
async fn spin_clip(
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
	let blocking_start = Instant::now();
	let i = spawn_blocking(move || spin_image(&i))
		.await?
		.context("Failed to rotate image")?;
	debug!(elapsed = ?blocking_start.elapsed(), "spawn_blocking rotate_image");
	// FIXME: update user entry when animated images are supported
	let file = CreateAttachment::bytes(Bytes::from(i), "spin.webp");
	cctx.followup_file(ctx, file, None)
		.await
		.context("Failed to send webp followup")?;
	Ok(())
}
#[command]
async fn spin(
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
	let blocking_start = Instant::now();
	let i = spawn_blocking(move || spin_image_no_clip(&i))
		.await?
		.context("Failed to rotate image")?;
	debug!(elapsed = ?blocking_start.elapsed(), "spawn_blocking rotate_image");
	// FIXME: update user entry when animated images are supported
	let file = CreateAttachment::bytes(Bytes::from(i), "spin.webp");
	let upload_start = Instant::now();
	cctx.followup_file(ctx, file, None)
		.await
		.context("Failed to send webp followup")?;
	debug!(elapsed = ?upload_start.elapsed(), "sent webp followup");
	Ok(())
}
