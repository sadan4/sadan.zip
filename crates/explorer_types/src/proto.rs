use std::{
	ptr,
	time::{Duration, SystemTime},
};

use jiff::tz::TimeZone;

use crate::{ModuleId, TModuleId, legacy};

pub mod google {
	pub mod protobuf {
		include!(concat!(env!("OUT_DIR"), "/google.protobuf.rs"));
	}
}

pub mod wire {
	include!(concat!(env!("OUT_DIR"), "/explorer_types.rs"));
}

pub use const_hex::FromHexError;

/// decodes the hex representation of a build hash into the raw bytes stored in
/// [`wire::BundleMetadata::build_hash`]
pub fn decode_build_hash(hex: &str) -> Result<Vec<u8>, FromHexError> {
	const_hex::decode(hex)
}

/// encodes the raw bytes of a build hash as hex, which is how a build is named
/// on disk and in urls
pub fn encode_build_hash(bytes: &[u8]) -> String {
	const_hex::encode(bytes)
}

impl google::protobuf::Timestamp {
	pub fn now() -> Self {
		SystemTime::now().into()
	}
}

impl From<google::protobuf::Timestamp> for jiff::Timestamp {
	fn from(value: google::protobuf::Timestamp) -> Self {
		Self::new(value.seconds, value.nanos)
			.expect("Invalid timestamp in protobuf")
	}
}

// upstream uses From
#[expect(clippy::fallible_impl_from)]
impl From<SystemTime> for google::protobuf::Timestamp {
	fn from(system_time: SystemTime) -> Self {
		let (seconds, nanos) =
			match system_time.duration_since(std::time::UNIX_EPOCH) {
				Ok(duration) => {
					let seconds = i64::try_from(duration.as_secs()).unwrap();
					(seconds, duration.subsec_nanos() as i32)
				}
				Err(error) => {
					let duration = error.duration();
					let seconds = i64::try_from(duration.as_secs()).unwrap();
					let nanos = duration.subsec_nanos() as i32;
					if nanos == 0 {
						(-seconds, 0)
					} else {
						(-seconds - 1, 1_000_000_000 - nanos)
					}
				}
			};
		Self { seconds, nanos }
	}
}

impl From<oxc_span::Span> for wire::Span {
	fn from(value: oxc_span::Span) -> Self {
		Self {
			start: value.start,
			end: value.end,
		}
	}
}

impl From<wire::Span> for oxc_span::Span {
	fn from(value: wire::Span) -> Self {
		Self::new(value.start, value.end)
	}
}

impl wire::IncomingModuleDeps {
	#[inline]
	pub const fn sync_ids(&self) -> &[ModuleId] {
		let raw_id_slice: &[TModuleId] = self.sync.as_slice();
		// SAFETY: This is safe because `ModuleId` is a transparent wrapper around `TModuleId`
		unsafe {
			&*(ptr::from_ref::<[TModuleId]>(raw_id_slice)
				as *const [ModuleId])
		}
	}

	#[inline]
	pub const fn lazy_ids(&self) -> &[ModuleId] {
		let raw_id_slice: &[TModuleId] = self.lazy.as_slice();
		// SAFETY: This is safe because `ModuleId` is a transparent wrapper around `TModuleId`
		unsafe {
			&*(ptr::from_ref::<[TModuleId]>(raw_id_slice)
				as *const [ModuleId])
		}
	}
}

impl wire::OutgoingModuleDepsWithLocs {
	pub const fn new() -> Self {
		Self {
			sync: Vec::new(),
			lazy: Vec::new(),
		}
	}
}

impl wire::BundleMetadata {
	pub fn first_seen_as_timestamp(&self) -> jiff::Timestamp {
		self.first_seen
			.unwrap_or_default()
			.into()
	}

	pub fn first_seen_as_zoned(&self) -> jiff::Zoned {
		self.first_seen_as_timestamp()
			.to_zoned(TimeZone::UTC)
	}

	/// the hex representation of [`Self::build_hash`], which is how a build is
	/// named on disk and in urls
	pub fn build_hash_hex(&self) -> String {
		encode_build_hash(&self.build_hash)
	}

	/// sets [`Self::build_hash`] from its hex representation
	pub fn set_build_hash_hex(
		&mut self,
		hex: &str,
	) -> Result<(), FromHexError> {
		self.build_hash = decode_build_hash(hex)?;
		Ok(())
	}

	pub fn shrink_to_fit(&mut self) {
		let Self {
			build_hash,
			build_number: _,
			first_seen: _,
			entry_point: _,
		} = self;
		build_hash.shrink_to_fit();
	}
}

