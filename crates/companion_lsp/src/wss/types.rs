use serde::{Deserialize, Serialize};

use crate::JValue;

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonNull;

impl<'de> Deserialize<'de> for JsonNull {
	fn deserialize<D>(des: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = JValue::deserialize(des)?;
		if value.is_null() {
			Ok(Self)
		} else {
			Err(serde::de::Error::custom("expected null"))
		}
	}
}

impl Serialize for JsonNull {
	fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<() as Serialize>::serialize(&(), ser)
	}
}

pub mod to_client;

pub mod from_client;

pub use to_client::MsgToClient;

pub use from_client::MsgFromClient;
