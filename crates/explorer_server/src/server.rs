use std::{
	fs::FileTimes,
	io,
	num::NonZeroUsize,
	time::{Duration, SystemTime},
};

use anyhow::Context;
use axum::{
	Router,
	body::{Body, Bytes},
	extract::{FromRequestParts, Path, Query, State},
	response::{IntoResponse, Response},
	routing::{get, post},
};
use explorer_server_core::{
	DATA_FILE_NAME,
	INTL_FILE_NAME,
	METADATA_FILE_NAME,
	get_around,
	get_build_path,
	get_root_build_path,
	read_intl_messages_from_dir,
	read_mpk_zst_file,
};
use explorer_types::{
	BuildList,
	BundleMetadata,
	Channel,
	FullBundle,
	TimestampQueryResults,
	intl::{IntlMessage, IntlMessages},
};
use git_hash::GIT_HASH;
use http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts};
use serde::Deserialize;
use serde_json::{Map, Value};
use sevenz_rust2::{
	ArchiveEntry,
	ArchiveWriter,
	EncoderConfiguration,
	SourceReader,
	encoder_options::Lzma2Options,
};
use tokio::{
	fs,
	net,
	task::{JoinSet, spawn_blocking},
};
use tokio_stream::{StreamExt, wrappers::ReadDirStream};
use tokio_util::io::ReaderStream;
use tower_http::cors;
use tracing::{error, info, instrument, warn};
use webpack_ast_parser::intl::{ast::hydrate::hydrate_ast, render_message};

type Result<T = Response> = std::result::Result<T, AppError>;

const ZSTD_MIME_TYPE: &str = "application/zstd";
const MSGPACK_MIME_TYPE: &str = "application/vnd.msgpack";
const SEVENZ_MIME_TYPE: &str = "application/x-7z-compressed";
const JSON_MIME_TYPE: &str = "application/json";
const TEXT_MIME_TYPE: &str = "text/plain; charset=utf-8";

const MB: usize = 1024 * 1024;

const fn zstd_headers(
	mime: &'static str,
) -> [(header::HeaderName, HeaderValue); 2] {
	[
		(header::CONTENT_TYPE, HeaderValue::from_static(mime)),
		(header::CONTENT_ENCODING, HeaderValue::from_static("zstd")),
	]
}

/// Headers for a response of `mime` type with a body of `len` bytes
fn content_headers(
	mime: &'static str,
	len: impl Into<HeaderValue>,
) -> [(header::HeaderName, HeaderValue); 2] {
	[
		(header::CONTENT_TYPE, HeaderValue::from_static(mime)),
		(header::CONTENT_LENGTH, len.into()),
	]
}

/// Builds a response of `mime` type with an in-memory body
fn sized_response(mime: &'static str, body: impl Into<Bytes>) -> Response {
	let body: Bytes = body.into();
	(content_headers(mime, body.len()), Body::from(body)).into_response()
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("internal server error: {:?}", self.0),
		)
			.into_response()
	}
}

impl<E> From<E> for AppError
where
	E: Into<anyhow::Error>,
{
	fn from(err: E) -> Self {
		Self(err.into())
	}
}

fn is_valid_build_hash(build_hash: &str) -> bool {
	build_hash
		.chars()
		.all(|c| c.is_ascii_hexdigit())
}

/// Resolves `latest`, `latest-stable` and `latest-canary` to the hash of
/// the newest matching build, passing anything else through as a hash
///
/// The `Err` is a response to send as-is: no build matches, or the hash is
/// invalid
async fn resolve_build_hash(
	state: &crate::State,
	id: String,
) -> std::result::Result<String, (StatusCode, String)> {
	let channel = match id.as_str() {
		"latest" => None,
		"latest-stable" => Some(Channel::Stable),
		"latest-canary" => Some(Channel::Canary),
		_ if is_valid_build_hash(&id) => return Ok(id),
		_ => {
			return Err((
				StatusCode::BAD_REQUEST,
				"invalid build hash".to_owned(),
			));
		}
	};
	let lock = state.read().await;
	lock.meta_by_time
		.values()
		.rev()
		.find(|m| channel.is_none_or(|c| m.channel.contains(&c)))
		.map(|m| m.build_hash.clone())
		.ok_or_else(|| {
			(StatusCode::NOT_FOUND, format!("no build matches {id}"))
		})
}