impl wire::KeyModules {
	pub fn shrink_to_fit(&mut self) {
		let Self {
			flux_dispatcher_class,
		} = self;
		flux_dispatcher_class.shrink_to_fit();
	}
}

impl wire::DepInfo {
	pub fn shrink_to_fit(&mut self) {
		let Self {
			key_modules,
			module_deps,
		} = self;
		if let Some(key_modules) = key_modules {
			key_modules.shrink_to_fit();
		}
		module_deps.shrink_to_fit();
	}
}

impl wire::ModuleSources {
	pub fn shrink_to_fit(&mut self) {
		let Self { sources } = self;
		sources.shrink_to_fit();
	}
}

impl wire::FullBundle {
	pub fn shrink_to_fit(&mut self) {
		let Self {
			metadata,
			dep_info,
			module_sources,
			modules,
			env_var_text,
		} = self;
		if let Some(metadata) = metadata {
			metadata.shrink_to_fit();
		}
		if let Some(dep_info) = dep_info {
			dep_info.shrink_to_fit();
		}
		if let Some(module_sources) = module_sources {
			module_sources.shrink_to_fit();
		}
		modules.shrink_to_fit();
		env_var_text.shrink_to_fit();
	}
}

/// `env_var_text` is dropped by the [`legacy::BundleMetadata`] conversion
/// because it lives on [`wire::FullBundle`] now; the [`legacy::FullBundle`]
/// conversion carries it over.
impl TryFrom<legacy::BundleMetadata> for wire::BundleMetadata {
	type Error = FromHexError;

	fn try_from(value: legacy::BundleMetadata) -> Result<Self, Self::Error> {
		let legacy::BundleMetadata {
			build_hash,
			build_number,
			first_seen,
			entry_point,
			env_var_text: _,
		} = value;
		Ok(Self {
			build_hash: decode_build_hash(&build_hash)?,
			build_number,
			first_seen: Some(
				(SystemTime::UNIX_EPOCH + Duration::from_millis(first_seen))
					.into(),
			),
			entry_point: entry_point.map(ModuleId::into),
		})
	}
}

impl From<legacy::ExportName> for wire::ExportName {
	fn from(value: legacy::ExportName) -> Self {
		Self {
			kind: Some(match value {
				legacy::ExportName::Default => {
					wire::export_name::Kind::DefaultExport(
						google::protobuf::Empty {},
					)
				}
				legacy::ExportName::Named(name) => {
					wire::export_name::Kind::Named(name)
				}
			}),
		}
	}
}

impl From<legacy::KeyModules> for wire::KeyModules {
	fn from(value: legacy::KeyModules) -> Self {
		let legacy::KeyModules {
			flux_dispatcher_class,
		} = value;
		Self {
			flux_dispatcher_class: flux_dispatcher_class
				.into_iter()
				.map(|(id, export_name)| wire::FluxDispatcherEntry {
					module_id: id.into(),
					export_name: Some(export_name.into()),
				})
				.collect(),
		}
	}
}

impl From<legacy::IncomingModuleDeps> for wire::IncomingModuleDeps {
	fn from(value: legacy::IncomingModuleDeps) -> Self {
		let legacy::IncomingModuleDeps { sync, lazy } = value;
		Self {
			sync: ModuleId::unconvert_vec(sync),
			lazy: ModuleId::unconvert_vec(lazy),
		}
	}
}

impl From<legacy::DepInfo> for wire::DepInfo {
	fn from(value: legacy::DepInfo) -> Self {
		let legacy::DepInfo {
			key_modules,
			module_deps,
		} = value;
		Self {
			key_modules: Some(key_modules.into()),
			module_deps: module_deps
				.into_iter()
				.map(|(id, deps)| (id.into(), deps.into()))
				.collect(),
		}
	}
}

impl From<legacy::ModuleSources> for wire::ModuleSources {
	fn from(value: legacy::ModuleSources) -> Self {
		Self {
			sources: value
				.into_iter()
				.map(|(source, ids)| {
					(
						source,
						wire::ModuleIdList {
							ids: ModuleId::unconvert_vec(ids),
						},
					)
				})
				.collect(),
		}
	}
}

impl TryFrom<legacy::FullBundle> for wire::FullBundle {
	type Error = FromHexError;

	fn try_from(value: legacy::FullBundle) -> Result<Self, Self::Error> {
		let legacy::FullBundle {
			metadata,
			dep_info,
			module_sources,
			modules,
		} = value;
		let env_var_text = metadata.env_var_text.clone();
		Ok(Self {
			metadata: Some(metadata.try_into()?),
			dep_info: Some(dep_info.into()),
			module_sources: Some(module_sources.into()),
			modules: modules
				.into_iter()
				.map(|(id, source)| (id.into(), source))
				.collect(),
			env_var_text,
		})
	}
}
