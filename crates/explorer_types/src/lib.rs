use derive_more::{Deref, Display, From, Into};
use serde::{Deserialize, Serialize};
use typesize::derive::TypeSize;

mod proto;
pub use proto::{
	FromHexError,
	decode_build_hash,
	encode_build_hash,
	google,
	wire::*,
};

pub type TModuleId = u32;

#[expect(clippy::unsafe_derive_deserialize)]
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
	Deserialize,
	Display,
	TypeSize,
)]
#[repr(transparent)]
pub struct ModuleId(pub TModuleId);

impl ModuleId {
	pub fn convert_vec(ids: Vec<TModuleId>) -> Vec<Self> {
		let (ptr, len, cap) = Vec::into_raw_parts(ids);
		// SAFETY: This is safe because `ModuleId` is a transparent wrapper around `TModuleId`
		unsafe { Vec::from_raw_parts(ptr.cast(), len, cap) }
	}
	pub fn unconvert_vec(ids: Vec<Self>) -> Vec<TModuleId> {
		let (ptr, len, cap) = Vec::into_raw_parts(ids);
		// SAFETY: This is safe because `ModuleId` is a transparent wrapper around `TModuleId`
		unsafe { Vec::from_raw_parts(ptr.cast(), len, cap) }
	}
}

pub mod legacy {
	use std::{
		collections::HashMap,
		time::{Duration, SystemTime},
	};

	use super::ModuleId;
	use jiff::{Timestamp, Zoned, tz::TimeZone};
	use oxc_span::Span;
	use serde::{Deserialize, Serialize};
	use typesize::derive::TypeSize;

