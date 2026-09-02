use prost::Message as _;

use crate::{
	BuildList,
	BundleMetadata,
	DepInfo,
	ExportName,
	FullBundle,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	TimestampQueryResults,
};

#[derive(prost::Message)]
pub struct FooBar {
	#[prost(tag = "1")]
	pub baz: ModuleId,
}

#[allow(clippy::all, clippy::pedantic, clippy::nursery, missing_docs)]
pub mod wire {
	include!(concat!(env!("OUT_DIR"), "/explorer_types.rs"));
}

#[derive(Debug, thiserror::Error)]
pub enum ProtoDecodeError {
	#[error("failed to decode protobuf message: {0}")]
	Decode(#[from] prost::DecodeError),
	#[error("missing required field `{0}`")]
	MissingField(&'static str),
	#[error("oneof `{0}` had no variant set")]
	MissingOneof(&'static str),
}

/// A domain type that has a protobuf wire representation.
pub trait ProtoWire: Sized {
	fn encode_proto(&self) -> Vec<u8>;
	fn decode_proto(buf: &[u8]) -> Result<Self, ProtoDecodeError>;
}

macro_rules! impl_proto_wire {
	($domain:ty, $wire:ty) => {
		impl ProtoWire for $domain {
			fn encode_proto(&self) -> Vec<u8> {
				<$wire>::from(self).encode_to_vec()
			}
			fn decode_proto(buf: &[u8]) -> Result<Self, ProtoDecodeError> {
				<$wire>::decode(buf)?.try_into()
			}
		}
	};
}

impl_proto_wire!(BundleMetadata, wire::BundleMetadata);
impl_proto_wire!(FullBundle, wire::FullBundle);
impl_proto_wire!(TimestampQueryResults, wire::TimestampQueryResults);

impl ProtoWire for BuildList {
	fn encode_proto(&self) -> Vec<u8> {
		wire::BuildList::from(self).encode_to_vec()
	}

	fn decode_proto(buf: &[u8]) -> Result<Self, ProtoDecodeError> {
		Ok(wire::BuildList::decode(buf)?.into())
	}
}

impl From<&BundleMetadata> for wire::BundleMetadata {
	fn from(v: &BundleMetadata) -> Self {
		Self {
			build_hash: v.build_hash.clone(),
			build_number: v.build_number,
			first_seen: v.first_seen,
			entry_point: v.entry_point.map(u32::from),
			env_var_text: v.env_var_text.clone(),
		}
	}
}

impl TryFrom<wire::BundleMetadata> for BundleMetadata {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::BundleMetadata) -> Result<Self, Self::Error> {
		Ok(Self {
			build_hash: v.build_hash,
			build_number: v.build_number,
			first_seen: v.first_seen,
			entry_point: v.entry_point.map(ModuleId),
			env_var_text: v.env_var_text,
		})
	}
}

impl From<&ExportName> for wire::ExportName {
	fn from(v: &ExportName) -> Self {
		let kind = match v {
			ExportName::Default => wire::export_name::Kind::IsDefault(true),
			ExportName::Named(s) => wire::export_name::Kind::Named(s.clone()),
		};
		Self { kind: Some(kind) }
	}
}

impl TryFrom<wire::ExportName> for ExportName {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::ExportName) -> Result<Self, Self::Error> {
		match v
			.kind
			.ok_or(ProtoDecodeError::MissingOneof("ExportName.kind"))?
		{
			wire::export_name::Kind::IsDefault(_) => Ok(Self::Default),
			wire::export_name::Kind::Named(s) => Ok(Self::Named(s)),
		}
	}
}

impl From<&KeyModules> for wire::KeyModules {
	fn from(v: &KeyModules) -> Self {
		Self {
			flux_dispatcher_class: v
				.flux_dispatcher_class
				.iter()
				.map(|(id, name)| wire::FluxDispatcherEntry {
					module_id: (*id).into(),
					export_name: Some(name.into()),
				})
				.collect(),
		}
	}
}

impl TryFrom<wire::KeyModules> for KeyModules {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::KeyModules) -> Result<Self, Self::Error> {
		Ok(Self {
			flux_dispatcher_class: v
				.flux_dispatcher_class
				.into_iter()
				.map(|e| {
					let export_name =
						e.export_name
							.ok_or(ProtoDecodeError::MissingField(
								"FluxDispatcherEntry.export_name",
							))?;
					Ok((
						ModuleId(e.module_id),
						ExportName::try_from(export_name)?,
					))
				})
				.collect::<Result<_, ProtoDecodeError>>()?,
		})
	}
}

