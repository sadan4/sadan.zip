use std::{
	fs::FileTimes,
	io,
	time::{Duration, SystemTime},
};

use axum::{
	Router,
	body::Body,
	extract::{Path, State},
	response::{IntoResponse, Response},
	routing::{get, post},
};
use explorer_server_core::{
	DATA_FILE_NAME,
	METADATA_FILE_NAME,
	compress_full_bundle_data,
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
use http::{StatusCode, header};
use tokio::{fs, net, task::JoinSet};
use tokio_stream::{StreamExt, wrappers::ReadDirStream};
use tokio_util::io::ReaderStream;
use tower_http::cors;
use tracing::{info, instrument};

type Result<T = Response> = std::result::Result<T, AppError>;

const ZSTD_MIME_TYPE: &str = "application/zstd";
const ZSTD_HEADERS: [(header::HeaderName, &str); 1] =
	[(header::CONTENT_TYPE, ZSTD_MIME_TYPE)];
const MSGPACK_MIME_TYPE: &str = "application/vnd.msgpack";
const MSGPACK_HEADERS: [(header::HeaderName, &str); 1] =
	[(header::CONTENT_TYPE, MSGPACK_MIME_TYPE)];

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
async fn get_build_metadata(
	Path(build_hash): Path<String>,
) -> Result<Response> {
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
	let meta_file = fs::File::open(meta_path).await?;

	let meta_stream = ReaderStream::new(meta_file);

	let meta_body = Body::from_stream(meta_stream);

	Ok((ZSTD_HEADERS, meta_body).into_response())
}

async fn get_build_full(Path(build_hash): Path<String>) -> Result<Response> {
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

	let data_stream = ReaderStream::new(data_file);

	let data_body = Body::from_stream(data_stream);

	Ok((ZSTD_HEADERS, data_body).into_response())
}

// TODO: ratelimit to like 4/hr
async fn touch_builds(State(state): State<crate::State>) -> Result<Response> {
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
		let meta_path = dir_path.join(METADATA_FILE_NAME);
		let meta_zstd_raw = fs::read(meta_path).await?;
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
	state.populate_from_disk().await?;
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

	let raw = rmp_serde::to_vec(&ret_data)?;
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
	let raw = rmp_serde::to_vec(&ret_data)?;
	let body = Body::from(raw);
	Ok((MSGPACK_HEADERS, body).into_response())
}

fn make_tarball(zstd_raw_data: &[u8]) -> Result<Vec<u8>> {
	let mpk_raw_data = zstd::decode_all(zstd_raw_data)?;
	let b: FullBundle = rmp_serde::from_slice(&mpk_raw_data)?;
	let mut a_data = Vec::new();
	let mut a = tar::Builder::new(&mut a_data);
	let mut header = tar::Header::new_gnu();
	header.set_mode(0o644);
	// .modules folder

	for (m_id, m_content) in b.modules {
		let m_content_bytes = m_content.as_bytes();
		header.set_size(m_content_bytes.len() as u64);
		a.append_data(
			&mut header,
			format!(".modules/{m_id}.js"),
			m_content_bytes,
		)?;
	}

	// top-level files
	// deps.json
	dbg!(&b.dep_info);
	let deps_json_data = serde_json::to_vec(&b.dep_info)?;
	header.set_size(deps_json_data.len() as u64);
	a.append_data(&mut header, "deps.json", &*deps_json_data)?;
	// info.json
	let info_json_data = serde_json::to_vec(&b.metadata)?;
	header.set_size(info_json_data.len() as u64);
	a.append_data(&mut header, "info.json", &*info_json_data)?;
	// modules.json
	let modules_json_data = serde_json::to_vec(&b.module_sources)?;
	header.set_size(modules_json_data.len() as u64);
	a.append_data(&mut header, "modules.json", &*modules_json_data)?;

	// we don't need the returned ref, we only want to check errors
	_ = a.into_inner()?;

	let zstd_a_data = compress_full_bundle_data(&a_data)?;

	Ok(zstd_a_data)
}

async fn get_bundle_tarball(
	Path(build_hash): Path<String>,
) -> Result<Response> {
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
	let data_file = fs::read(data_path).await?;

	let zstd_tarball = make_tarball(&data_file)?;

	let body = Body::from(zstd_tarball);

	Ok((ZSTD_HEADERS, body).into_response())
}

async fn get_all_builds() -> Result<Response> {
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
	let builds_mpk = rmp_serde::to_vec(&BuildList { builds })?;
	let body = Body::from(builds_mpk);

	Ok((MSGPACK_HEADERS, body).into_response())
}

#[instrument]
pub async fn serve(bind_addr: &str, state: crate::State) -> anyhow::Result<()> {
	let app = Router::new()
		.route("/build/{id}/metadata", get(get_build_metadata))
		.route("/build/{id}/full", get(get_build_full))
		.route("/build/{id}/archive.tar.zst", get(get_bundle_tarball))
		.route("/builds", get(get_all_builds))
		.route("/builds/before/time/{timestamp}", get(get_before_timestamp))
		.route("/builds/before/hash/{hash}", get(get_before_hash))
		.route("/fixup-timestamps", post(touch_builds))
		.with_state(state)
		.layer(cors::CorsLayer::new().allow_origin(cors::Any));
	let listener = net::TcpListener::bind(bind_addr).await?;
	info!("Server listening on http://{}", bind_addr);
	axum::serve(listener, app).await?;
	Ok(())
}
