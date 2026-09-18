use std::{borrow::Cow, path::Path, str::FromStr as _};

use anyhow::{Context, Result, ensure};
use percent_encoding::AsciiSet;
use tower_lsp_server::ls_types::Uri;

pub const ASCII_SET: AsciiSet =
	// RFC3986 allows only alphanumeric characters, `-`, `.`, `_`, and `~` in the path.
	percent_encoding::NON_ALPHANUMERIC
		.remove(b'-')
		.remove(b'.')
		.remove(b'_')
		.remove(b'~')
		// we do not want path separators to be percent-encoded
		.remove(b'/');

/// The percent-encoded URI path component for `path`.
///
/// Implementation from [`tower_lsp_server::ls_types::Uri::from_file_path`]
fn encode_path(path: &Path) -> Result<String> {
	ensure!(
		path.is_absolute(),
		"Path must be absolute: {}",
		path.display()
	);

	#[cfg(windows)]
	// we want to parse a triple-slash path for Windows paths
	// it's a shorthand for `file://localhost/C:/Windows` with the `localhost` omitted.
	// We encode the driver Letter `C:` as well. LSP Specification allows it.
	// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#uri
	let raw_path = format!(
		"/{}",
		capitalize_drive_letter(
			&path
				.to_string_lossy()
				.replace('\\', "/")
		)
	);

	#[cfg(not(windows))]
	let raw_path = path.to_string_lossy();

	Ok(
		percent_encoding::utf8_percent_encode(&raw_path, &ASCII_SET)
			.to_string(),
	)
}

#[cfg(windows)]
fn capitalize_drive_letter(path: &str) -> Cow<'_, str> {
	let mut chars = path.chars();
	match (chars.next(), chars.next()) {
		(Some(drive @ 'a'..='z'), Some(':')) => {
			Cow::Owned(format!("{}{}", drive.to_ascii_uppercase(), &path[1..]))
		}
		_ => Cow::Borrowed(path),
	}
}

/// Implementation from [`tower_lsp_server::ls_types::Uri::from_file_path`]
pub fn from_path<A: AsRef<Path>>(path: A) -> Result<Uri> {
	// `file:` always carries an authority, empty though it is here
	Ok(Uri::from_str(&format!(
		"file://{}",
		encode_path(path.as_ref())?
	))?)
}

/// A URI for `path` under a scheme other than `file`.
///
/// Has no authority component
pub fn from_path_with_scheme<A: AsRef<Path>>(
	scheme: &str,
	path: A,
) -> Result<Uri> {
	Ok(Uri::from_str(&format!(
		"{scheme}:{}",
		encode_path(path.as_ref())?
	))?)
}

pub fn from_url(url: &url::Url) -> Uri {
	// lsp_types is fucking stupid and doesn't implement From<fluent_uri::Uri> for Uri, so we have to allocate again
	Uri::from_str(url.as_str()).unwrap()
}

pub fn to_path(uri: &Uri) -> Result<Cow<'_, Path>> {
	let scheme_str = uri.scheme().as_str();
	ensure!(
		scheme_str == "file",
		"URI scheme must be 'file', got '{scheme_str}'"
	);
	uri.to_file_path().context("Empty path")
}