/// A valid build hash, extracted from a single path param that is either a
/// hash or an alias accepted by [`resolve_build_hash`]
struct BuildHash(String);

impl FromRequestParts<crate::State> for BuildHash {
	type Rejection = (StatusCode, String);

	async fn from_request_parts(
		parts: &mut Parts,
		state: &crate::State,
	) -> std::result::Result<Self, Self::Rejection> {
		let Path(id) = Path::<String>::from_request_parts(parts, state)
			.await
			.map_err(|e| (e.status(), e.body_text()))?;
		resolve_build_hash(state, id)
			.await
			.map(Self)
	}
}

#[axum::debug_handler(state = crate::State)]
async fn get_build_metadata(BuildHash(build_hash): BuildHash) -> Result {
	let meta_path = get_build_path(&build_hash)?.join(METADATA_FILE_NAME);
	if !fs::try_exists(&meta_path).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("build {build_hash} not found"),
		)
			.into_response());
	}
	// not big enough (<5KiB) to bother with streaming, just read it all into memory and send it
	let meta = fs::read(meta_path).await?;

	Ok(sized_response(ZSTD_MIME_TYPE, meta))
}

/// Streams `file_name` from the build directory of `build_hash`
async fn stream_build_file(build_hash: &str, file_name: &str) -> Result {
	let path = get_build_path(build_hash)?.join(file_name);
	if !fs::try_exists(&path).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("{file_name} for build {build_hash} not found"),
		)
			.into_response());
	}
	let file = fs::File::open(path).await?;
	let len = file.metadata().await?.len();

	// the default stream size is 4KiB, which makes our requests VERY slow
	// as our files are 25-30MiB. use a default of 5MiB to make them not slow.
	let stream = ReaderStream::with_capacity(file, 5 * MB);

	Ok((
		content_headers(ZSTD_MIME_TYPE, len),
		Body::from_stream(stream),
	)
		.into_response())
}

async fn get_build_full(BuildHash(build_hash): BuildHash) -> Result {
	stream_build_file(&build_hash, DATA_FILE_NAME).await
}

async fn intl_as_json(build_hash: &str) -> Result {
	let path = get_build_path(build_hash)?.join(INTL_FILE_NAME);
	if !fs::try_exists(&path).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("intl messages for build {build_hash} not found"),
		)
			.into_response());
	}
	let bts = spawn_blocking(move || {
		let guh = read_mpk_zst_file::<IntlMessages>(&path)?;
		let guh = serde_json::to_vec(&guh)?;
		anyhow::Ok(Bytes::from(guh))
	})
	.await??;
	Ok((
		[(
			header::CONTENT_TYPE,
			HeaderValue::from_static(JSON_MIME_TYPE),
		)],
		Body::from(bts),
	)
		.into_response())
}

async fn get_build_intl(
	headers: HeaderMap,
	BuildHash(build_hash): BuildHash,
) -> Result {
	let wants_json = try {
		headers
			.get(header::ACCEPT)?
			.to_str()
			.ok()?
			== JSON_MIME_TYPE
	}
	.unwrap_or_default();
	if wants_json {
		intl_as_json(&build_hash).await
	} else {
		stream_build_file(&build_hash, INTL_FILE_NAME).await
	}
}

#[derive(Deserialize)]
struct IntlKeyQuery {
	/// the hashed intl key
	key: String,
}

