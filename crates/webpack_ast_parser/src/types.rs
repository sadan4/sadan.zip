use derive_more::{Deref, Display, From, Into};
use serde::Serialize;

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	From,
	Into,
	Deref,
	PartialOrd,
	Ord,
	Serialize,
	Display,
)]
// TODO: should this be a non-zero u32
pub struct ModuleId(pub u32);

impl TryFrom<f64> for ModuleId {
	// TODO: is this a good error type
	type Error = ();

	fn try_from(value: f64) -> Result<Self, Self::Error> {
		if value.fract() == 0. && value >= 0. && value <= f64::from(u32::MAX) {
			Ok(Self(value as u32))
		} else {
			Err(())
		}
	}
}