impl From<&IncomingModuleDeps> for wire::IncomingModuleDeps {
	fn from(v: &IncomingModuleDeps) -> Self {
		Self {
			sync: v
				.sync
				.iter()
				.map(|id| (*id).into())
				.collect(),
			lazy: v
				.lazy
				.iter()
				.map(|id| (*id).into())
				.collect(),
		}
	}
}

impl From<wire::IncomingModuleDeps> for IncomingModuleDeps {
	fn from(v: wire::IncomingModuleDeps) -> Self {
		Self {
			sync: v
				.sync
				.into_iter()
				.map(ModuleId)
				.collect(),
			lazy: v
				.lazy
				.into_iter()
				.map(ModuleId)
				.collect(),
		}
	}
}

impl From<&DepInfo> for wire::DepInfo {
	fn from(v: &DepInfo) -> Self {
		Self {
			key_modules: Some((&v.key_modules).into()),
			module_deps: v
				.module_deps
				.iter()
				.map(|(id, deps)| ((*id).into(), deps.into()))
				.collect(),
		}
	}
}

impl TryFrom<wire::DepInfo> for DepInfo {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::DepInfo) -> Result<Self, Self::Error> {
		let key_modules = v
			.key_modules
			.ok_or(ProtoDecodeError::MissingField("DepInfo.key_modules"))?;
		Ok(Self {
			key_modules: key_modules.try_into()?,
			module_deps: v
				.module_deps
				.into_iter()
				.map(|(id, deps)| (ModuleId(id), deps.into()))
				.collect(),
		})
	}
}

impl From<&FullBundle> for wire::FullBundle {
	fn from(v: &FullBundle) -> Self {
		Self {
			metadata: Some((&v.metadata).into()),
			dep_info: Some((&v.dep_info).into()),
			module_sources: v
				.module_sources
				.iter()
				.map(|(k, ids)| {
					(
						k.clone(),
						wire::ModuleIdList {
							ids: ids
								.iter()
								.map(|id| (*id).into())
								.collect(),
						},
					)
				})
				.collect(),
			modules: v
				.modules
				.iter()
				.map(|(id, src)| ((*id).into(), src.clone()))
				.collect(),
		}
	}
}

impl TryFrom<wire::FullBundle> for FullBundle {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::FullBundle) -> Result<Self, Self::Error> {
		let metadata = v
			.metadata
			.ok_or(ProtoDecodeError::MissingField("FullBundle.metadata"))?;
		let dep_info = v
			.dep_info
			.ok_or(ProtoDecodeError::MissingField("FullBundle.dep_info"))?;
		Ok(Self {
			metadata: metadata.try_into()?,
			dep_info: dep_info.try_into()?,
			module_sources: v
				.module_sources
				.into_iter()
				.map(|(k, list)| {
					(
						k,
						list.ids
							.into_iter()
							.map(ModuleId)
							.collect(),
					)
				})
				.collect(),
			modules: v
				.modules
				.into_iter()
				.map(|(id, src)| (ModuleId(id), src))
				.collect(),
		})
	}
}

impl From<&BuildList> for wire::BuildList {
	fn from(v: &BuildList) -> Self {
		Self {
			builds: v
				.builds
				.iter()
				.map(|b| b.to_vec())
				.collect(),
		}
	}
}

impl From<wire::BuildList> for BuildList {
	fn from(v: wire::BuildList) -> Self {
		Self {
			builds: v
				.builds
				.into_iter()
				.map(Vec::into_boxed_slice)
				.collect(),
		}
	}
}

impl From<&TimestampQueryResults> for wire::TimestampQueryResults {
	fn from(v: &TimestampQueryResults) -> Self {
		Self {
			before: v.before.as_ref().map(Into::into),
			after: v.after.as_ref().map(Into::into),
		}
	}
}

impl TryFrom<wire::TimestampQueryResults> for TimestampQueryResults {
	type Error = ProtoDecodeError;

	fn try_from(v: wire::TimestampQueryResults) -> Result<Self, Self::Error> {
		Ok(Self {
			before: v
				.before
				.map(TryInto::try_into)
				.transpose()?,
			after: v
				.after
				.map(TryInto::try_into)
				.transpose()?,
		})
	}
}
