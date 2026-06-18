use serde::{Deserialize, Deserializer, Serialize, Serializer};

	#[derive(Serialize, Deserialize)]
	struct RegressError<'a> {
		text: &'a str,
	}

	pub fn serialize<S: Serializer>(
		t: &regress::Error,
		s: S,
	) -> Result<S::Ok, S::Error> {
		let own_err = RegressError { text: &t.text };
		own_err.serialize(s)
	}
	pub fn deserialize<'de, D: Deserializer<'de>>(
		d: D,
	) -> Result<regress::Error, D::Error> {
		let own_err = RegressError::deserialize(d)?;
		Ok(regress::Error {
			text: own_err.text.to_string(),
		})
	}
