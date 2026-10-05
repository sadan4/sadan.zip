use daft::Diffable;
use serde::{Deserialize, Serialize};
use typesize::derive::TypeSize;

use crate::{SpannedId, size_serde_json_value};

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
#[serde(rename_all = "camelCase")]
pub enum ExperimentKind {
	/// <https://docs.discord.food/topics/experiments#user-experiments>
	///
	/// <https://docs.discord.food/topics/experiments#guild-experiments>
	Apex(ApexExperiment),
	/// <https://docs.discord.food/topics/experiments#apex-experiments>
	Normal(NormalExperiment),
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
pub enum ExperimentScope {
	#[serde(rename = "user")]
	User,
	#[serde(rename = "guild")]
	Guild,
	#[serde(rename = "installation")]
	Installation,
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
pub struct Experiment {
	#[daft(ignore)]
	pub loc: SpannedId,
	pub obj: ExperimentKind,
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
#[serde(rename_all = "camelCase")]
pub struct ApexExperiment {
	pub kind: ExperimentScope,
	pub name: String,
	#[typesize(with = size_serde_json_value)]
	#[daft(leaf)]
	pub default_config: serde_json::Value,
	pub label: Option<String>,
	pub variations: Vec<Variation>,
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
pub struct Variation {
	pub key: String,
	#[typesize(with = size_serde_json_value)]
	#[daft(leaf)]
	pub config: serde_json::Value,
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
pub struct NormalExperiment {
	pub kind: ExperimentScope,
	pub id: String,
	pub label: String,
	#[typesize(with = size_serde_json_value)]
	#[daft(leaf)]
	pub default_config: serde_json::Value,
	pub treatments: Vec<Treatment>,
	/// TODO: was seen in 2025-09_hotwheels_nvidia_boost
	pub common_trigger_point: Option<()>,
}

#[derive(
	Serialize,
	Deserialize,
	Debug,
	TypeSize,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Diffable,
)]
pub struct Treatment {
	pub id: i32,
	pub label: String,
	#[typesize(with = size_serde_json_value)]
	#[daft(leaf)]
	pub config: serde_json::Value,
}
