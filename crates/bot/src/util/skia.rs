use std::{debug_assert_matches, f32};

use anyhow::{Result, bail};
use skia_safe::{
	AlphaType,
	ColorType,
	ImageInfo,
	Path,
	PathBuilder,
	Point,
	Surface,
	scalar,
};
use tracing::warn;

pub fn mk_circle(center: Point, radius: scalar) -> Path {
	let mut pb = PathBuilder::new();
	pb.add_circle(center, radius, None);
	Path::from(pb)
}

pub fn point_on_circle(center: Point, radius: scalar, angle: scalar) -> Point {
	let mut ret = polar_to_cartesian(radius, angle);
	ret.offset(center);
	ret
}

pub fn polar_to_cartesian(r: scalar, theta: scalar) -> Point {
	debug_assert!(r >= 0., "Radius must be non-negative");
	debug_assert_matches!(
		theta,
		0.0..f32::consts::TAU,
		"Angle must be in [0, 2π)"
	);
	let (sin, cos) = theta.sin_cos();
	Point::new(r * cos, r * sin)
}

/// ```txt
/// ----------------- < ret
/// | < ret        /
/// |            /
/// |          /
/// |        / < hyp√2
/// |      /
/// |    /
/// |  /
/// |/
/// ```
pub fn leg_45(hyp: scalar) -> scalar {
	debug_assert!(hyp >= 0., "Hypotenuse must be non-negative");
	hyp * f32::consts::FRAC_1_SQRT_2
}

pub fn mk_diagonal_line(center: Point, length: scalar, width: scalar) -> Path {
	debug_assert!(length >= 0., "Length must be non-negative");
	debug_assert!(width >= 0., "Width must be non-negative");
	if cfg!(debug_assertions) && (center.x < 0. || center.y < 0.) {
		warn!("Center point is negative: {:?}", center);
	}
	let delta = leg_45(width);
	let ld = leg_45(length);
	let ld2 = ld / 2.;
	let mut pb = PathBuilder::new();
	pb.move_to(center);
	pb.r_move_to((ld2, -ld2));
	pb.r_line_to((delta, delta));
	pb.r_line_to((-ld, ld));
	pb.r_line_to((-delta, -delta));
	pb.r_line_to((-delta, -delta));
	pb.r_line_to((ld, -ld));
	pb.r_line_to((delta, delta));
	Path::from(pb)
}

/// Draws an X over a square of size (w, h) with a given line width. The X is centered in the square.
pub fn mk_x_path(p_width: scalar, (w, h): (scalar, scalar)) -> Path {
	let delta = leg_45(p_width);
	// Not 1/√2
	let center_delta = (p_width / 2.) * f32::consts::SQRT_2;
	let mut pb = PathBuilder::new();
	let (mid_x, mid_y) = (w / 2., h / 2.);
	debug_assert_matches!(
		pb.get_last_pt(),
		None | Some(Point { x: 0., y: 0. })
	);
	pb.line_to((delta, 0.));
	pb.line_to((mid_x, mid_y - center_delta));
	pb.line_to((w - delta, 0.));
	pb.line_to((w, 0.));
	pb.line_to((w, delta));
	pb.line_to((mid_x + center_delta, mid_y));
	pb.line_to((w, h - delta));
	pb.line_to((w, h));
	pb.line_to((w - delta, h));
	pb.line_to((mid_x, mid_y + center_delta));
	pb.line_to((delta, h));
	pb.line_to((0., h));
	pb.line_to((0., h - delta));
	pb.line_to((mid_x - center_delta, mid_y));
	pb.line_to((0., delta));
	pb.line_to((0., 0.));
	Path::from(pb)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[expect(clippy::float_cmp)]
	fn delta_45_communicative() {
		let input = f32::consts::TAU * 2.;
		let output = leg_45(input) / 2.;
		let output2 = leg_45(input / 2.);
		assert_eq!(output, output2);
	}
}

/// Captures a frame as RGBA8888 with the given alpha handling.
///
/// out has it's size adjusted if needed; however, it's not zeroed
pub fn capture_frame(
	s: &mut Surface,
	out: &mut Vec<u8>,
	alpha: AlphaType,
) -> Result<()> {
	let w = s.width();
	let h = s.height();
	let info = ImageInfo::new((w, h), ColorType::RGBA8888, alpha, None);
	let mrb = info.min_row_bytes();
	let needed_len = h as usize * mrb;
	out.reserve_exact(out.len().saturating_sub(needed_len));
	out.resize(needed_len, 0);
	let pixels = s.read_pixels(&info, out, mrb, (0, 0));
	if !pixels {
		bail!("Failed to read pixels from surface");
	}
	Ok(())
}

/// Captures a frame for a gif (premultiplied alpha).
///
/// out has it's size adjusted if needed; however, it's not zeroed
pub fn capture_gif_frame(s: &mut Surface, out: &mut Vec<u8>) -> Result<()> {
	capture_frame(s, out, AlphaType::Premul)
}
