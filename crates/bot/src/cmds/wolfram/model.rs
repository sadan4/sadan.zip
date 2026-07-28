#![expect(dead_code)]
#![expect(clippy::struct_excessive_bools)]
use std::{
	collections::HashMap,
	fmt::{Debug, Display},
	str::FromStr,
};

use serde::{Deserialize, Deserializer};

fn de_num<'de, T, D>(de: D) -> Result<T, D::Error>
where
	T: FromStr + Deserialize<'de>,
	T::Err: Debug + Display,
	D: Deserializer<'de>,
{
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum NumOrStr<'a, T> {
		Num(T),
		Str(&'a str),
	}
	match NumOrStr::deserialize(de)? {
		NumOrStr::Num(n) => Ok(n),
		NumOrStr::Str(s) => T::from_str(s).map_err(serde::de::Error::custom),
	}
}

fn de_maybe_singleton<'de, T, D>(de: D) -> Result<Vec<T>, D::Error>
where
	T: Deserialize<'de>,
	D: Deserializer<'de>,
{
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum MaybeSingleton<T> {
		Single(T),
		Multiple(Vec<T>),
	}
	match MaybeSingleton::deserialize(de)? {
		MaybeSingleton::Single(s) => Ok(vec![s]),
		MaybeSingleton::Multiple(m) => Ok(m),
	}
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Response {
	#[serde(rename = "queryresult")]
	pub query_result: QueryResult,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct QueryResult {
	pub pods: Vec<Pod>,
	pub success: bool,
	pub error: bool,
	pub numpods: usize,
	pub datatypes: String,
	#[serde(rename = "parsetiming")]
	pub parse_timing: f64,
	#[serde(rename = "parsetimedout")]
	pub parse_timedout: bool,
	pub id: String,
	#[serde(rename = "kernelId")]
	pub kernel_id: String,
	#[serde(rename = "processId")]
	pub process_id: u32,
	pub version: String,
	#[serde(rename = "inputstring")]
	pub input_string: String,
	#[serde(rename = "sbsallowed")]
	pub sbs_allowed: bool,
	#[serde(rename = "parentId")]
	pub parent_id: String,
	#[serde(rename = "requestId")]
	pub request_id: String,
	pub timing: f64,
	#[serde(rename = "timedout")]
	pub timed_out: String,
	#[serde(rename = "timedoutpods")]
	pub timed_out_pods: String,
	#[serde(default)]
	pub sources: Vec<Source>,
	#[serde(default)]
	pub assumptions: Vec<Assumption>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Assumption {
	#[serde(rename = "type")]
	type_: String,
	word: String,
	template: String,
	count: usize,
	values: Vec<HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Source {
	pub url: String,
	pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Image {
	pub alt: String,
	#[serde(default, rename = "colorinvertable")]
	pub color_invertable: bool,
	#[serde(rename = "contenttype")]
	pub content_type: Option<String>,
	#[serde(deserialize_with = "de_num")]
	pub height: u64,
	pub src: String,
	pub themes: Option<String>,
	pub title: String,
	#[serde(rename = "type")]
	pub type_: Option<String>,
	#[serde(deserialize_with = "de_num")]
	pub width: u64,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ExpressionTypes {
	pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Subpod {
	pub img: Image,
	pub plaintext: String,
	pub title: String,
	#[serde(default)]
	pub microsources: Vec<String>,
	#[serde(default, deserialize_with = "de_maybe_singleton")]
	pub infos: Vec<Info>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Link {
	pub url: String,
	pub text: String,
	pub title: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Info {
	pub text: Option<String>,
	#[serde(deserialize_with = "de_maybe_singleton")]
	pub links: Vec<Link>,
	pub img: Option<Image>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Pod {
	pub error: bool,
	#[serde(
		rename = "expressiontypes",
		deserialize_with = "de_maybe_singleton"
	)]
	pub expression_types: Vec<ExpressionTypes>,
	pub id: String,
	#[serde(rename = "numsubpods")]
	pub num_subpods: usize,
	pub position: f64,
	pub scanner: String,
	pub subpods: Vec<Subpod>,
	pub title: String,
	#[serde(default)]
	pub primary: bool,
	pub states: Option<Vec<State>>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StateInner {
	pub name: String,
	pub input: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct StateWrapper {
	pub count: usize,
	pub value: String,
	pub delimiters: String,
	pub states: Vec<StateInner>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields, untagged)]
pub enum State {
	Wrapped(StateWrapper),
	Raw(StateInner),
}

#[cfg(test)]
mod tests {
	use super::*;
	fn de(j: &str) -> Response {
		serde_json::from_str(j).unwrap()
	}
	#[test]
	fn test_de() {
		let j = include_str!("./wolfram.json");
		_ = de(j);
	}
	#[test]
	fn test_de_1() {
		let j = include_str!("./wolfram1.json");
		_ = de(j);
	}
	#[test]
	fn test_de_2() {
		let j = include_str!("./wolfram2.json");
		_ = de(j);
	}
	#[test]
	fn test_de_3() {
		let j = include_str!("./wolfram3.json");
		_ = de(j);
	}
}
