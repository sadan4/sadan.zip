use axum::{
    Router,
    body::Body,
    extract::Path,
    response::{IntoResponse, Response},
    routing::get,
};
use explorer_types::BuildList;
use http::{StatusCode, header};
use tokio::{fs, net};
use tokio_stream::{StreamExt, wrappers::ReadDirStream};
use tokio_util::io::ReaderStream;
use tracing::{info, instrument};

use crate::util::{DATA_FILE_NAME, METADATA_FILE_NAME, get_build_path, get_root_build_path};

type Result<T = Response> = std::result::Result<T, AppError>;

const SERVER_ADDR: &str = "0.0.0.0:8080";
const ZSTD_MIME_TYPE: &str = "application/zstd";
const ZSTD_HEADERS: [(header::HeaderName, &str); 1] = [(header::CONTENT_TYPE, ZSTD_MIME_TYPE)];
const MSGPACK_MIME_TYPE: &str = "application/vnd.msgpack";
const MSGPACK_HEADERS: [(header::HeaderName, &str); 1] =
    [(header::CONTENT_TYPE, MSGPACK_MIME_TYPE)];

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("internal server error: {}", self.0),
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
    build_hash.chars().all(|c| c.is_ascii_hexdigit())
}

#[axum::debug_handler]
async fn get_build_metadata(Path(build_hash): Path<String>) -> Result<Response> {
    if !is_valid_build_hash(&build_hash) {
        return Ok((StatusCode::BAD_REQUEST, "invalid build hash").into_response());
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
        return Ok((StatusCode::BAD_REQUEST, "invalid build hash").into_response());
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
        let meta_file = fs::read(meta_path).await?.into_boxed_slice();
        builds.push(meta_file);
    }
    let builds_mpk = rmp_serde::to_vec(&BuildList { builds })?;
    let body = Body::from(builds_mpk);

    Ok((MSGPACK_HEADERS, body).into_response())
}

#[instrument]
pub async fn serve() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/build/{id}/metadata", get(get_build_metadata))
        .route("/build/{id}/full", get(get_build_full))
        .route("/builds", get(get_all_builds));
    let listener = net::TcpListener::bind(SERVER_ADDR).await?;
    info!("Server listening on http://{}", SERVER_ADDR);
    axum::serve(listener, app).await?;
    Ok(())
}
