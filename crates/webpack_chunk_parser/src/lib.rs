pub mod base;
mod types;

pub(crate) trait Sealed {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
	}
}
