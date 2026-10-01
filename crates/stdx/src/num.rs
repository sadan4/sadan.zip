/// Convert `v` to an `i32` if it can be converted losslessly, otherwise return `None`.
/// ```ignore
/// assert_eq!(f64_to_i32(42.0), Some(42));
/// assert_eq!(f64_to_i32(42.5), None);
/// assert_eq!(f64_to_i32(f64::INFINITY), None);
/// ```
pub fn f64_to_i32(v: f64) -> Option<i32> {
	if !v.is_finite() {
		return None;
	}
	if v.fract() != 0. {
		return None;
	}
	if v < f64::from(i32::MIN) || v > f64::from(i32::MAX) {
		return None;
	}
	Some(v as i32)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_f64_to_i32() {
		assert_eq!(f64_to_i32(42.0), Some(42));
		assert_eq!(f64_to_i32(42.5), None);
		assert_eq!(f64_to_i32(f64::INFINITY), None);
		assert_eq!(f64_to_i32(f64::NAN), None);
		assert_eq!(f64_to_i32(f64::from(i32::MAX)), Some(i32::MAX));
		assert_eq!(f64_to_i32(f64::from(i32::MIN)), Some(i32::MIN));
	}
}