async fn find_intl_message(
	build_hash: String,
	key: &str,
) -> Result<std::result::Result<IntlMessage, Response>> {
	let build_path = get_build_path(&build_hash)?;
	if !fs::try_exists(build_path.join(INTL_FILE_NAME)).await? {
		return Ok(Err((
			StatusCode::NOT_FOUND,
			format!("intl messages for build {build_hash} not found"),
		)
			.into_response()));
	}
	let mut intl =
		spawn_blocking(move || read_intl_messages_from_dir(&build_path))
			.await??;
	Ok(intl
		.messages
		.remove(key)
		.ok_or_else(|| {
			(
				StatusCode::NOT_FOUND,
				format!("intl key {key} not found in build {build_hash}"),
			)
				.into_response()
		}))
}

async fn get_intl_raw(
	BuildHash(build_hash): BuildHash,
	Query(IntlKeyQuery { key }): Query<IntlKeyQuery>,
) -> Result {
	let message = match find_intl_message(build_hash, &key).await? {
		Ok(message) => message,
		Err(res) => return Ok(res),
	};
	let raw = serde_json::to_vec(&message.value)?;
	Ok(sized_response(JSON_MIME_TYPE, raw))
}

async fn get_intl_rendered(
	BuildHash(build_hash): BuildHash,
	Query(IntlKeyQuery { key }): Query<IntlKeyQuery>,
) -> Result {
	let message = match find_intl_message(build_hash, &key).await? {
		Ok(message) => message,
		Err(res) => return Ok(res),
	};
	let ast = match message.value {
		Value::Array(nodes) => nodes,
		// a message that is a single literal isn't wrapped in an array
		lit @ Value::String(_) => vec![lit],
		other => {
			return Err(anyhow::anyhow!(
				"unexpected intl message value: {other}"
			)
			.into());
		}
	};
	let nodes = hydrate_ast(ast).context("Failed to hydrate intl message")?;
	let mut rendered = String::new();
	render_message(&mut rendered, &nodes, &Map::new())
		.context("Failed to render intl message")?;
	Ok(sized_response(TEXT_MIME_TYPE, rendered))
}

// TODO: ratelimit to like 4/hr
#[instrument(skip(state))]
async fn touch_builds(State(state): State<crate::State>) -> Result {
	async fn update_times(
		file: fs::File,
		time: std::fs::FileTimes,
	) -> io::Result<()> {
		let file = file.into_std().await;
		tokio::task::spawn_blocking(move || file.set_times(time))
			.await
			.expect("should never panic")
	}
	async fn update_build_timestamp(entry: fs::DirEntry) -> Result<()> {
		let ft = entry.file_type().await?;
		if !ft.is_dir() {
			return Ok(());
		}
		let dir_path = entry.path();
		if fs::read_dir(&dir_path)
			.await?
			.next_entry()
			.await?
			.is_none()
		{
			warn!("skipping empty build directory: {}", dir_path.display());
			return Ok(());
		}
		let meta_path = dir_path.join(METADATA_FILE_NAME);
		let meta = tokio::task::spawn_blocking(move || -> Result<_> {
			read_mpk_zst_file::<BundleMetadata>(&meta_path)
				.context("Failed to read bundle metadata")
				.map_err(Into::into)
		})
		.await??;
		let time = meta.first_seen_as_time();
		let file_times = FileTimes::new().set_modified(time);
		let file = fs::File::open(dir_path).await?;
		update_times(file, file_times).await?;
		Ok(())
	}
	info!("updating build timestamps");
	let mut dirs = fs::read_dir(get_root_build_path()?).await?;
	let mut js = JoinSet::new();
	while let Some(d) = dirs.next_entry().await? {
		js.spawn(update_build_timestamp(d));
	}
	let mut err = None;
	while let Some(n) = js.join_next().await {
		if let Err(e) = n? {
			err = Some(e);
			break;
		}
	}
	js.join_all().await;
	state
		.populate_from_disk()
		.await
		.context("Failed to re-populate builds from disk")?;
	match err {
		Some(e) => Err(e),
		None => Ok(StatusCode::NO_CONTENT.into_response()),
	}
}

