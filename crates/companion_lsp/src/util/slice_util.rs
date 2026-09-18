pub fn offset_into<T>(whole: &[T], sub: &[T]) -> Option<usize> {
	const {
		assert!(
			size_of::<T>() != 0,
			"offset_into is not supported for zero-sized types"
		);
	}
	whole
		.as_ptr_range()
		.contains(&sub.as_ptr())
		.then(|| {
			let sub = sub.as_ptr().addr();
			let whole = whole.as_ptr().addr();
			let delta = sub - whole;
			debug_assert!(delta.is_multiple_of(size_of::<T>()));
			delta / size_of::<T>()
		})
}

#[cfg(test)]
mod tests {
	use super::offset_into;

	#[test]
	fn whole_is_its_own_offset_zero() {
		let buf = [1u8, 2, 3, 4];
		assert_eq!(offset_into(&buf, &buf), Some(0));
	}

	#[test]
	fn subslice_offsets() {
		let buf = [1u8, 2, 3, 4, 5];
		assert_eq!(offset_into(&buf, &buf[..2]), Some(0));
		assert_eq!(offset_into(&buf, &buf[2..4]), Some(2));
		assert_eq!(offset_into(&buf, &buf[4..]), Some(4));
	}

	#[test]
	fn offset_is_in_elements_not_bytes() {
		let buf = [10u32, 20, 30, 40];
		assert_eq!(offset_into(&buf, &buf[3..]), Some(3));

		let buf = [(0u64, 0u64); 4];
		assert_eq!(offset_into(&buf, &buf[1..3]), Some(1));
	}

	#[test]
	fn str_bytes() {
		let s = "hello, world";
		let idx = s.find("world").unwrap();
		let sub = &s[idx..];
		assert_eq!(offset_into(s.as_bytes(), sub.as_bytes()), Some(idx));
	}

	#[test]
	fn multibyte_str_bytes() {
		let s = "héllo → wörld";
		let idx = s.find('→').unwrap();
		let sub = &s[idx..];
		assert_eq!(offset_into(s.as_bytes(), sub.as_bytes()), Some(idx));
	}

	#[test]
	fn empty_subslice_inside() {
		let buf = [1u8, 2, 3];
		assert_eq!(offset_into(&buf, &buf[1..1]), Some(1));
	}

	#[test]
	fn unrelated_allocation_is_none() {
		let a = [1u8, 2, 3];
		let b = [1u8, 2, 3];
		assert_eq!(offset_into(&a, &b), None);
		assert_eq!(offset_into(&a, &b[1..]), None);
	}

	#[test]
	fn sub_before_whole_is_none() {
		let buf = [1u8, 2, 3, 4, 5];
		let whole = &buf[2..];
		assert_eq!(offset_into(whole, &buf[..2]), None);
		assert_eq!(offset_into(whole, &buf[1..]), None);
	}

	#[test]
	fn sub_after_whole_is_none() {
		let buf = [1u8, 2, 3, 4, 5];
		let whole = &buf[..2];
		assert_eq!(offset_into(whole, &buf[3..]), None);
	}

	#[test]
	fn empty_whole_is_none() {
		let buf = [1u8, 2, 3];
		assert_eq!(offset_into(&buf[1..1], &buf[1..1]), None);
		assert_eq!(offset_into(&buf[1..1], &buf[1..]), None);
	}
}
