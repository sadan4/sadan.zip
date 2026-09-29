use serde::{Deserialize, Serialize};
use typesize::derive::TypeSize;

use crate::SpannedId;

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ExperimentKind {
	#[serde(rename = "user")]
	User(UserExperiment),
	#[serde(rename = "guild")]
	Guild(GuildExperiment),
}

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
pub struct Experiment {
	pub loc: SpannedId,
	pub obj: ExperimentKind,
}

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "camelCase")]
pub struct UserExperiment {
	pub name: String,
	#[typesize(with = size_serde_json_value)]
	pub default_config: serde_json::Value,
	pub label: Option<String>,
	pub variations: Vec<Variation>,
	pub treatments: Vec<Treatment>,
	/// TODO: was seen in 2025-09_hotwheels_nvidia_boost
	pub common_trigger_point: Option<()>,
}

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
pub struct Variation {
	pub key: String,
	#[typesize(with = size_serde_json_value)]
	pub config: serde_json::Value,
}

fn size_serde_json_value(value: &serde_json::Value) -> usize {
	use serde_json::Value;
	size_of::<Value>()
		+ match value {
			Value::Null | Value::Bool(_) | Value::Number(_) => 0,
			Value::String(s) => s.capacity(),
			Value::Array(values) => {
				let len = values.len();
				let cap = values.capacity();
				let values_size: usize = values
					.iter()
					.map(size_serde_json_value)
					.sum();
				(cap - len) * size_of::<Value>() + values_size
			}
			Value::Object(map) => {
				// map doesn't provide capacity, so we just have to iterate over the values
				map.iter()
					.map(|(k, v)| k.capacity() + size_serde_json_value(v))
					.sum()
			}
		}
}

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
pub struct GuildExperiment {
	pub id: String,
	pub label: String,
	#[typesize(with = size_serde_json_value)]
	pub default_config: serde_json::Value,
	pub treatments: Vec<Treatment>,
}

#[derive(
	Serialize, Deserialize, Debug, TypeSize, Clone, PartialEq, Eq, Hash,
)]
pub struct Treatment {
	pub id: i32,
	pub label: String,
	#[typesize(with = size_serde_json_value)]
	pub config: serde_json::Value,
}
