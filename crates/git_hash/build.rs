use std::process::Command;

fn set_build_env_var(key: impl AsRef<str>, value: impl AsRef<str>) {
	let key = key.as_ref();
	let value = value.as_ref();
	println!("cargo:rustc-env={key}={value}");
}

fn main() {
	println!("cargo:rerun-if-env-changed=GIT_HASH");

	// When building via Nix, the source tree is filtered and does not
	// contain `.git`, so `git rev-parse` isn't available. In that case the
	// hash is passed in explicitly via the `GIT_HASH` env var.
	#[allow(clippy::single_match_else)]
	let hash = match std::env::var("GIT_HASH") {
		Ok(hash) => hash,
		Err(_) => {
			let out = Command::new("git")
				.arg("rev-parse")
				.arg("HEAD")
				.output()
				.expect("Failed to exec git rev-parse HEAD");
			if out.status.code() != Some(0) {
				panic!(
					"git rev-parse HEAD failed with code {:?}: {}",
					out.status.code(),
					String::from_utf8_lossy(&out.stderr)
				);
			}
			String::from_utf8(out.stdout).expect("output is not utf8")
		}
	};
	let hash = hash.trim();

	set_build_env_var("BUILD_GIT_HASH_SHORT", &hash[..7]);
	set_build_env_var("BUILD_GIT_HASH", hash);
}
