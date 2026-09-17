use std::{
	borrow::Cow,
	sync::{Arc, atomic::AtomicU64},
};

use anyhow::{Context as _, Result, anyhow, bail};
use dashmap::DashMap;
use explorer_types::ModuleId;
use itertools::Itertools;
use miette::{Diagnostic, Severity};
use parser_diag::LocalSource;
use patch_engine::{
	ApplyEvent,
	ApplyOptions,
	apply_patch,
	compile_patch_regexes,
};
use pretty_printer::format_with_alloc;
use smol_str::SmolStr;
use tokio::sync::Mutex;
use tower_lsp_server::ls_types::{ShowDocumentParams, Uri};
use tracing::{debug, warn};
use vencord_ast_parser::{Match, Patch, VencordAstParser};

use crate::{
	LspResult,
	lsp::{
		self,
		custom::{
			EphemeralDocument,
			ephemera::{self, EphemeralChange},
		},
		lenses::PatchLensArgs,
	},
	util::{iter::IterExt, uri},
	wss::types::to_client,
};

const INDENT: u8 = 2;

#[derive(Default)]
pub struct Helpers {
	by_plugin_file: DashMap<Uri, Arc<PatchHelper>>,
	by_ephemeral_file: DashMap<Uri, Arc<PatchHelper>>,
	next_id: AtomicU64,
}

struct PatchHelper {
	plugin_file: Uri,
	ephemeral_file: Uri,
	state: Mutex<State>,
}
// FIXME: Store stats
struct State {
	patch: Patch,
	plugin_name: SmolStr,
	other_patches: Vec<Patch>,
	module_id: ModuleId,
	unpatched_source: String,
	current_source: String,
}

impl State {
	fn find_patch(&self, new_patches: &[Patch]) -> Option<usize> {
		let mut patches = Vec::from_iter(new_patches);
		if let Some((idx, _)) = patches
			.iter()
			.enumerate()
			.filter(|(_, p)| self.patch.track_cmp(p))
			.exactly_one()
			.ok()
		{
			return Some(idx);
		}
		patches.retain(|p| {
			// does it track to an ignored one
			!self
				.other_patches
				.iter()
				.any(|op| op.track_cmp(p))
				// do the finds match
				&& self.patch.find.track_cmp(&p.find)
		});
		match patches.len() {
			1 => Some(0),
			0 => {
				debug!("all patches were filtered out, no match found");
				None
			}
			_ => {
				debug!(
					?new_patches,
					"multiple patches matched, no match found"
				);
				None
			}
		}
	}
}

struct InitialState {
	patch: Patch,
	plugin_name: SmolStr,
	other_patches: Vec<Patch>,
}

fn view_uri(plugin_file: &Uri, id: u64) -> Result<Uri> {
	let file_path = uri::to_path(plugin_file)
		.context("Could not convert plugin URI to path")?;
	let [folder, file] = {
		let mut it = file_path.components().rev();
		let file = it
			.next()
			.map_or(Cow::Borrowed("file"), |x| x.as_os_str().to_string_lossy());
		let folder = it
			.next()
			.map_or(Cow::Borrowed("folder"), |x| {
				x.as_os_str().to_string_lossy()
			});
		[folder, file]
	};
	ephemera::uri(format!("/patch-helper/{folder}-{file}-{id}.js"))
}

