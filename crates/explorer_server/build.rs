use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, anyhow, bail};

trait CommandExt {
    fn run(&mut self) -> Result<()>;
    fn search(program: impl AsRef<OsStr>) -> Result<Self>
    where
        Self: Sized;
}

impl CommandExt for process::Command {
    fn run(&mut self) -> Result<()> {
        dbg!(&self);
        let status = self
            .status()
            .with_context(|| format!("Failed to execute command {self:?}"))?;
        if !status.success() {
            Err(anyhow!("Command failed with status {status}"))
                .with_context(|| format!("Failed to execute command {self:?}"))?;
        }
        Ok(())
    }
    fn search(program: impl AsRef<OsStr>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new(resolve_program_in_path(program)?))
    }
}

#[cfg(not(windows))]
const PATH_DELIMITER: char = ':';

#[cfg(windows)]
const PATH_DELIMITER: char = ';';

fn resolve_program_in_path(file: impl AsRef<OsStr>) -> Result<PathBuf> {
    let file = Path::new(&file);
    if file.is_absolute() {
        if !file.exists() {
            bail!("Program {} does not exist", file.display());
        }
        eprintln!("absolute path {} exists", file.display());
        return Ok(file.to_owned());
    }

    let env_path = env::var("PATH")?;
    let env_path = env_path.split(PATH_DELIMITER);
    if cfg!(windows) && file.extension().is_none() {
        let search_path_refs = env_path.map(Path::new);
        let path_exts = env::var("PATHEXT")?;
        let paths = path_exts
            .split(PATH_DELIMITER)
            .map(|ext| ext.trim_start_matches('.'))
            .map(|ext| file.with_extension(ext))
            .flat_map(|file_name| {
                search_path_refs
                    .clone()
                    .map(move |dir| dir.join(&file_name))
            });
        for path in paths {
            dbg!(&path);
            if path.exists() {
                eprintln!("found {} in PATH at {}", file.display(), path.display());
                return Ok(path);
            }
        }
    } else {
        let paths = env_path.map(Path::new).map(|dir| dir.join(file));
        for path in paths {
            if path.exists() {
                eprintln!("found {} in PATH at {}", file.display(), path.display());
                return Ok(path);
            }
        }
    }

    bail!("Could not find {} in PATH", file.display());
}

fn main() -> Result<()> {
    // get workspace root
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?)
        .join("../..")
        .canonicalize()?;
    let server_src_dir = root.join("server");
    let server_native_out_dir = server_src_dir.join("native");
    let server_rollup_file = server_src_dir.join("rollup.config.ts");
    // build the native node module the js side of the server needs first
    process::Command::search("npx")?
        .current_dir(&root)
        .arg("napi")
        .arg("build")
        .arg("-p")
        .arg("explorer_writer")
        .arg("-o")
        .arg(server_native_out_dir)
        .run()?;
    // we need to build the js code that we will run via node
    process::Command::search("npx")?
        .current_dir(&root)
        .arg("rollup")
        .arg("-c")
        .arg(server_rollup_file)
        .run()?;
    bail!("Testing");
}
