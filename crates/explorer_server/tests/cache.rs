use std::{
	collections::HashMap,
	fs,
	net::TcpListener,
	path::{Path, PathBuf},
	process::{Child, Command, Stdio},
	time::Duration,
};

use explorer_server_core::{DATA_FILE_NAME, METADATA_FILE_NAME};
use explorer_types::{
	BundleMetadata,
	DepInfo,
	FullBundle,
	ModuleId,
	ProtoWire,
};
use redis::AsyncCommands as _;
use tempfile::TempDir;
use tokio::time::{Instant, sleep};

/// must match `ARCHIVE_KEY_PREFIX` in `src/cache.rs`
const KEY_PREFIX: &str = "discord-build-archive:";
/// must match `ARCHIVE_TTL` in `src/cache.rs`
const ARCHIVE_TTL: i64 = 60 * 60 * 24 * 7;

const BUILD_HASH: &str = "0f1e2d3c4b5a69788796a5b4c3d2e1f000112233";
const SEVENZ_MIME: &str = "application/x-7z-compressed";

const CACHE_START_TIMEOUT: Duration = Duration::from_secs(20);
const SERVER_START_TIMEOUT: Duration = Duration::from_secs(30);
const CACHE_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum Flavor {
	Redis,
	Valkey,
}

impl Flavor {
	const fn server_bin(self) -> &'static str {
		match self {
			Self::Redis => "redis-server",
			Self::Valkey => "valkey-server",
		}
	}
}

#[derive(Clone, Copy)]
enum Transport {
	Tcp,
	Unix,
}

/// kills the child however the test ends, panic included
struct Reaper(Child);

impl Drop for Reaper {
	fn drop(&mut self) {
		let _ = self.0.kill();
		let _ = self.0.wait();
	}
}

fn free_port() -> u16 {
	TcpListener::bind("127.0.0.1:0")
		.expect("no free port")
		.local_addr()
		.unwrap()
		.port()
}

/// Writes a small but structurally real bundle for `make_archive` to chew on.
/// Deliberately tiny — what's under test is the cache, not LZMA2 throughput.
fn write_fixture(root: &Path) {
	let build_dir = root.join("builds").join(BUILD_HASH);
	fs::create_dir_all(&build_dir).unwrap();

	let metadata = BundleMetadata {
		build_hash: BUILD_HASH.to_owned(),
		build_number: 1,
		first_seen: 1_700_000_000_000,
		entry_point: None,
		env_var_text: String::new(),
	};

	let mut modules = HashMap::new();
	let mut module_sources = HashMap::new();
	for id in 0..8u32 {
		modules.insert(
			ModuleId::from(id),
			format!("export const m{id} = () => {id};\n").repeat(32),
		);
		module_sources
			.insert(format!("module-{id}.js"), vec![ModuleId::from(id)]);
	}

	let bundle = FullBundle {
		metadata: metadata.clone(),
		dep_info: DepInfo::default(),
		module_sources,
		modules,
	};

	// the same encoding `explorer_server_core::write_full_bundle` produces,
	// inlined because that helper resolves paths against the process cwd
	let meta = metadata.encode_proto();
	fs::write(
		build_dir.join(METADATA_FILE_NAME),
		zstd::encode_all(&*meta, 0).unwrap(),
	)
	.unwrap();
	let data = bundle.encode_proto();
	fs::write(
		build_dir.join(DATA_FILE_NAME),
		zstd::encode_all(&*data, 10).unwrap(),
	)
	.unwrap();
}

/// Starts a cache server and returns the URI `explorer_server` should use.
async fn start_cache(
	flavor: Flavor,
	transport: Transport,
	dir: &Path,
) -> (Reaper, String) {
	let bin = flavor.server_bin();
	let mut cmd = Command::new(bin);
	cmd.current_dir(dir)
		.args(["--dir", "."])
		.args(["--save", ""])
		.args(["--appendonly", "no"]);

	let uri = match transport {
		Transport::Tcp => {
			let port = free_port();
			cmd.args(["--port", &port.to_string()])
				.args(["--bind", "127.0.0.1"]);
			format!("redis://127.0.0.1:{port}")
		}
		Transport::Unix => {
			let sock = dir.join("cache.sock");
			cmd.args(["--port", "0"])
				.arg("--unixsocket")
				.arg(&sock);
			format!("unix://{}", sock.display())
		}
	};

	let child = cmd
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.unwrap_or_else(|e| {
			panic!("could not run {bin}: {e}. the nix devshell provides it")
		});
	let reaper = Reaper(child);

	let client = redis::Client::open(uri.as_str()).unwrap();
	let deadline = Instant::now() + CACHE_START_TIMEOUT;
	loop {
		if client
			.get_multiplexed_async_connection()
			.await
			.is_ok()
		{
			break;
		}
		assert!(
			Instant::now() < deadline,
			"{bin} never accepted connections"
		);
		sleep(Duration::from_millis(50)).await;
	}

	(reaper, uri)
}

