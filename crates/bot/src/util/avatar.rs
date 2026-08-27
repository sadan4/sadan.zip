use anyhow::{Context as _, Result};

use crate::{
	fw::CommandFramework,
	util::{Image, ImageFormat},
};

pub async fn download_avatar(
	url: &str,
	fw: &CommandFramework,
) -> Result<Image> {
	let res = fw
		.http
		.get(url)
		.send()
		.await
		.context("Failed to send request to download avatar")?;
	let res = res
		.error_for_status()
		.context("Failed to download avatar")?;
	let content_type = res
		.headers()
		.get("content-type")
		.context("Failed to get content-type header")?
		.clone();
	let bytes = res
		.bytes()
		.await
		.context("Failed to read avatar bytes")?;
	let format = ImageFormat::from_content_type(content_type.as_bytes())
		.with_context(|| {
			format!(
				"Unsupported content-type: {}",
				String::from_utf8_lossy(content_type.as_bytes())
			)
		})?;
	Ok(Image { bytes, format })
}