async fn get_before_timestamp(
	Path(timestamp): Path<u64>,
	State(state): State<crate::State>,
) -> Result {
	let time = SystemTime::UNIX_EPOCH + Duration::from_millis(timestamp);
	let state = state.read().await;
	// we discard the upper_bound because this method only reutrns the build before the given timestamp, not after
	let (lower_bound, _) = get_around(&state.meta_by_time, &time);
	let lower_bound = lower_bound.map(|(_, v)| v.as_ref().clone());
	drop(state);
	let ret_data = TimestampQueryResults {
		before: lower_bound,
		after: None,
	};

	let raw = rmp_serde::to_vec_named(&ret_data)?;
	Ok(sized_response(MSGPACK_MIME_TYPE, raw))
}

async fn get_before_hash(
	BuildHash(hash): BuildHash,
	State(state): State<crate::State>,
) -> Result {
	let state = state.read().await;
	let build = state.meta_by_hash.get(&hash);
	let Some(build) = build else {
		return Ok(StatusCode::NOT_FOUND.into_response());
	};
	let (before, _) =
		get_around(&state.meta_by_time, &build.first_seen_as_time());
	let before = before.map(|(_, v)| v.as_ref().clone());
	drop(state);
	let ret_data = TimestampQueryResults {
		before,
		after: None,
	};
	let raw = rmp_serde::to_vec_named(&ret_data)?;
	Ok(sized_response(MSGPACK_MIME_TYPE, raw))
}

fn make_archive(build_path: &std::path::Path) -> Result<Vec<u8>> {
	let b: FullBundle = read_mpk_zst_file(&build_path.join(DATA_FILE_NAME))?;
	// builds from before intl scraping may not have been backfilled yet
	let intl = build_path
		.join(INTL_FILE_NAME)
		.is_file()
		.then(|| read_intl_messages_from_dir(build_path))
		.transpose()?;
	// Most archives are around 22MB, allocate a bit more
	let buf = Vec::with_capacity(25 * MB);
	let mut a = ArchiveWriter::new(io::Cursor::new(buf))?;

	fn new_entry(name: String) -> ArchiveEntry {
		ArchiveEntry {
			name,
			is_directory: false,
			has_stream: true,
			// size and CRC are filled in from the reader
			..Default::default()
		}
	}

	// .modules folder
	let mut modules: Vec<_> = b.modules.into_iter().collect();
	modules.sort_unstable_by_key(|&(m_id, _)| m_id);

	// top-level files
	let deps_json = serde_json::to_vec(&b.dep_info)?;
	let info_json = serde_json::to_vec(&b.metadata)?;
	let modules_json = serde_json::to_vec(&b.module_sources)?;
	let intl_json = intl
		.as_ref()
		.map(serde_json::to_vec)
		.transpose()?;
	let mut top_level: Vec<(&str, &[u8])> = vec![
		("deps.json", &deps_json),
		("info.json", &info_json),
		("modules.json", &modules_json),
	];
	if let Some(intl_json) = &intl_json {
		top_level.push(("intl.json", intl_json));
	}

	const DICT_SIZE: u32 = 1 << 24;

	let threads =
		std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
	let mut lzma2 = Lzma2Options::from_level_mt(
		6,
		u32::try_from(threads).unwrap_or(u32::MAX),
		u64::from(DICT_SIZE),
	);
	lzma2.set_dictionary_size(DICT_SIZE);
	a.set_content_methods(vec![EncoderConfiguration::from(lzma2)]);

	let (entries, readers): (Vec<_>, Vec<_>) = modules
		.iter()
		.map(|(m_id, m_content)| {
			(
				new_entry(format!(".modules/{m_id}.js")),
				SourceReader::new(m_content.as_bytes()),
			)
		})
		.chain(top_level.iter().map(|&(name, data)| {
			(new_entry(name.to_owned()), SourceReader::new(data))
		}))
		.unzip();

	// LZMA2 will compress the data in parallel
	a.push_archive_entries(entries, readers)?;

	Ok(a.finish()?.into_inner())
}

