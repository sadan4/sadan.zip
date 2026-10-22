use std::{
	collections::HashMap,
	fs,
	net::TcpListener,
	path::Path,
	process::{Child, Command, Stdio},
	time::Duration,
};

use explorer_server_core::{
	DATA_DELTA_FILE_NAME,
	DATA_FILE_NAME,
	DATA_ZSTD_LEVEL,
	METADATA_FILE_NAME,
	METADATA_ZSTD_LEVEL,
	write_mpk_zst_atomic,
};
use explorer_types::{BundleMetadata, Channel, DepInfo, FullBundle, ModuleId};
use tempfile::TempDir;
use tokio::time::{Instant, sleep};

/// canary seen before any stable build, so it has no base
const OLD_CANARY_HASH: &str = "0000000000000000000000000000000000000001";
const STABLE_HASH: &str = "0000000000000000000000000000000000000002";
const CANARY_HASH: &str = "0000000000000000000000000000000000000003";

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(60);

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

/// Modules that look enough like real ones that most of them are shared
/// between builds, with `changed` of them differing
fn modules(changed: u32) -> HashMap<ModuleId, String> {
	(0..2000u32)
		.map(|id| {
			let rev = u8::from(id < changed);
			let src = format!(
				"{id}:function(e,t,n){{n.d(t,{{Z:()=>r{rev}}});var r{rev}=\
				 n({});function a(){{return r{rev}.useState({id})}}}},",
				id * 7919 % 100_003
			)
			.repeat(16);
			(ModuleId::from(id), src)
		})
		.collect()
}

fn bundle(
	hash: &str,
	first_seen: u64,
	channel: Channel,
	changed: u32,
) -> FullBundle {
	let modules = modules(changed);
	let module_sources = modules
		.keys()
		.map(|&id| (format!("{id}.js"), vec![id]))
		.collect();
	FullBundle {
		metadata: BundleMetadata {
			build_hash: hash.to_owned(),
			build_number: 1,
			first_seen,
			entry_point: None,
			channel: vec![channel],
		},
		dep_info: DepInfo::default(),
		module_sources,
		modules,
		env_var_text: String::new(),
		experiments: Vec::new(),
		css_chunks: HashMap::new(),
	}
}

/// Writes `bundle` in full, the way builds were stored before deltas
fn write_fixture(root: &Path, bundle: &FullBundle) {
	let dir = root
		.join("builds")
		.join(&bundle.metadata.build_hash);
	fs::create_dir_all(&dir).unwrap();
	write_mpk_zst_atomic(
		&dir.join(METADATA_FILE_NAME),
		&bundle.metadata,
		METADATA_ZSTD_LEVEL,
	)
	.unwrap();
	write_mpk_zst_atomic(&dir.join(DATA_FILE_NAME), bundle, DATA_ZSTD_LEVEL)
		.unwrap();
}

async fn start_server(dir: &Path) -> (Reaper, String) {
	let port = free_port();
	let log_path = dir.join("server.log");
	let log = fs::File::create(&log_path).unwrap();

	let child = Command::new(env!("CARGO_BIN_EXE_explorer_server"))
		// the server resolves `builds` against its cwd
		.current_dir(dir)
		.args(["--host", "127.0.0.1"])
		.args(["--port", &port.to_string()])
		.args(["--scrape-stable", "false"])
		.args(["--scrape-canary", "false"])
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

	(reaper, base)
}

async fn get_full(base: &str, hash: &str) -> FullBundle {
	let res = reqwest::get(format!("{base}/build/{hash}/full"))
		.await
		.unwrap();
	assert_eq!(res.status().as_u16(), 200, "/full for {hash}");
	let body = res.bytes().await.unwrap();
	let raw = zstd::decode_all(body.as_ref()).unwrap();
	rmp_serde::from_slice(&raw).unwrap()
}

#[tokio::test]
async fn canary_migrated_to_delta_and_served() {
	let tmp = TempDir::new().unwrap();
	let root = tmp.path();
	let builds = root.join("builds");

	let old_canary = bundle(OLD_CANARY_HASH, 1_000, Channel::Canary, 0);
	let stable = bundle(STABLE_HASH, 2_000, Channel::Stable, 0);
	let canary = bundle(CANARY_HASH, 3_000, Channel::Canary, 40);
	for b in [&old_canary, &stable, &canary] {
		write_fixture(root, b);
	}
	// the last version without deltas, so the server runs the migration
	fs::write(builds.join(".ver"), "8").unwrap();

	let (_server, base) = start_server(root).await;

	assert_eq!(fs::read_to_string(builds.join(".ver")).unwrap(), "9");
	let canary_dir = builds.join(CANARY_HASH);
	assert!(
		canary_dir
			.join(DATA_DELTA_FILE_NAME)
			.is_file()
	);
	assert!(!canary_dir.join(DATA_FILE_NAME).exists());
	// nothing stable came before it to be a base
	let old_dir = builds.join(OLD_CANARY_HASH);
	assert!(old_dir.join(DATA_FILE_NAME).is_file());
	assert!(
		!old_dir
			.join(DATA_DELTA_FILE_NAME)
			.exists()
	);
	assert!(
		builds
			.join(STABLE_HASH)
			.join(DATA_FILE_NAME)
			.is_file()
	);

	let delta_len = fs::metadata(canary_dir.join(DATA_DELTA_FILE_NAME))
		.unwrap()
		.len();
	let full_len = fs::metadata(old_dir.join(DATA_FILE_NAME))
		.unwrap()
		.len();
	assert!(
		delta_len * 4 < full_len,
		"delta is {delta_len} bytes, full is {full_len}"
	);

	for (hash, expected) in [
		(CANARY_HASH, &canary),
		(STABLE_HASH, &stable),
		(OLD_CANARY_HASH, &old_canary),
	] {
		let served = get_full(&base, hash).await;
		assert_eq!(served.metadata.build_hash, expected.metadata.build_hash);
		assert_eq!(served.modules, expected.modules, "modules of {hash}");
		assert_eq!(
			served.module_sources, expected.module_sources,
			"module sources of {hash}"
		);
	}

	let res = reqwest::get(format!("{base}/build/archive/{CANARY_HASH}.7z"))
		.await
		.unwrap();
	assert_eq!(res.status().as_u16(), 200, "archive of a delta build");
	assert!(!res.bytes().await.unwrap().is_empty());
}