/// Starts `explorer_server` against `dir` and returns its base URL plus the
/// path its logs are going to.
async fn start_server(dir: &Path, uri: &str) -> (Reaper, String, PathBuf) {
	let port = free_port();
	let log_path = dir.join("server.log");
	let log = fs::File::create(&log_path).unwrap();

	let child = Command::new(env!("CARGO_BIN_EXE_explorer_server"))
		// the server resolves `builds` against its cwd
		.current_dir(dir)
		.args(["--host", "127.0.0.1"])
		.args(["--port", &port.to_string()])
		.args(["--redis-uri", uri])
		.env("RUST_LOG", "info")
		.stdout(Stdio::from(log.try_clone().unwrap()))
		.stderr(Stdio::from(log))
		.spawn()
		.expect("could not run explorer_server");
	let reaper = Reaper(child);

	let base = format!("http://127.0.0.1:{port}");
	let deadline = Instant::now() + SERVER_START_TIMEOUT;
	loop {
		if reqwest::get(format!("{base}/version"))
			.await
			.is_ok_and(|r| r.status().is_success())
		{
			break;
		}
		assert!(
			Instant::now() < deadline,
			"explorer_server never came up: {}",
			fs::read_to_string(&log_path).unwrap_or_default()
		);
		sleep(Duration::from_millis(50)).await;
	}

	(reaper, base, log_path)
}

fn logged(log_path: &Path, needle: &str) -> bool {
	fs::read_to_string(log_path)
		.unwrap_or_default()
		.contains(needle)
}

async fn wait_for_log(log_path: &Path, needle: &str) {
	let deadline = Instant::now() + CACHE_WRITE_TIMEOUT;
	while !logged(log_path, needle) {
		assert!(
			Instant::now() < deadline,
			"never logged {needle:?}:\n{}",
			fs::read_to_string(log_path).unwrap_or_default()
		);
		sleep(Duration::from_millis(50)).await;
	}
}

async fn get_archive(url: &str) -> (u16, Option<String>, Vec<u8>) {
	let res = reqwest::get(url)
		.await
		.expect("archive request failed");
	let status = res.status().as_u16();
	let content_type = res
		.headers()
		.get(reqwest::header::CONTENT_TYPE)
		.and_then(|v| v.to_str().ok())
		.map(ToOwned::to_owned);
	let body = res
		.bytes()
		.await
		.expect("could not read archive body")
		.to_vec();
	(status, content_type, body)
}

async fn run_cell(flavor: Flavor, transport: Transport) {
	// TempDir sits directly under /tmp, which keeps the unix socket path short
	let tmp = TempDir::new().unwrap();
	let root = tmp.path();
	write_fixture(root);

	let (cache, uri) = start_cache(flavor, transport, root).await;
	let mut verify = redis::Client::open(uri.as_str())
		.unwrap()
		.get_multiplexed_async_connection()
		.await
		.unwrap();

	let (_server, base, log_path) = start_server(root, &uri).await;

	// connected for real, rather than silently running cache-less
	assert!(
		logged(&log_path, "Connected to redis cache"),
		"server did not connect to {uri}"
	);

	let url = format!("{base}/build/archive/{BUILD_HASH}.7z");
	let key = format!("{KEY_PREFIX}{BUILD_HASH}");

	// 1. cache miss: built from disk and served
	let (status, content_type, miss_body) = get_archive(&url).await;
	assert_eq!(status, 200, "miss should have served the archive");
	assert_eq!(content_type.as_deref(), Some(SEVENZ_MIME));
	assert!(!miss_body.is_empty(), "miss served an empty archive");
	assert!(
		!logged(&log_path, "serving cached archive"),
		"an empty cache should not have produced a hit"
	);

	// 2. the archive lands in the cache, from the detached task
	let deadline = Instant::now() + CACHE_WRITE_TIMEOUT;
	loop {
		let cached: bool = verify.exists(&key).await.unwrap();
		if cached {
			break;
		}
		assert!(
			Instant::now() < deadline,
			"archive never made it into the cache under {key}"
		);
		sleep(Duration::from_millis(50)).await;
	}
	let cached_len: usize = verify.strlen(&key).await.unwrap();
	assert_eq!(
		cached_len,
		miss_body.len(),
		"cached archive is a different size to the one served"
	);
	let ttl: i64 = verify.ttl(&key).await.unwrap();
	assert!(
		(1..=ARCHIVE_TTL).contains(&ttl),
		"ttl {ttl} outside 1..={ARCHIVE_TTL}"
	);

	// 3. cache hit: the same bytes, straight off the cache
	let (status, content_type, hit_body) = get_archive(&url).await;
	assert_eq!(status, 200, "hit should have served the archive");
	assert_eq!(content_type.as_deref(), Some(SEVENZ_MIME));
	assert_eq!(
		hit_body, miss_body,
		"hit served different bytes to the miss"
	);
	assert!(
		logged(&log_path, "serving cached archive"),
		"second request rebuilt the archive instead of using the cache"
	);

	// 4. cache down: degrades to rebuilding rather than failing
	drop(cache);
	let (status, _, down_body) = get_archive(&url).await;
	assert_eq!(status, 200, "a dead cache must not fail the request");
	// deliberately not compared byte-for-byte against the others:
	// multithreaded LZMA2 block splitting makes make_archive
	// non-deterministic, so a rebuild legitimately differs
	assert!(!down_body.is_empty(), "rebuild served an empty archive");
	assert!(
		logged(&log_path, "failed to read archive from cache"),
		"a failed cache read should warn"
	);
	// the failing write is detached too, so give it a moment to land
	wait_for_log(&log_path, "failed to cache archive").await;
}

#[tokio::test]
async fn redis_over_tcp() {
	run_cell(Flavor::Redis, Transport::Tcp).await;
}

#[tokio::test]
async fn redis_over_unix_socket() {
	run_cell(Flavor::Redis, Transport::Unix).await;
}

#[tokio::test]
async fn valkey_over_tcp() {
	run_cell(Flavor::Valkey, Transport::Tcp).await;
}

#[tokio::test]
async fn valkey_over_unix_socket() {
	run_cell(Flavor::Valkey, Transport::Unix).await;
}
