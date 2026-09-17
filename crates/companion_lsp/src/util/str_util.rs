pub fn offset_into(whole: &[u8], sub: &[u8]) -> Option<usize> {
	whole
		.as_ptr_range()
		.contains(&sub.as_ptr())
		.then(|| whole.as_ptr() as usize - sub.as_ptr() as usize)
}
