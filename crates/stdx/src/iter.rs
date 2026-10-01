pub trait IterExt: Iterator {
    /// returns Some(item) if the iterator contains exactly one item, else None,
    /// ```rs
    /// # use stdx::iter::IterExt;
    /// assert_eq!(vec![0].into_iter().expect_one(), Some(0));
    /// assert_eq!(vec![].into_iter().expect_one(), None);
    /// assert_eq!(vec![0, 1].into_iter().expect_one(), None);
    /// ```
	fn expect_one(mut self) -> Option<Self::Item>
	where
		Self: Sized,
	{
		let a = self.next()?;
		self.next().is_none().then_some(a)
	}
}

impl<I: Iterator + ?Sized> IterExt for I {}
