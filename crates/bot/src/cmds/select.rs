use anyhow::{Context as _, Result};
use macros::command;
use serenity::{
	all::{
		CommandDataOption,
		CommandDataOptionValue,
		CommandOptionType,
		Context,
		CreateAttachment,
	},
	small_fixed_array::FixedString,
};
use tracing::warn;
use url::Url;

use crate::{
	fw::{CommandCtx, CommandFramework, SlashOption, SlashSchema},
	util::{Image, ImageFormat},
};

use clap::Parser;

#[derive(Parser)]
struct SelectArgs {
	/// The image to select
	/// If not present, will attempt to find attachments.
	/// If no attachments are present, will attempt to find an image in the message context (eg: reply)
	#[arg()]
	image: Option<String>,
}

impl SlashSchema for SelectArgs {
	fn slash_options() -> Vec<crate::fw::SlashOption> {
		// FIXME: support url option
		vec![SlashOption {
			name: "image",
			kind: CommandOptionType::Attachment,
			choices: Vec::new(),
		}]
	}
}

/// Short description of the command.
#[command]
#[arg_parser = SelectArgs]
#[slash_args]
async fn select(
	args: SelectArgs,
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	cctx.defer(ctx)
		.await
		.context("Failed to defer interaction")?;
	match cctx {
		CommandCtx::Prefix { msg } => {
			todo!()
		}
		CommandCtx::Application { interaction } => {
			let msg = 'm: {
				if let Some(CommandDataOption {
					value: CommandDataOptionValue::Attachment(id),
					name,
					..
				}) = interaction.data.options.first()
				{
					debug_assert_eq!(name, "image", "name mismatch");
					let attachment = interaction
					.data
					.resolved
					.attachments
					.get(id)
					.context("Attachment id not found in resolved attachments. This should never happen.")?;
					let Some(fmt) = attachment
						.content_type
						.as_deref()
						.and_then(|s| {
							ImageFormat::from_content_type(s.as_bytes())
						})
					else {
						break 'm format!(
							"Attachment content type {:?} is not a supported image format",
							attachment.content_type
						);
					};
					let url = if is_url_trusted(&attachment.url) {
						attachment.url.as_str()
					} else {
						&attachment.proxy_url
					};
					fw.image_cache
						.launch_dl_for_user(
							async {
								let res =
									fw.http.get(url).send().await.context(
										"Failed to send request to attachment URL",
									)?;
								let content_type = res
									.headers()
									.get("content-type")
									.and_then(|v| {
										ImageFormat::from_content_type(
											v.as_bytes(),
										)
									});
								if content_type != Some(fmt) {
									warn!(
										"Attachment content type {:?} does not match expected format {:?}",
										content_type, fmt
									);
								}
								let bytes = res.bytes().await.context(
									"Failed to read bytes from attachment response",
								)?;
								let image = Image { bytes, format: fmt };
								anyhow::Ok(image)
							},
							interaction.user.id,
							Some(attachment.proxy_url.to_string()),
						)
						.await?;
					format!("Selected attachment: {}", attachment.filename)
				} else {
					String::from("No attachment provided")
				}
			};
			cctx.followup_text(ctx, msg)
				.await
				.context("Failed to send followup message")?;
		}
	}
	Ok(())
}

/// Show the currently selected image
#[command]
async fn show(
	ctx: &Context,
	cctx: &CommandCtx<'_>,
	fw: &CommandFramework,
) -> Result<()> {
	let image = fw
		.image_cache
		.get_user_entry(cctx.author().id);
	match image {
		Some(entry) => match entry.get() {
			Some(image) => {
				let file = CreateAttachment::bytes(
					image.bytes.clone(),
					format!("selected_image{}", image.format.ext()),
				);
				cctx.reply_file(ctx, file, None)
					.await
					.context("Failed to send reply with selected image")?;
			}
			None => {
				cctx.reply(ctx, "Image is still downloading, please wait.")
					.await
					.context("Failed to send reply")?;
			}
		},
		None => {
			cctx.reply(ctx, "No image selected.")
				.await
				.context("Failed to send reply")?;
		}
	}
	Ok(())
}

/// Given a name like `foo.example.com` or `foo.bar.example.com` or `example.com`,
/// return the root domain name, e.g. `example.com`.
///
/// This is a naive last-two-labels split; it does not consult the public
/// suffix list, so `foo.example.co.uk` yields `co.uk`.
pub fn root_domain_name(name: &str) -> &str {
	let name = name.strip_suffix('.').unwrap_or(name);
	let mut dots = name.rmatch_indices('.');
	// the separator before the tld
	if dots.next().is_none() {
		return name;
	}
	match dots.next() {
		Some((idx, _)) => &name[idx + 1..],
		None => name,
	}
}

const TRUSTED_DOMAINS: &[&str] = &["cdn.discordapp.com"];

pub fn is_url_trusted(url: &str) -> bool {
	let Ok(parsed) = Url::parse(url) else {
		warn!("Failed to parse URL from discord: {}", url);
		return false;
	};
	if TRUSTED_DOMAINS
		.contains(&root_domain_name(parsed.host_str().unwrap_or("")))
	{
		true
	} else {
		warn!("Untrusted URL from discord: {}", url);
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn root_domain_name_strips_subdomains() {
		assert_eq!(root_domain_name("example.com"), "example.com");
		assert_eq!(root_domain_name("foo.example.com"), "example.com");
		assert_eq!(root_domain_name("foo.bar.example.com"), "example.com");
	}

	#[test]
	fn root_domain_name_edge_cases() {
		assert_eq!(root_domain_name("localhost"), "localhost");
		assert_eq!(root_domain_name(""), "");
		assert_eq!(root_domain_name("foo.example.com."), "example.com");
		assert_eq!(root_domain_name(".com"), ".com");
	}
}
