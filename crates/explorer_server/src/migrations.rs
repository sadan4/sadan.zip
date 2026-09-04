use std::{collections::HashMap, fs, io, path::Path};

use anyhow::{Context, Result, anyhow, bail};
use explorer_types::{
	BundleMetadata,
	DepInfo,
	ExportName,
	FullBundle,
	IncomingModuleDeps,
	KeyModules,
	ModuleId,
	ModuleSources,
	Modules,
};
use serde::Deserialize;
use tracing::{Level, error, info, instrument, span, warn};

use explorer_server_core::{
	DATA_FILE_NAME,
	build_has_data,
	get_root_build_path,
	get_version_file_path,
	read_full_bundle_from_dir,
	read_mpk_zst_file,
	write_full_bundle,
};

#[derive(Clone, Copy, Debug)]
#[repr(u16)]
enum Versions {
	V0 = 0,
	V1,
	V2,
	V3,
	V4,
	V5,
}

const CURRENT_VERSION: Versions = Versions::V5;

impl Versions {
	fn get_current() -> Result<Self> {
		let p = get_version_file_path()?;
		if p.exists() {
			Ok(
				match fs::read_to_string(p)?
					.trim()
					.parse::<u16>()?
				{
					0u16 => Self::V0,
					1 => Self::V1,
					2 => Self::V2,
					3 => Self::V3,
					4 => Self::V4,
					5 => Self::V5,
					_ => {
						bail!("Unknown version in version file")
					}
				},
			)
		} else {
			CURRENT_VERSION.save_as_current()?;
			Ok(CURRENT_VERSION)
		}
	}
	fn save_as_current(self) -> Result<()> {
		let v_str = u16::from(self).to_string();
		fs::write(get_version_file_path()?, v_str)?;
		Ok(())
	}
	fn get_migration(self) -> Box<dyn Migration> {
		match self {
			Self::V0 | Self::V1 | Self::V2 => {
				panic!("Migrations for versions below 3 are implemented in JS")
			}
			Self::V3 => Box::new(V3Migration),
			Self::V4 => Box::new(V4Migration),
			Self::V5 => Box::new(V5Migration),
		}
	}
	const fn next(self) -> Option<Self> {
		match self {
			Self::V0 => Some(Self::V1),
			Self::V1 => Some(Self::V2),
			Self::V2 => Some(Self::V3),
			Self::V3 => Some(Self::V4),
			Self::V4 => Some(Self::V5),
			Self::V5 => None,
		}
	}
}

impl From<Versions> for u16 {
	fn from(value: Versions) -> Self {
		value as Self
	}
}

trait Migration {
	fn migrate(&self) -> Result<()>;
}

struct V3Migration;

/// Re-read and write every bundle to use named fields in messagepack
struct V4Migration;

/// move [`env_var_text`](FullBundle::env_var_text) from [`BundleMetadata`] to [`FullBundle`]
struct V5Migration;

/// [`BundleMetadata`] as written by V4, where `envVarText` still lived here.
///
/// Optional so a build that has already been migrated still deserializes.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct V4BundleMetadata {
	build_hash: String,
	build_number: u32,
	first_seen: u64,
	entry_point: Option<ModuleId>,
	#[serde(default)]
	env_var_text: Option<String>,
}

/// [`FullBundle`] as written by V4. Deserializing into this instead of an
/// `rmpv::Value` keeps only one copy of the bundle in memory.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct V4FullBundle {
	metadata: V4BundleMetadata,
	dep_info: DepInfo,
	module_sources: ModuleSources,
	modules: Modules,
	/// only present once the build has been migrated
	#[serde(default)]
	env_var_text: Option<String>,
}

type JsModulesJson = HashMap<String, Vec<String>>;

#[derive(Deserialize)]
struct JsInfoJson {
	#[serde(rename = "buildHash")]
	build_hash: String,
	#[serde(rename = "buildNumber")]
	build_number: String,
	#[serde(rename = "firstSeen")]
	first_seen: u64,
	#[serde(rename = "entryPoint")]
	entry_point: Option<String>,
	#[serde(rename = "envVarText")]
	env_var_text: String,
}

