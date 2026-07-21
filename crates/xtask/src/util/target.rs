//! Mapping between VS Code platform identifiers (used by `vsce`/`ovsx` for
//! platform-specific extensions) and the Rust target triples used to
//! cross-compile the bundled `companion_lsp` binary.

use std::{fmt, str::FromStr};

use anyhow::{Result, bail};

/// A VS Code platform-specific extension target.
///
/// See <https://code.visualstudio.com/api/working-with-extensions/publishing-extension#platform-specific-extensions>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtensionTarget {
	/// The VS Code platform identifier, e.g. `linux-x64`. Passed verbatim to
	/// `vsce package --target` / `ovsx publish --target`.
	vscode: &'static str,
	/// The Rust target triple, e.g. `x86_64-unknown-linux-gnu`. Passed to
	/// `cargo build --target`.
	triple: &'static str,
	/// Whether the platform is Windows (the staged binary needs a `.exe`
	/// extension).
	windows: bool,
}

impl ExtensionTarget {
	/// Every VS Code platform we ship a native `companion_lsp` binary for.
	const ALL: &'static [Self] = &[
		Self {
			vscode: "linux-x64",
			triple: "x86_64-unknown-linux-gnu",
			windows: false,
		},
		Self {
			vscode: "linux-arm64",
			triple: "aarch64-unknown-linux-gnu",
			windows: false,
		},
		Self {
			vscode: "win32-x64",
			triple: "x86_64-pc-windows-msvc",
			windows: true,
		},
		Self {
			vscode: "win32-arm64",
			triple: "aarch64-pc-windows-msvc",
			windows: true,
		},
		Self {
			vscode: "darwin-x64",
			triple: "x86_64-apple-darwin",
			windows: false,
		},
		Self {
			vscode: "darwin-arm64",
			triple: "aarch64-apple-darwin",
			windows: false,
		},
	];

	/// The VS Code platform identifier (e.g. `linux-x64`).
	pub const fn vscode(self) -> &'static str {
		self.vscode
	}

	/// The Rust target triple (e.g. `x86_64-unknown-linux-gnu`).
	pub const fn triple(self) -> &'static str {
		self.triple
	}

	/// The `companion_lsp` binary filename for this target, including the
	/// `.exe` extension on Windows.
	pub const fn bin_name(self) -> &'static str {
		if self.windows {
			"companion_lsp.exe"
		} else {
			"companion_lsp"
		}
	}
}

impl fmt::Display for ExtensionTarget {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.vscode)
	}
}

impl FromStr for ExtensionTarget {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		if let Some(&target) = Self::ALL.iter().find(|t| t.vscode == s) {
			return Ok(target);
		}
		let known = Self::ALL
			.iter()
			.map(|t| t.vscode)
			.collect::<Vec<_>>()
			.join(", ");
		bail!("unknown VS Code target `{s}`; expected one of: {known}")
	}
}
