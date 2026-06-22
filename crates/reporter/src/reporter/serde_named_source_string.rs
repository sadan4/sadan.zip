use std::fmt;

use miette::NamedSource;
use serde::{Deserialize, Deserializer, Serializer, de, ser::SerializeStruct};

const STRUCT_NAME: &str = "NamedSource<String>";
const FIELDS: &[&str] = &["source", "name", "language"];

struct FieldVisitor;
struct NamedSourceVisitor;

enum Fields {
	Source,
	Name,
	Language,
}

impl de::Visitor<'_> for FieldVisitor {
	type Value = Fields;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("`source`, `name`, or `language`")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: de::Error,
	{
		match v {
			"source" => Ok(Fields::Source),
			"name" => Ok(Fields::Name),
			"language" => Ok(Fields::Language),
			_ => Err(de::Error::unknown_field(v, FIELDS)),
		}
	}
}

impl<'de> Deserialize<'de> for Fields {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(FieldVisitor)
	}
}

impl<'de> de::Visitor<'de> for NamedSourceVisitor {
	type Value = NamedSource<String>;
	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("struct NamedSource<String>")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: de::SeqAccess<'de>,
	{
		let source = seq
			.next_element()?
			.ok_or_else(|| de::Error::invalid_length(0, &self))?;
		let name: &str = seq
			.next_element()?
			.ok_or_else(|| de::Error::invalid_length(1, &self))?;
		let language: &str = seq
			.next_element()?
			.ok_or_else(|| de::Error::invalid_length(2, &self))?;
		Ok(NamedSource::new(name, source).with_language(language))
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where
		A: de::MapAccess<'de>,
	{
		let mut source: Option<String> = None;
		let mut name: Option<&'de str> = None;
		let mut language: Option<&'de str> = None;
		while let Some(key) = map.next_key()? {
			match key {
				Fields::Source => {
					if source.is_some() {
						return Err(de::Error::duplicate_field("source"));
					}
					source = Some(map.next_value()?);
				}
				Fields::Name => {
					if name.is_some() {
						return Err(de::Error::duplicate_field("name"));
					}
					name = Some(map.next_value()?);
				}
				Fields::Language => {
					if language.is_some() {
						return Err(de::Error::duplicate_field("language"));
					}
					language = Some(map.next_value()?);
				}
			}
		}
		let source =
			source.ok_or_else(|| de::Error::missing_field("source"))?;
		let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
		let language =
			language.ok_or_else(|| de::Error::missing_field("language"))?;
		Ok(NamedSource::new(name, source).with_language(language))
	}
}
pub fn serialize<S: Serializer>(
	t: &NamedSource<String>,
	s: S,
) -> Result<S::Ok, S::Error> {
	let mut state = s.serialize_struct(STRUCT_NAME, 3)?;
	state.serialize_field("source", &t.inner())?;
	state.serialize_field("name", &t.name())?;
	state.serialize_field("language", "JavaScript")?;
	state.end()
}
pub fn deserialize<'de, D: Deserializer<'de>>(
	d: D,
) -> Result<NamedSource<String>, D::Error> {
	d.deserialize_struct(STRUCT_NAME, FIELDS, NamedSourceVisitor)
}
