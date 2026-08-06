use std::{iter, time::Duration};

use anyhow::{Context as _, Result};
use macros::command;
use serenity::all::{Context, CreateEmbed, CreateEmbedAuthor, Timestamp};
use tokio::io::AsyncWriteExt;

use crate::{
	fw::{CommandCtx, CommandFramework},
	util::mktemp,
};

mod model;

async fn get_random_vnc(fw: &CommandFramework) -> Result<model::Response> {
	let r = fw
		.http
		.get("https://computernewb.com/vncresolver/api/v1/random")
		.timeout(Duration::from_secs(4))
		.send()
		.await
		.context("Failed to send request for random vnc server")?
		.bytes()
		.await
		.context("Failed to read response for random vnc server")?;
	match serde_json::from_slice(&r) {
		Ok(r) => Ok(r),
		Err(e) => {
			const MSG: &str = "Failed to parse response from random vnc server";
			let (mut tmp_file, path) = mktemp("random_vnc_response_", ".json")
				.await
				.context(
					"Failed to make temp file for random vnc server response",
				)?;
			tmp_file
				.write_all(&r)
				.await
				.context("Failed to write api response")?;
			Err(anyhow::Error::from(e).context(format!(
				"{MSG}. Wrote response to {}",
				path.display()
			)))
		}
	}
}

/// Send an image of a random unsecured VNC server.
#[command]
async fn random_vnc(
	c: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx
		.defer(c)
		.await
		.context("Failed to defer randomvnc")?;
	let data = get_random_vnc(fw).await?;
	let mut e = CreateEmbed::new()
		.url(format!(
			"https://computernewb.com/vncresolver/embed?id={id}",
			id = data.id
		))
		.title(data.ip_address)
		.field("Who", data.asn, true)
		.timestamp(
			Timestamp::from_unix_timestamp(data.scanned_on).with_context(
				|| {
					format!(
						"api returned invalid timestamp: {}",
						data.scanned_on
					)
				},
			)?,
		);
	if !data.desktop_name.is_empty() {
		e = e.field("How", data.desktop_name, true);
	}
	if !data.password.is_empty() {
		e = e.field("Password", data.password, true);
	}
	e = e
		.image(
			format!(
				"https://computernewb.com/vncresolver/api/v1/screenshot/{id}",
				id = data.id
			),
			None,
		)
		.author(
			CreateEmbedAuthor::new("VNC Resolver")
				.url("https://computernewb.com/vncresolver/")
				.icon_url("https://computernewb.com/favicon.ico"),
		);
	cctx.followup_embed(c, iter::once(e))
		.await
		.context("Failed to send random vnc embed")?;
	Ok(())
}
