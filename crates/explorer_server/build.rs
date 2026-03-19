use std::env;

use anyhow::{Context, Result};
use xxhash_rust::xxh3::xxh3_64;

fn set_env(key: &str, value: &str) {
    println!("cargo:rustc-env={key}={value}");
}

fn set_build_seed(seed: u64) {
    let seed: String = format!("{seed:x}");
    println!("Setting build seed to {seed}");
    set_env(BUILD_SEED_KEY, &seed);
}

const BUILD_SEED_KEY: &str = "EXPLORER_BUILD_SEED";

fn main() -> Result<()> {
    println!("cargo:rerun-if-env-changed=BUILD_SEED");
    println!("cargo:rerun-if-changed=build.rs");
    match env::var("BUILD_SEED") {
        Ok(seed) => {
            set_build_seed(xxh3_64(seed.as_bytes()));
        }
        Err(env::VarError::NotPresent) => {
            let seed = rand::random::<u64>();
            set_build_seed(seed);
        }
        Err(e) => {
            return Err(e).context("Failed to read BUILD_SEED env var");
        }
    }
    Ok(())
}
