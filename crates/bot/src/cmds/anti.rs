use std::{f32};

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::{
		UserArg,
		avatar::download_avatar,
		Image as BotImage,
		skia::{mk_circle, mk_diagonal_line, mk_x_path},
	},
};
use anyhow::{Context as _, Result, bail};
use clap::{Parser, ValueEnum};
use macros::{SlashArgs, SlashChoices, command};
use serenity::all::{Context, CreateAttachment};
use skia_safe::{
	Canvas,
	ClipOp,
	Color,
	Color4f,
	Data,
	EncodedImageFormat,
	Image,
	Paint,
	Pixmap,
	Point,
	scalar,
	surfaces,
};

#[derive(Parser, SlashArgs, Clone, Copy)]
struct AntiArgs {
	/// The width of the ❌
	///
	/// Defaults to 5/36 of the avatar width (the same width as the ❌ in twemoji)
	#[arg(short, long)]
	width: Option<u8>,
	/// The style of the anti-user image,
	#[arg(short, long, value_enum, default_value_t = AntiStyle::X)]
	style: AntiStyle,
	/// The user to use the avatar of
	#[arg()]
	target: UserArg,
}

/// The default width ratio of ❌ in twemoji
const DEFAULT_WIDTH_RATIO: scalar = 5. / 36.;

type WebpImage = Vec<u8>;

#[derive(ValueEnum, SlashChoices, Clone, Copy, Debug)]
enum AntiStyle {
	/// Draw a red ❌ over the avatar
	X,
	/// Draw a red circle with a line through it (🚫) over the avatar
	NotAllowed,
}

impl AntiStyle {
	#[expect(clippy::cast_precision_loss)]
	fn render(self, canvas: &Canvas, bar_width: f32) {
		let img_info = canvas.image_info();
		let w = img_info.width() as f32;
		let h = img_info.height() as f32;
		let red_paint =
			Paint::new(Color4f::from(Color::RED), &img_info.color_space());
		match self {
			Self::X => {
				let path = mk_x_path(bar_width, (w, h));
				canvas.draw_path(&path, &red_paint);
			}
			Self::NotAllowed => {
				let center = Point::new(w / 2., h / 2.);
				canvas.save();
				let clip_path = mk_circle(center, w / 2. - bar_width);
				canvas.clip_path(&clip_path, ClipOp::Difference, None);
				canvas.draw_circle(center, w / 2., &red_paint);
				canvas.restore();
				// 2 * (radius - (bar_width / 2))
				let line_len = w - bar_width;
				let line = mk_diagonal_line(center, line_len, bar_width / 2.);
				canvas.draw_path(&line, &red_paint);
			}
		}
	}
}

#[expect(clippy::cast_precision_loss)]
fn make_anti(avatar: &BotImage, args: AntiArgs) -> Result<WebpImage> {
	let AntiArgs {
		width: bar_width,
		style,
		target: _,
	} = args;
	// SAFETY: data never escapes this function
	let data = unsafe { Data::new_bytes(&avatar.bytes) };
	let avatar_img = Image::from_encoded(data)
		.context("Failed to make avatar image from avatar bytes")?;
	let info = avatar_img.image_info();
	let height = info.height();
	let width = info.width();
	let widthf32 = width as f32;
	debug_assert_eq!(height, width, "Avatar image is not square");
	let bar_width = bar_width.map_or(widthf32 * DEFAULT_WIDTH_RATIO, f32::from);
	let mut surface = surfaces::raster(info, info.min_row_bytes(), None)
		.context("Failed to make surface")?;
	let c = surface.canvas();
	c.draw_image(&avatar_img, (0, 0), None);
	style.render(c, bar_width);
	let mut buf = vec![0; info.height() as usize * info.min_row_bytes()];
	let mut pixmap = Pixmap::new(info, &mut buf, info.min_row_bytes())
		.context("Failed to build output pixmap")?;
	let did_read = c.read_pixels_to_pixmap(&mut pixmap, (0, 0));
	if !did_read {
		bail!("Failed to read pixels from surface");
	}
	let webp = pixmap
		.encode(EncodedImageFormat::WEBP, None)
		.context("Failed to encode pixmap to webp")?;
	Ok(webp)
}

/// Generate an anit-user image
#[command]
#[arg_parser = AntiArgs]
#[slash_args]
async fn anti(
	args: AntiArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	let user = ctx
		.http
		.get_user(*args.target)
		.await
		.context("Failed to get user")?;
	let avatar_url = user
		.avatar_url()
		.context("User has no avatar")?;
	let avatar = download_avatar(&avatar_url, fw)
		.await
		.context("Failed to download avatar")?;
	let webp = tokio::task::spawn_blocking(move || make_anti(&avatar, args))
		.await
		.context("Join Error")?
		.context("Failed to make anti image")?;
	let filename = format!("anti_{user}.webp", user = user.display_name());
	let file = CreateAttachment::bytes(webp, filename);
	cctx.followup_file(ctx, file)
		.await
		.context("Failed to upload anti-user image")?;
	Ok(())
}
