mod parse;

use std::debug_assert_matches;

use anyhow::{Context, Result, bail};
use reqwest::{Method, Request, Response, Url};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::{constants, model::IAppItemFullDetail};
pub use parse::ParsedHtml;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Options {
	pub app_id: String,
	pub lang: String,
	pub country: String,
}

impl Options {
	fn validate(&self) -> Result<()> {
		if self.app_id.is_empty() {
			bail!("app_id is required");
		}
		Ok(())
	}
	fn req_url(&self) -> Url {
		let url_str = format!("{}/store/apps/details", constants::BASE_URL);
		let mut url = Url::parse(&url_str).expect("valid url, no user input");
		url.query_pairs_mut()
			.append_pair("id", &self.app_id)
			.append_pair("hl", &self.lang)
			.append_pair("gl", &self.country);
		url
	}
	#[expect(clippy::too_many_lines)]
	pub fn handle_parsed_html(
		&self,
		p: &ParsedHtml,
	) -> Result<IAppItemFullDetail> {
		use IAppItemFullDetail as I;
		let d = &p.script_data;
		let app_id = self.app_id.clone();
		let url = self.req_url();
		let title = d[I::TITLE]
			.as_str()
			.context("title is not a string")?
			.to_string();
		let description = String::from("TODO - parse description");
		let description_html = String::from("TODO - parse description_html");
		let summary = d[I::SUMMARY]
			.as_str()
			.context("summary is not a string")?
			.to_string();
		let installs = d[I::INSTALLS]
			.as_str()
			.context("installs is not a string")?
			.to_string();
		let min_installs = d[I::MIN_INSTALLS]
			.as_f64()
			.with_context(|| {
				format!(
					"min_installs is not a number. Value: {:?}",
					d[I::MIN_INSTALLS]
				)
			})?;
		let max_installs = d[I::MAX_INSTALLS]
			.as_f64()
			.with_context(|| {
				format!(
					"max_installs is not a number. Value: {:?}",
					d[I::MAX_INSTALLS]
				)
			})?;
		let score = d[I::SCORE].as_f64().with_context(|| {
			format!("score is not a f64. Value: {:?}", d[I::SCORE])
		})?;
		let score_text = d[I::SCORE_TEXT]
			.as_str()
			.context("score_text is not a string")?
			.to_string();
		let ratings = d[I::RATINGS]
			.as_f64()
			.with_context(|| {
				format!("ratings is not a f64. Value: {:?}", d[I::RATINGS])
			})?;
		let reviews = d[I::REVIEWS]
			.as_f64()
			.with_context(|| {
				format!("reviews is not a f64. Value: {:?}", d[I::REVIEWS])
			})?;
		// TODO: parse histogram
		let histogram = [0.; 5];
		let price = d[I::PRICE].as_f64().with_context(|| {
			format!("price is not a f64. Value: {:?}", d[I::PRICE])
		})?;
		// FIXME: handle no value vs invalid value
		let original_price = d[I::ORIGINAL_PRICE].as_f64();
		let discount_end_date = d[I::DISCOUNT_END_DATE]
			.as_str()
			.map(ToString::to_string);
		let free = d[I::FREE]
			.as_f64()
			.context("free is not a f64")?
			== 0.;
		let currency = d[I::CURRENCY]
			.as_str()
			.context("currency is not a string")?
			.to_string();
		let price_text = d[I::PRICE_TEXT]
			.as_str()
			.unwrap_or("Free")
			.to_string();
		let available = d[I::AVAILABLE]
			.as_f64()
			.with_context(|| {
				format!("available is not a f64. Value: {:?}", d[I::AVAILABLE])
			})? != 0.;
		let iap_range = d[I::IAP_RANGE]
			.as_str()
			.unwrap_or("")
			.to_string();
		let offers_iap = !iap_range.is_empty();
		let android_version_text = d[I::ANDROID_VERSION_TEXT]
			.as_str()
			.or_else(|| d[I::ANDROID_VERSION_TEXT_FALLBACK].as_str())
			.unwrap_or("VARY")
			.to_string();
		let android_version = android_version_text
			.split(|c: char| !c.is_ascii_digit() && c != '.')
			.next()
			.filter(|s| s.bytes().any(|b| b.is_ascii_digit()))
			.unwrap_or("VARY")
			.to_string();
		let developer = d[I::DEVELOPER]
			.as_str()
			.context("developer is not a string")?
			.to_string();
		let developer_id = String::from("TODO - parse developer_id");
		let developer_internal_id =
			String::from("TODO - parse developer_internal_id");
		let developer_email = d[I::DEVELOPER_EMAIL]
			.as_str()
			.with_context(|| {
				format!(
					"developer_email is not a string. Value: {:?}",
					d[I::DEVELOPER_EMAIL]
				)
			})?
			.to_string();
		let developer_website = d[I::DEVELOPER_WEBSITE]
			.as_str()
			.context("developer_website is not a string")?
			.to_string();
		// TODO: handle optional developer_address vs invalid value
		let developer_address = d[I::DEVELOPER_ADDRESS]
			.as_str()
			.map(ToString::to_string);
		let developer_legal_name = d[I::DEVELOPER_LEGAL_NAME]
			.as_str()
			.context("developer_legal_name is not a string")?
			.to_string();
		let developer_legal_email = d[I::DEVELOPER_LEGAL_EMAIL]
			.as_str()
			.context("developer_legal_email is not a string")?
			.to_string();
		let developer_legal_address = d[I::DEVELOPER_LEGAL_ADDRESS]
			.as_str()
			.context("developer_legal_address is not a string")?
			.to_string();
		let developer_legal_phone_number = d[I::DEVELOPER_LEGAL_PHONE_NUMBER]
			.as_str()
			.context("developer_legal_phone_number is not a string")?
			.to_string();
		let genre = d[I::GENRE]
			.as_str()
			.context("genre is not a string")?
			.to_string();
		let genre_id = d[I::GENRE_ID]
			.as_str()
			.context("genre_id is not a string")?
			.to_string();
		// FIXME: parse categories
		let categories = vec![];
		let icon = d[I::ICON]
			.as_str()
			.context("icon is not a string")?
			.to_string();
		let header_image = d[I::HEADER_IMAGE]
			.as_str()
			.context("header_image is not a string")?
			.to_string();
		// FIXME: parse screenshots
		let screenshots = vec![];
		let video = d[I::VIDEO]
			.as_str()
			.context("video is not a string")?
			.to_string();
		let video_image = d[I::VIDEO_IMAGE]
			.as_str()
			.context("video_image is not a string")?
			.to_string();
		let preview_video = d[I::PREVIEW_VIDEO]
			.as_str()
			.context("preview_video is not a string")?
			.to_string();
		let content_rating = d[I::CONTENT_RATING]
			.as_str()
			.context("content_rating is not a string")?
			.to_string();
		let content_rating_description =
			d[I::CONTENT_RATING_DESCRIPTION].clone();
		debug_assert_matches!(
			content_rating_description,
			serde_json::Value::Null,
			"handle content_rating_description parsing"
		);
		let ad_supported = !d[I::AD_SUPPORTED]
			.as_str()
			.is_none_or(str::is_empty);
		let released = d[I::RELEASED]
			.as_str()
			.context("released is not a string")?
			.to_string();
		// FIXME: parse updated
		let updated = u32::MAX;
		let version = d[I::VERSION]
			.as_str()
			.or_else(|| d[I::VERSION_FALLBACK].as_str())
			.unwrap_or("VARY")
			.to_string();
		let recent_changes = String::from("TODO - parse recent_changes");
		// FIXME: parse comments
		let comments = vec![];
		#[expect(clippy::float_cmp)]
		let preregister = d[I::PREREGISTER]
			.as_f64()
			.with_context(|| {
				format!(
					"preregister is not a bool. Value: {:?}",
					d[I::PREREGISTER]
				)
			})? == 1.;
		// let early_access_enabled = d[I::EARLY_ACCESS_ENABLED]
		// 	.as_u64()
		// 	.with_context(|| {
		// 		format!(
		// 			"early_access_enabled is not a bool. Value: {:?}",
		// 			d[I::EARLY_ACCESS_ENABLED]
		// 		)
		// 	})? != 0;
		let is_available_in_play_pass = &d[I::IS_AVAILABLE_IN_PLAY_PASS];
		debug_assert_matches!(
			is_available_in_play_pass,
			serde_json::Value::Null,
			"handle is_available_in_play_pass parsing"
		);

		Ok(I {
			app_id,
			url,
			title,
			description,
			description_html,
			summary,
			installs,
			min_installs,
			max_installs,
			score,
			score_text,
			ratings,
			reviews,
			histogram,
			price,
			original_price,
			discount_end_date,
			free,
			currency,
			price_text,
			available,
			offers_iap,
			iap_range,
			// size,
			android_version,
			android_version_text,
			developer,
			developer_id,
			developer_internal_id,
			developer_email,
			developer_website,
			developer_address,
			developer_legal_name,
			developer_legal_email,
			developer_legal_address,
			developer_legal_phone_number,
			genre,
			genre_id,
			categories,
			icon,
			header_image,
			screenshots,
			video,
			video_image,
			preview_video,
			content_rating,
			content_rating_description,
			ad_supported,
			released,
			updated,
			version,
			recent_changes,
			comments,
			preregister,
			// early_access_enabled,
			// is_available_in_play_pass,
		})
	}

