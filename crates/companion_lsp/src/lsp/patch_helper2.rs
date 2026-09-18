use std::{
	borrow::Cow,
	mem,
	sync::{Arc, atomic::AtomicU64},
};

use anyhow::{Context as _, Result, anyhow, bail};
use dashmap::DashMap;
use explorer_types::ModuleId;
use itertools::Itertools;
use miette::{Diagnostic, Severity};
use oxc::span::Span;
use parser_diag::LocalSource;
use patch_engine::{
	ApplyEvent,
	ApplyOptions,
	apply_patch,
	compile_patch_regexes,
};
use pretty_printer::format_with_alloc;
use smol_str::SmolStr;
use text_diff::DiffHunkKind;
use tokio::sync::Mutex;
use tower_lsp_server::ls_types::{Range, ShowDocumentParams, Uri};
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
	util::{iter::IterExt, slice_util::offset_into, uri},
	wss::types::to_client,
};

const INDENT: u8 = 2;

#[derive(Default)]
pub struct Helpers {
	/// Multiple patch helpers can be open for the same plugin file, one per
	/// patch being worked on.
	by_plugin_file: DashMap<Uri, Vec<Arc<PatchHelper>>>,
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
		if new_patches.len() == 1 {
			return Some(0);
		}
		if new_patches.is_empty() {
			return None;
		}
		let mut patches = Vec::from_iter(new_patches);
		if let Ok((idx, _)) = patches
			.iter()
			.enumerate()
			.filter(|(_, p)| self.patch.track_cmp(p))
			.exactly_one()
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
	fn get_reveal_range(&self, before: &str, after: &str) -> Option<Range> {
		use text_diff::diff;
		let changes = diff([before, after]);
		let mut first_empty = None;
		for change in changes {
			if change.kind == DiffHunkKind::Different {
				let new_insertions = change.contents[1];
				if new_insertions.is_empty() {
					first_empty.get_or_insert(new_insertions);
					continue;
				}
				let start = offset_into(after.as_bytes(), new_insertions)
					.expect("not substring of after") as u32;
				let span = Span::sized(start, new_insertions.len() as u32);
				return Some(
					self.files
						.encoding()
						.range_in(after, span),
				);
			}
		}
		first_empty.map(|s| {
			let start = offset_into(after.as_bytes(), s)
				.expect("not substring of after 2") as u32;
			debug_assert_eq!(
				s.len(),
				0,
				"first empty insertion should be empty"
			);
			let span = Span::sized(start, 0);
			self.files
				.encoding()
				.range_in(after, span)
		})
	}
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
			.entry(helper.plugin_file.clone())
			.or_default()
			.push(Arc::clone(&helper));
		let (current_source, reveal_span) = {
			let guard = helper.state.lock().await;
			let current_source = guard.current_source.clone();
			let unpatched_source = guard.unpatched_source.clone();
			drop(guard);
			let alloc = self.pool.get();
			let formatted_unpatched_source =
				format_with_alloc(&unpatched_source, &alloc, INDENT)
					.expect("how did this fail")
					.code;
			let reveal_span = self
				.get_reveal_range(&formatted_unpatched_source, &current_source);

			(current_source, reveal_span)
		};
		self.create_ephemeral_document(EphemeralDocument {
			uri: helper.ephemeral_file.clone(),
			content: current_source.as_str().into(),
		});
		self.show_document(ShowDocumentParams {
			external: None,
			// TODO: initial selection
			selection: reveal_span,
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

	fn update_state_replacement_text(
		&self,
		state: &mut State,
	) -> Result<String> {
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

		let new_str = if let Some(err) = err {
			String::from(&*err.source.unwrap().source_code)
		} else {
			format_with_alloc(&res.src, &alloc, INDENT)
				.expect("apply_patch didn't report a syntax error")
				.code
		};

		Ok(mem::replace(&mut state.current_source, new_str))
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
		compile_patch_regexes([&mut guard.patch]);
		guard.other_patches = patches;

		let old_source = self.update_state_replacement_text(&mut guard)?;
		let new_src = guard.current_source.clone();
		drop(guard);
		let reveal_range = self.get_reveal_range(&old_source, &new_src);
		self.update_ephemeral_document(EphemeralChange {
			uri: state.ephemeral_file.clone(),
			content: Some(new_src.into()),
			deleted: None,
		})?;
		self.show_document(ShowDocumentParams {
			uri: state.ephemeral_file.clone(),
			external: None,
			take_focus: Some(false),
			selection: reveal_range,
		})
		.await
		.context("Failed to update reveal range")?;
		Ok(())
	}

	pub(super) async fn patch_helper_change_hook(
		&self,
		uri: &Uri,
	) -> Result<()> {
		// clone out of the map so the ref isn't held across an await
		let helpers = self
			.patch_helpers
			.by_plugin_file
			.get(uri)
			.map(|x| x.value().clone())
			.unwrap_or_default();
		let mut first_err = None;
		for helper in helpers {
			// one failing helper shouldn't stop the others from updating
			if let Err(e) = self.update_state(&helper).await {
				first_err.get_or_insert(e);
			}
		}
		first_err.map_or(Ok(()), Err)
	}

	/// close any relevant patch helpers
	pub(super) fn patch_helper_close_hook(&self, uri: &Uri) {
		let Some(helper) = self.patch_helpers.remove(uri) else {
			return;
		};
		debug!(?helper.plugin_file, ?helper.ephemeral_file, "Closing patch helper");
		if let Err(e) =
			self.delete_ephemeral_document(helper.ephemeral_file.clone())
		{
			warn!("Failed to delete patch helper document:{e:?}");
		}
	}
}

impl Helpers {
	fn mint_id(&self) -> u64 {
		self.next_id
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
	}

	/// Forgets the helper backing `ephemeral_file`
	fn remove(&self, ephemeral_file: &Uri) -> Option<Arc<PatchHelper>> {
		let (_, helper) = self
			.by_ephemeral_file
			.remove(ephemeral_file)?;
		if let Some(mut siblings) = self
			.by_plugin_file
			.get_mut(&helper.plugin_file)
		{
			siblings.retain(|h| !Arc::ptr_eq(h, &helper));
			// drop so below doesn't deadlock
			drop(siblings);
			// remove an empty plugin -> vec entry
			self.by_plugin_file
				.remove_if(&helper.plugin_file, |_, v| v.is_empty());
		}
		Some(helper)
	}
}
