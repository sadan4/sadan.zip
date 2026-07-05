use std::process::Command;

fn set_build_env_var(key: impl AsRef<str>, value: impl AsRef<str>) {
	let key = key.as_ref();
	let value = value.as_ref();
	println!("cargo:rustc-env={key}={value}");
}

fn main() {
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
	let hash = out.stdout;
	let hash = String::from_utf8(hash).expect("output is not utf8");
	set_build_env_var("BUILD_GIT_HASH_SHORT", &hash[..7]);
	set_build_env_var("BUILD_GIT_HASH", &hash);
}
