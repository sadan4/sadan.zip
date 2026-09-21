//! Locating the `vencord-companion.nvim` checkout.
//!
//! The Neovim plugin lives in its own repository rather than in this
//! workspace, so the targets that write into it have to be told where it is.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args;

/// The environment variable naming the plugin checkout.
pub const DIR_ENV: &str = "VENCORD_COMPANION_NVIM_DIR";

/// The `--plugin-dir` flag, shared by every target that writes into the
/// Neovim plugin.
#[derive(Args, Clone, Debug)]
pub struct PluginDir {
	/// Path to the `vencord-companion.nvim` checkout. Defaults to
	/// `$VENCORD_COMPANION_NVIM_DIR`.
	#[arg(long = "plugin-dir")]
	plugin_dir: Option<PathBuf>,
}

impl PluginDir {
	/// Resolve the checkout, preferring the flag over the environment.
	///
	/// The directory has to already exist: creating it would silently produce
	/// a half a plugin somewhere the user did not mean, and a typo in either
	/// the flag or the variable is far likelier than a missing checkout.
	pub fn resolve(&self) -> Result<PathBuf> {
		let dir = match (&self.plugin_dir, std::env::var_os(DIR_ENV)) {
			(Some(dir), _) => dir.clone(),
			(None, Some(dir)) if !dir.is_empty() => PathBuf::from(dir),
			_ => bail!(
				"the vencord-companion.nvim checkout is not configured: pass \
				 --plugin-dir <path> or set ${DIR_ENV}"
			),
		};
		let dir = dir.canonicalize().with_context(|| {
			format!("{} is not a readable directory", dir.display())
		})?;
		if !dir
			.join("lua")
			.join("vencord-companion")
			.is_dir()
		{
			bail!(
				"{} does not look like a vencord-companion.nvim checkout: no \
				 lua/vencord-companion directory",
				dir.display()
			);
		}
		Ok(dir)
	}
}
