//! Scraper for the Discord mobile app.
mod model;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use bytes::Bytes;
use play_store_api::app::Options;
use regress::Regex;
use reqwest::{Client, Request, redirect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MobileVersion {
	pub major: u16,
	pub minor: u8,
	pub channel: Channel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
	Stable,
}

const APP_ID: &str = "com.discord";

static VERSION_REGEX: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"^(\d+?)\.(\d+) - (Stable)$").unwrap());

pub async fn get_latest_version() -> Result<MobileVersion> {
	let opts = Options {
		app_id: String::from(APP_ID),
		..Default::default()
	};
	let req = Request::try_from(&opts).context("Failed to create request")?;
	let resp = Client::builder()
		.redirect(redirect::Policy::limited(10))
		.build()
		.context("Failed to build reqwest client")?
		.execute(req)
		.await
		.context("Failed to send request")?;
	let parsed_html = opts
		.handle_response(resp)
		.await
		.context("Failed to handle response")?;
	let info = opts
		.handle_parsed_html(&parsed_html)
		.context("Failed to handle parsed HTML")?;
	let version_str = info.version;
	let captures = VERSION_REGEX
		.find(&version_str)
		.with_context(|| format!("invalid version string {version_str:?}"))?;
	let major = captures.group(1).unwrap();
	let minor = captures.group(2).unwrap();
	let channel = captures.group(3).unwrap();
	let major = version_str[major.clone()]
		.parse()
		.with_context(|| {
			format!(
				"Failed to parse major version as u16 {:?}",
				&version_str[major]
			)
		})?;
	let minor = version_str[minor.clone()]
		.parse()
		.with_context(|| {
			format!(
				"Failed to parse minor version as u8 {:?}",
				&version_str[minor]
			)
		})?;
	let channel = match &version_str[channel.clone()] {
		"Stable" => Channel::Stable,
		_ => anyhow::bail!("Unknown channel {:?}", &version_str[channel]),
	};
	Ok(MobileVersion {
		major,
		minor,
		channel,
	})
}

pub async fn fetch_manifest(version: MobileVersion) -> Result<model::Manifest> {
	let url = format!(
		"https://discord.com/android/{major}.{minor}/manifest.json",
		major = version.major,
		minor = version.minor
	);
	reqwest::get(&url)
		.await
		.context("Failed to fetch manifest")?
		.json()
		.await
		.context("Failed to parse manifest JSON")
}

pub async fn fetch_main_bundle(commit: &str) -> Result<Bytes> {
	reqwest::get(format!(
		"https://discord.com/assets/android/{commit}/app/src/main/assets/index.android.bundle"
	))
		.await
		.context("Failed to fetch main bundle")?
		.bytes()
		.await
		.context("Failed to read main bundle bytes")
}
