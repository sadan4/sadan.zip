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
	extract::{Path, State},
	response::{IntoResponse, Response},
	routing::{get, post},
};
use explorer_server_core::{
	DATA_FILE_NAME,
	METADATA_FILE_NAME,
	get_around,
	get_build_path,
	get_root_build_path,
};
use explorer_types::{
	BuildList,
	BundleMetadata,
	FullBundle,
	TimestampQueryResults,
};
use git_hash::GIT_HASH;
use http::{StatusCode, header};
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
use tracing::{info, instrument, warn};

type Result<T = Response> = std::result::Result<T, AppError>;

const ZSTD_MIME_TYPE: &str = "application/zstd";
const ZSTD_HEADERS: [(header::HeaderName, &str); 1] =
	[(header::CONTENT_TYPE, ZSTD_MIME_TYPE)];
const MSGPACK_MIME_TYPE: &str = "application/vnd.msgpack";
const MSGPACK_HEADERS: [(header::HeaderName, &str); 1] =
	[(header::CONTENT_TYPE, MSGPACK_MIME_TYPE)];
const SEVENZ_MIME_TYPE: &str = "application/x-7z-compressed";
const SEVENZ_HEADERS: [(header::HeaderName, &str); 1] =
	[(header::CONTENT_TYPE, SEVENZ_MIME_TYPE)];

const MB: usize = 1024 * 1024;

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

#[axum::debug_handler]
async fn get_build_metadata(Path(build_hash): Path<String>) -> Result {
	if !is_valid_build_hash(&build_hash) {
		return Ok(
			(StatusCode::BAD_REQUEST, "invalid build hash").into_response()
		);
	}
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
	let meta_body = Body::from(meta);

	Ok((ZSTD_HEADERS, meta_body).into_response())
}

async fn get_build_full(Path(build_hash): Path<String>) -> Result {
	if !is_valid_build_hash(&build_hash) {
		return Ok(
			(StatusCode::BAD_REQUEST, "invalid build hash").into_response()
		);
	}
	let data_path = get_build_path(&build_hash)?.join(DATA_FILE_NAME);
	if !fs::try_exists(&data_path).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("build {build_hash} not found"),
		)
			.into_response());
	}
	let data_file = fs::File::open(data_path).await?;

	// the default stream size is 4KiB, which makes our requests VERY slow
	// as our files are 25-30MiB. use a default of 5MiB to make them not slow.
	let data_stream = ReaderStream::with_capacity(data_file, 5 * MB);

	let data_body = Body::from_stream(data_stream);

	Ok((ZSTD_HEADERS, data_body).into_response())
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
		let meta_zstd_raw = fs::read(meta_path)
			.await
			.context("Failed to read bundle metadata")?;
		let meta = tokio::task::spawn_blocking(move || -> Result<_> {
			let meta_raw = zstd::decode_all(&*meta_zstd_raw)?;
			let meta = rmp_serde::from_slice::<BundleMetadata>(&meta_raw)?;
			Ok(meta)
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
	let body = Body::from(raw);
	Ok((MSGPACK_HEADERS, body).into_response())
}

async fn get_before_hash(
	Path(hash): Path<String>,
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
	let body = Body::from(raw);
	Ok((MSGPACK_HEADERS, body).into_response())
}

fn make_archive(zstd_raw_data: &[u8]) -> Result<Vec<u8>> {
	let mpk_raw_data = zstd::decode_all(zstd_raw_data)?;
	let b: FullBundle = rmp_serde::from_slice(&mpk_raw_data)?;
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
	let top_level: [(&str, &[u8]); 3] = [
		("deps.json", &deps_json),
		("info.json", &info_json),
		("modules.json", &modules_json),
	];

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
	let Some(build_hash) = file_name.strip_suffix(".7z") else {
		return Ok((
			StatusCode::BAD_REQUEST,
			"invalid archive name. expected {hash}.7z",
		)
			.into_response());
	};
	if !is_valid_build_hash(build_hash) {
		return Ok(
			(StatusCode::BAD_REQUEST, "invalid build hash").into_response()
		);
	}
	// a broken cache shouldn't take the endpoint down with it, so treat any
	// error here as a miss
	match state
		.cache
		.get_cached_archive(build_hash)
		.await
	{
		Ok(Some(archive)) => {
			info!("serving cached archive for build {build_hash}");
			return Ok((SEVENZ_HEADERS, Body::from(archive)).into_response());
		}
		Ok(None) => {}
		Err(e) => warn!("failed to read archive from cache: {e:?}"),
	}
	let data_path = get_build_path(build_hash)?.join(DATA_FILE_NAME);
	if !fs::try_exists(&data_path).await? {
		return Ok((
			StatusCode::NOT_FOUND,
			format!("build {build_hash} not found"),
		)
			.into_response());
	}
	let data_file = fs::read(data_path).await?;

	let archive =
		Bytes::from(spawn_blocking(move || make_archive(&data_file)).await??);

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

	let body = Body::from(archive);

	Ok((SEVENZ_HEADERS, body).into_response())
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
	let body = Body::from(builds_mpk);

	Ok((MSGPACK_HEADERS, body).into_response())
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
	Ok((
		MSGPACK_HEADERS,
		Body::from(
			rmp_serde::to_vec_named(&*meta)
				.context("Failed to serialize meta")?,
		),
	)
		.into_response())
}

#[instrument]
pub async fn serve(bind_addr: &str, state: crate::State) -> anyhow::Result<()> {
	let app = Router::new()
		.route("/build/{id}/metadata", get(get_build_metadata))
		.route("/build/{id}/full", get(get_build_full))
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