	pub async fn handle_response(&self, resp: Response) -> Result<ParsedHtml> {
		let text = resp
			.text()
			.await
			.context("Failed to get response text")?;
		spawn_blocking(move || {
			parse::parse(&text).context("Failed to parse html")
		})
		.await?
	}
}

impl TryFrom<&Options> for Request {
	type Error = anyhow::Error;
	fn try_from(this: &Options) -> Result<Self> {
		this.validate()?;
		Ok(Self::new(Method::GET, this.req_url()))
	}
}

impl Default for Options {
	fn default() -> Self {
		Self {
			app_id: String::new(),
			lang: String::from("en"),
			country: String::from("us"),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use crate::{cc, mapping};

	use super::*;

	#[tokio::test]
	#[macros::test]
	async fn fetch_valid_app_data() {
		let opts = Options {
			app_id: String::from("com.discord"),
			..Options::default()
		};
		let req = Request::try_from(&opts).unwrap();
		let client = cc();
		let resp = client.execute(req).await.unwrap();
		let text = resp.text().await.unwrap();
		// fs::write("app.html", &text).unwrap();
		let parsed = parse::parse(&text).unwrap();
		mapping!(TEST_MAPPING, "ds:5", 1, 2);
		dbg!(
			&parsed.script_data[TEST_MAPPING]
				.as_array()
				.unwrap()
				.len()
		);
		opts.handle_parsed_html(&parsed)
			.unwrap();
	}
}