	#[derive(Serialize, Deserialize, Default, Debug, Clone, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct BundleMetadata {
		pub build_hash: String,
		pub build_number: u32,
		pub first_seen: u64,
		pub entry_point: Option<ModuleId>,
		pub env_var_text: String,
	}

	#[derive(Serialize, Deserialize, Debug, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub enum ExportName {
		Default,
		#[serde(untagged)]
		Named(String),
	}

	#[derive(Serialize, Deserialize, Debug, Default, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct KeyModules {
		pub flux_dispatcher_class: Vec<(ModuleId, ExportName)>,
	}

	impl KeyModules {
		pub fn shrink_to_fit(&mut self) {
			let Self {
				flux_dispatcher_class,
			} = self;
			flux_dispatcher_class.shrink_to_fit();
		}
	}

	#[derive(Serialize, Deserialize, Debug, Default, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct DepInfo {
		pub key_modules: KeyModules,
		pub module_deps: HashMap<ModuleId, IncomingModuleDeps>,
	}

	impl DepInfo {
		pub fn shrink_to_fit(&mut self) {
			let Self {
				key_modules,
				module_deps,
			} = self;
			key_modules.shrink_to_fit();
			module_deps.shrink_to_fit();
			for v in module_deps.values_mut() {
				v.shrink_to_fit();
			}
		}
	}

	pub type ModuleSources = HashMap<String, Vec<ModuleId>>;

	pub type Modules = HashMap<ModuleId, String>;

	#[derive(Serialize, Deserialize, Debug, Default, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct FullBundle {
		pub metadata: BundleMetadata,
		pub dep_info: DepInfo,
		pub module_sources: HashMap<String, Vec<ModuleId>>,
		pub modules: HashMap<ModuleId, String>,
	}

	impl FullBundle {
		pub fn shrink_to_fit(&mut self) {
			let Self {
				metadata,
				dep_info,
				module_sources,
				modules,
			} = self;
			metadata.shrink_to_fit();
			dep_info.shrink_to_fit();
			module_sources.shrink_to_fit();
			for v in module_sources.values_mut() {
				v.shrink_to_fit();
			}
			modules.shrink_to_fit();
			for v in modules.values_mut() {
				v.shrink_to_fit();
			}
		}
	}

	#[derive(Serialize, Deserialize, Debug, Default, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct BuildList {
		/// array of zstd compressed, protobuf-encoded [`BundleMetadata`]
		pub builds: Vec<Box<[u8]>>,
	}

	impl TryFrom<f64> for ModuleId {
		// TODO: is this a good error type
		type Error = ();

		fn try_from(value: f64) -> Result<Self, Self::Error> {
			if value.fract() == 0.
				&& value >= 0.
				&& value <= f64::from(u32::MAX)
			{
				Ok(Self(value as u32))
			} else {
				Err(())
			}
		}
	}

	/// Information about a module's dependents
	#[derive(Debug, Clone, Default, Serialize, Deserialize, TypeSize)]
	#[serde(rename_all = "camelCase")]
	pub struct IncomingModuleDeps {
		/// The modules that require this module synchronously
		pub sync: Vec<ModuleId>,
		/// the module that require this module lazily (dynamic import)
		pub lazy: Vec<ModuleId>,
	}

	/// Information about a module's dependencies
	#[derive(Default, Clone, Debug, TypeSize)]
	pub struct OutgoingModuleDeps {
		/// The modules that this module requires synchronously
		pub sync: Vec<ModuleId>,
		/// the module that this module requires lazily (dynamic import)
		pub lazy: Vec<ModuleId>,
	}

	#[derive(Copy, Clone, Debug, TypeSize)]
	pub struct SpannedId {
		pub id: ModuleId,
		#[typesize(with = std::mem::size_of_val)]
		pub span: Span,
	}

	/// Information about a module's dependencies with source locations
	#[derive(Default, Clone, Debug, TypeSize)]
	pub struct OutgoingModuleDepsWithLocs {
		/// The modules that this module requires synchronously
		pub sync: Vec<SpannedId>,
		/// the module that this module requires lazily (dynamic import)
		pub lazy: Vec<SpannedId>,
	}

	impl OutgoingModuleDepsWithLocs {
		pub const fn new() -> Self {
			Self {
				sync: Vec::new(),
				lazy: Vec::new(),
			}
		}
	}

	impl From<OutgoingModuleDepsWithLocs> for OutgoingModuleDeps {
		fn from(value: OutgoingModuleDepsWithLocs) -> Self {
			Self {
				sync: value
					.sync
					.into_iter()
					.map(|s| s.id)
					.collect(),
				lazy: value
					.lazy
					.into_iter()
					.map(|s| s.id)
					.collect(),
			}
		}
	}

	impl OutgoingModuleDeps {
		pub const fn new() -> Self {
			Self {
				sync: Vec::new(),
				lazy: Vec::new(),
			}
		}
	}

	impl IncomingModuleDeps {
		pub fn shrink_to_fit(&mut self) {
			let Self { sync, lazy } = self;
			sync.shrink_to_fit();
			lazy.shrink_to_fit();
		}

		pub const fn new() -> Self {
			Self {
				sync: Vec::new(),
				lazy: Vec::new(),
			}
		}
	}

	impl BundleMetadata {
		pub fn shrink_to_fit(&mut self) {
			let Self {
				build_hash,
				build_number: _,
				first_seen: _,
				entry_point: _,
				env_var_text,
			} = self;
			build_hash.shrink_to_fit();
			env_var_text.shrink_to_fit();
		}

		pub fn first_seen_as_time(&self) -> SystemTime {
			SystemTime::UNIX_EPOCH + Duration::from_millis(self.first_seen)
		}

		pub fn first_seen_as_timestamp(&self) -> Timestamp {
			Timestamp::UNIX_EPOCH + Duration::from_millis(self.first_seen)
		}

		pub fn first_seen_as_zoned(&self) -> Zoned {
			self.first_seen_as_timestamp()
				.to_zoned(TimeZone::UTC)
		}
	}

	#[derive(Serialize, Deserialize, Debug, TypeSize)]
	/// the results of querying for the builds before and after a given timestamp
	pub struct TimestampQueryResults {
		pub before: Option<BundleMetadata>,
		pub after: Option<BundleMetadata>,
	}
}