impl lsp::Server {
	pub(super) async fn open_patch_helper(
		&self,
		args: PatchLensArgs,
	) -> Result<()> {
		let InitialState {
			patch,
			plugin_name,
			other_patches,
		} = self.get_starting_patches(&args.uri, args.hash)?;
		let (module_id, unpatched_source) = self
			.extract_module(&patch.find.v)
			.await?;
		let mut state = State {
			patch,
			plugin_name,
			other_patches,
			module_id,
			unpatched_source,
			current_source: String::new(),
		};
		self.update_state_replacement_text(&mut state)?;
		let helper = Arc::new(PatchHelper {
			ephemeral_file: view_uri(&args.uri, self.patch_helpers.mint_id())?,
			plugin_file: args.uri,
			state: Mutex::new(state),
		});
		self.patch_helpers
			.by_ephemeral_file
			.insert(helper.ephemeral_file.clone(), Arc::clone(&helper));
		self.patch_helpers
			.by_plugin_file
			.insert(helper.plugin_file.clone(), Arc::clone(&helper));
		self.create_ephemeral_document(EphemeralDocument {
			uri: helper.ephemeral_file.clone(),
			content: helper
				.state
				.lock()
				.await
				.current_source
				.as_str()
				.into(),
		});
		self.show_document(ShowDocumentParams {
			external: None,
			// TODO: initial selection
			selection: None,
			take_focus: Some(true),
			uri: helper.ephemeral_file.clone(),
		})
		.await?;
		Ok(())
	}
	async fn extract_module(&self, find: &Match) -> Result<(ModuleId, String)> {
		let search = match find {
			Match::Str(s) => to_client::Search::String {
				string: str::from_utf8(s.needle())
					.expect("always utf8")
					.to_owned(),
			},
			Match::Regex(r) => to_client::Search::Regex {
				regex: format!("/{}/{}", r.pattern, r.flags),
			},
		};
		let res = self
			.state
			.ws
			.send_msg(to_client::ExtractMessage {
				data: to_client::FindQuery::Search {
					search,
					use_patched: Some(false),
				},
			})
			.await
			.context("Failed to extract module patch wants")?;
		let m_id = res.module_result.module_number;
		let txt = res.module;
		Ok((m_id, txt))
	}
	// FIXME: needle is built from span, which can change while the content stays the same
	fn get_starting_patches(
		&self,
		uri: &Uri,
		needle: u64,
	) -> Result<InitialState> {
		let doc = self
			.files
			.get(uri)
			.context("document not found")?;
		let alloc = self.pool.get();

		let parser =
			VencordAstParser::try_new(&alloc, &doc.text, Some(uri.as_str()))
				.map_err(|inner| {
					let e = LocalSource {
						inner,
						name: uri.as_str(),
						source: &doc.text,
					};
					anyhow!("Failed to parse:{e:?}")
				})?;
		let mut patches = parser.patches(true).map_err(|e| {
			let e = LocalSource {
				inner: miette::Report::from(e),
				name: uri.as_str(),
				source: &doc.text,
			};
			anyhow!("Failed to get patches:{e:?}")
		})?;
		compile_patch_regexes(&mut patches);
		let patch_idx = patches
			.iter()
			.position(|p| p.content_hash() == needle)
			.context("patch not found")?;
		let patch = patches.swap_remove(patch_idx);
		let other_patches = patches;
		let plugin_name = parser
			.plugin_info()
			.ok()
			.map_or(const { SmolStr::new_static("MyPlugin") }, |info| {
				info.name
			});
		Ok(InitialState {
			patch,
			plugin_name,
			other_patches,
		})
	}

	fn update_state_replacement_text(&self, state: &mut State) -> Result<()> {
		let mut alloc = self.pool.get();
		let mut res = apply_patch(
			&mut alloc,
			&state.patch,
			state.unpatched_source.clone(),
			&ApplyOptions {
				plugin_name: Some(&state.plugin_name),
				..Default::default()
			},
		);
		alloc.reset();
		let err = try {
			let idx = res
				.events
				.iter()
				.position(|e| matches!(e, ApplyEvent::SyntaxError { .. }))?;
			let err = res.events.remove(idx);
			let ApplyEvent::SyntaxError {
				replace_span: _,
				cause,
			} = err
			else {
				unreachable!()
			};
			cause
		};

		if let Some(err) = err {
			let src = &*err.source.unwrap().source_code;
			state.current_source.clear();
			state.current_source.push_str(src);
		} else {
			let fmt = format_with_alloc(&res.src, &alloc, INDENT)
				.expect("apply_patch didn't report a syntax error");
			state.current_source = fmt.code;
		}

		Ok(())
	}

	async fn update_state(&self, state: &PatchHelper) -> Result<()> {
		let doc = self
			.files
			.get(&state.plugin_file)
			.context("plugin file not found")?;
		// !send + !sync
		let mut patches = {
			let alloc = self.pool.get();
			let parser = match VencordAstParser::try_new(
				&alloc,
				&doc.text,
				Some(state.plugin_file.as_str()),
			) {
				Ok(p) => p,
				Err(inner) => {
					let e = LocalSource {
						inner,
						name: state.plugin_file.as_str(),
						source: &doc.text,
					};
					warn!(
						"Failed to parse changed plugin file, skipping:{e:?}"
					);
					return Ok(());
				}
			};
			match parser.patches(true) {
				Ok(p) => p,
				Err(e) => {
					let e = LocalSource {
						inner: miette::Report::from(e),
						name: state.plugin_file.as_str(),
						source: &doc.text,
					};
					warn!(
						"Failed to get patches from changed plugin file, skipping:{e:?}"
					);
					return Ok(());
				}
			}
		};
		let mut guard = state.state.lock().await;
		let Some(idx) = guard.find_patch(&patches) else {
			bail!("Failed to find patch in changed plugin file, skipping");
		};
		guard.patch = patches.swap_remove(idx);
		guard.other_patches = patches;

		self.update_state_replacement_text(&mut guard)?;
		let src = guard.current_source.clone();
		drop(guard);
		self.update_ephemeral_document(EphemeralChange {
			uri: state.ephemeral_file.clone(),
			content: Some(src.into()),
			deleted: None,
		})
	}

	pub(super) async fn patch_helper_change_hook(
		&self,
		uri: &Uri,
	) -> Result<()> {
		let p = self
			.patch_helpers
			.by_plugin_file
			.get(uri)
			.map(|x| Arc::clone(&x));
		if let Some(p) = p {
			let p2 = Arc::clone(&p);
			drop(p);
			let p = p2;
			self.update_state(&p).await?;
		}
		Ok(())
	}
}

impl Helpers {
	fn mint_id(&self) -> u64 {
		self.next_id
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
	}
}
