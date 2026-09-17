pub trait IterExt: Iterator {
	fn exactly_once<F>(self, mut predicate: F) -> Option<Self::Item>
	where
		F: FnMut(&Self::Item) -> bool,
		Self: Sized
	{
		let mut cur = None;
		for item in self {
			if predicate(&item) {
				if cur.is_some() {
					return None;
				}
				cur = Some(item);
			}
		}
		cur
	}
}

impl <T> IterExt for T where T: Iterator + ?Sized {}