#[derive(Deserialize)]
struct JsDepsJsonMainDepsEntryValue {
	#[serde(rename = "syncUses")]
	sync_uses: Vec<String>,
	#[serde(rename = "lazyUses")]
	lazy_uses: Vec<String>,
}

type JsDepsJsonMainDeps = HashMap<String, JsDepsJsonMainDepsEntryValue>;

#[derive(Deserialize)]
struct JsDepsJsonKeyModules {
	#[serde(rename = "fluxDispatcherClass")]
	flux_dispatcher_class: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct JsDepsJson {
	deps: JsDepsJsonMainDeps,
	#[serde(rename = "keyModules")]
	key_modules: JsDepsJsonKeyModules,
}

impl V3Migration {
	fn read_js_json<T>(path: &Path) -> Result<T>
	where
		T: for<'de> Deserialize<'de>,
	{
		let file = fs::File::open(path)?;
		let reader = io::BufReader::new(file);
		let des = serde_json::from_reader(reader)?;
		Ok(des)
	}
	fn read_js_bundle(path: &Path) -> Result<FullBundle> {
		let deps_json_path = path.join("deps.json");
		let info_json_path = path.join("info.json");
		let modules_json_path = path.join("modules.json");
		let modules_path = path.join(".modules");
		let deps_json: JsDepsJson = Self::read_js_json(&deps_json_path)?;
		let info_json: JsInfoJson = Self::read_js_json(&info_json_path)?;
		let modules_json: JsModulesJson =
			Self::read_js_json(&modules_json_path)?;
		let mut modules = HashMap::new();
		for m in fs::read_dir(modules_path)? {
			let m = m?;
			if m.file_type()?.is_dir() {
				bail!("Expected .modules to only contain files");
			}
			let m_path = m.path();
			let m_id = m_path
				.file_stem()
				.ok_or_else(|| {
					anyhow!("expected file in module dir to end with .js")
				})?
				.to_string_lossy()
				.parse()
				.map(ModuleId)?;
			let m_source = fs::read_to_string(m_path)?;

			modules.insert(m_id, m_source);
		}
		let ret = FullBundle {
			metadata: BundleMetadata {
				build_hash: info_json.build_hash,
				build_number: info_json.build_number.parse()?,
				first_seen: info_json.first_seen,
				entry_point: info_json
					.entry_point
					.map(|s| s.parse().map(ModuleId))
					.transpose()?,
			},
			env_var_text: info_json.env_var_text,
			dep_info: DepInfo {
				key_modules: KeyModules {
					flux_dispatcher_class: deps_json
						.key_modules
						.flux_dispatcher_class
						.into_iter()
						.map(|(k, v)| {
							Ok((k.parse().map(ModuleId)?, ExportName::Named(v)))
						})
						.collect::<Result<_>>()?,
				},
				module_deps: deps_json
					.deps
					.into_iter()
					.map(|(k, v)| {
						Ok((
							k.parse().map(ModuleId)?,
							IncomingModuleDeps {
								sync: v
									.sync_uses
									.into_iter()
									.map(|s| s.parse().map(ModuleId))
									.collect::<Result<_, _>>()?,
								lazy: v
									.lazy_uses
									.into_iter()
									.map(|s| s.parse().map(ModuleId))
									.collect::<Result<_, _>>()?,
							},
						))
					})
					.collect::<Result<_>>()?,
			},
			module_sources: modules_json
				.into_iter()
				.map(|(k, v)| {
					let new_v = v
						.into_iter()
						.map(|s| s.parse().map(ModuleId))
						.collect::<Result<_, _>>()?;
					Ok((k, new_v))
				})
				.collect::<Result<_>>()?,
			modules,
		};
		Ok(ret)
	}
	fn rm_dir_all_not_pred(
		path: &Path,
		exclude: impl Fn(&Path) -> bool,
	) -> Result<()> {
		for entry in fs::read_dir(path)? {
			let entry = entry?;
			let entry_path = entry.path();
			if exclude(&entry_path) {
				continue;
			}
			if entry.file_type()?.is_dir() {
				fs::remove_dir_all(entry_path)?;
			} else {
				fs::remove_file(entry_path)?;
			}
		}
		Ok(())
	}
	fn is_data_file(path: &Path) -> bool {
		path.file_name()
			.is_some_and(|f| f == DATA_FILE_NAME)
	}
}

impl Migration for V3Migration {
	fn migrate(&self) -> Result<()> {
		let base_build_path = get_root_build_path()?;
		for entry in fs::read_dir(&base_build_path)? {
			let entry = entry?;
			if !entry.file_type()?.is_dir() {
				continue;
			}
			let entry_path = entry.path();
			// read_dir does not add `.` and `..`, so this will always be Some
			if entry_path.file_name().unwrap() == "chunks" {
				continue;
			}
			if build_has_data(&entry_path) {
				bail!(
					"Build {} already has data file, invalid state!",
					entry_path.display()
				);
			}
			let bundle = Self::read_js_bundle(&entry_path)?;
			write_full_bundle(&bundle)?;
			Self::rm_dir_all_not_pred(&entry_path, Self::is_data_file)?;
		}
		let chunks_path = base_build_path.join("chunks");
		if chunks_path.exists() && chunks_path.is_dir() {
			fs::remove_dir_all(chunks_path)?;
		}
		Ok(())
	}
}

impl Migration for V4Migration {
	fn migrate(&self) -> Result<()> {
		let base_build_path = get_root_build_path()?;
		for entry in fs::read_dir(&base_build_path)? {
			let entry = entry?;
			if !entry.file_type()?.is_dir() {
				continue;
			}
			let entry_path = entry.path();
			if !build_has_data(&entry_path) {
				warn!(
					"Skipping {}: empty build directory, no data file",
					entry_path.display()
				);
				continue;
			}
			info!("Re-encoding build {}", entry_path.display());
			let data = read_full_bundle_from_dir(&entry_path)?;
			write_full_bundle(&data)?;
		}
		Ok(())
	}
}

impl Migration for V5Migration {
	fn migrate(&self) -> Result<()> {
		let base_build_path = get_root_build_path()?;
		for entry in fs::read_dir(&base_build_path)? {
			let entry = entry?;
			if !entry.file_type()?.is_dir() {
				continue;
			}
			let entry_path = entry.path();
			if !build_has_data(&entry_path) {
				warn!(
					"Skipping {}: empty build directory, no data file",
					entry_path.display()
				);
				continue;
			}
			let data_path = entry_path.join(DATA_FILE_NAME);
			let V4FullBundle {
				metadata,
				dep_info,
				module_sources,
				modules,
				env_var_text,
			} = read_mpk_zst_file(&data_path).with_context(|| {
				format!("Failed to read {}", data_path.display())
			})?;
			if env_var_text.is_some() {
				info!("Skipping {}: already migrated", entry_path.display());
				continue;
			}
			let env_var_text = metadata.env_var_text.with_context(|| {
				format!(
					"{} has no `envVarText` on either the bundle or its metadata",
					data_path.display()
				)
			})?;
			info!("Re-encoding build {}", entry_path.display());
			let full_bundle = FullBundle {
				metadata: BundleMetadata {
					build_hash: metadata.build_hash,
					build_number: metadata.build_number,
					first_seen: metadata.first_seen,
					entry_point: metadata.entry_point,
				},
				dep_info,
				module_sources,
				modules,
				env_var_text,
			};
			write_full_bundle(&full_bundle)
				.context("Failed to write full bundle")?;
		}
		Ok(())
	}
}

#[instrument]
#[expect(
	clippy::cognitive_complexity,
	reason = "https://github.com/rust-lang/rust-clippy/issues/14417"
)]
pub fn migrate_if_needed() -> Result<()> {
	let mut cur = Versions::get_current()?;
	while let Some(next) = cur.next() {
		let _ =
			span!(Level::INFO, "Migration", from = ?cur, to = ?next).entered();
		let mig = next.get_migration();
		info!("Starting migration");
		match mig.migrate() {
			Ok(()) => {
				info!("Migration successful");
				cur = next;
			}
			Err(e) => {
				error!("Migration failed: {}", e);
				return Err(e);
			}
		}
		cur.save_as_current()?;
		info!("Updated version on disk to {:?}", next);
	}
	Ok(())
}
