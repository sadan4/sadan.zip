use serde::{Deserialize, Serialize};
use tracing::debug;

/// a u64
#[derive(Serialize, Deserialize, Debug)]
struct HashRepr {
	hi: u32,
	lo: u32,
}

const SHIFT: u32 = u64::BITS / 2;

impl From<u64> for HashRepr {
	fn from(value: u64) -> Self {
		Self {
			lo: value.truncate(),
			hi: (value >> SHIFT).truncate(),
		}
	}
}

impl From<HashRepr> for u64 {
	fn from(value: HashRepr) -> Self {
		(Self::from(value.hi) << SHIFT) | Self::from(value.lo)
	}
}

#[expect(clippy::trivially_copy_pass_by_ref)]
pub fn serialize<S>(value: &u64, ser: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	HashRepr::from(*value).serialize(ser)
}

pub fn deserialize<'de, D>(de: D) -> Result<u64, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let repr = HashRepr::deserialize(de)?;
	debug!(?repr, "deserialized hash repr");
	Ok(u64::from(repr))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn a(value: u64) {
		let repr = HashRepr::from(value);
		assert_eq!(u64::from(repr), value);
	}

	#[test]
	fn round_trips() {
		for i in 0..64 {
			for j in 0..64 {
				a((1 << i) | (1 << j));
			}
			a(1 << i);
		}
	}
}