#[instrument(skip(state))]
async fn get_bundle_archive(
	Path(file_name): Path<String>,
	State(state): State<crate::State>,
) -> Result {
	let Some(id) = file_name.strip_suffix(".7z") else {
		return Ok((
			StatusCode::BAD_REQUEST,
			"invalid archive name. expected <hash>.7z",
		)
			.into_response());
	};
	let build_hash = match resolve_build_hash(&state, id.to_owned()).await {
		Ok(hash) => hash,
		Err(res) => return Ok(res.into_response()),
	};
	let build_hash = build_hash.as_str();
	// a broken cache shouldn't take the endpoint down with it, so treat any
	// error here as a miss
	match state
		.cache
		.get_cached_archive(build_hash)
		.await
	{
		Ok(Some(archive)) => {
			info!("serving cached archive for build {build_hash}");
			return Ok(sized_response(SEVENZ_MIME_TYPE, archive));
		}
		Ok(None) => {}
		Err(e) => warn!("failed to read archive from cache: {e:?}"),
	}
	let build_path = get_build_path(build_hash)?;
	if !fs::try_exists(build_path.join(DATA_FILE_NAME)).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("build {build_hash} not found"),
		)
			.into_response());
	}
	let archive =
		Bytes::from(spawn_blocking(move || make_archive(&build_path)).await??);

	let cache = state.cache.clone();
	let cached_archive = archive.clone();
	let cached_hash = build_hash.to_owned();
	tokio::spawn(async move {
		if let Err(e) = cache
			.cache_archive(&cached_hash, &cached_archive)
			.await
		{
			warn!("failed to cache archive: {e:?}");
		}
	});

	Ok(sized_response(SEVENZ_MIME_TYPE, archive))
}

async fn get_all_builds() -> Result {
	let dirs = fs::read_dir(get_root_build_path()?).await?;
	let mut st = ReadDirStream::new(dirs);
	let mut builds = Vec::new();
	while let Some(p) = st.next().await {
		let p = p?;
		if !p.file_type().await?.is_dir() {
			continue;
		}
		let meta_path = p.path().join(METADATA_FILE_NAME);
		if !fs::try_exists(&meta_path).await? {
			continue;
		}
		let meta_file = fs::read(meta_path)
			.await?
			.into_boxed_slice();
		builds.push(meta_file);
	}
	let builds_mpk = rmp_serde::to_vec_named(&BuildList { builds })?;

	Ok(sized_response(MSGPACK_MIME_TYPE, builds_mpk))
}

async fn get_latest_build_meta(State(state): State<crate::State>) -> Result {
	let lock = state.read().await;
	let meta = lock
		.meta_by_time
		.iter()
		.next_back()
		.map(|(_, v)| v.clone());
	drop(lock);
	let Some(meta) = meta else {
		return Ok(
			(StatusCode::NOT_FOUND, "server has no builds").into_response()
		);
	};
	let raw =
		rmp_serde::to_vec_named(&*meta).context("Failed to serialize meta")?;
	Ok(sized_response(MSGPACK_MIME_TYPE, raw))
}

#[instrument]
pub async fn serve(bind_addr: &str, state: crate::State) -> anyhow::Result<()> {
	let app = Router::new()
		.route("/build/{id}/metadata", get(get_build_metadata))
		.route("/build/{id}/full", get(get_build_full))
		.route("/build/{id}/intl", get(get_build_intl))
		.route("/build/{id}/intl/raw", get(get_intl_raw))
		.route("/build/{id}/intl/rendered", get(get_intl_rendered))
		.route("/build/archive/{file_name}", get(get_bundle_archive))
		.route("/builds", get(get_all_builds))
		.route("/builds/before/time/{timestamp}", get(get_before_timestamp))
		.route("/builds/before/hash/{hash}", get(get_before_hash))
		.route("/builds/latest/meta", get(get_latest_build_meta))
		.route("/fixup-timestamps", post(touch_builds))
		.route("/version", get(|| async { GIT_HASH }))
		.with_state(state)
		.layer(cors::CorsLayer::new().allow_origin(cors::Any));
	let listener = net::TcpListener::bind(bind_addr).await?;
	info!("Server listening on http://{}", bind_addr);
	axum::serve(listener, app).await?;
	Ok(())
}
