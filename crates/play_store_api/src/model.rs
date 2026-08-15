use url::Url;
use serde::{Deserialize, Serialize};

use crate::{mapping, mapping::Mapping};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct AppItem {
	pub url: String,
	pub app_id: String,
	pub title: String,
	pub summary: String,
	pub developer: String,
	pub developer_id: String,
	pub icon: String,
	pub score: Option<f64>,
	pub score_text: Option<String>,
	pub price_text: String,
	pub free: bool,
}

impl AppItem {
	mapping!(TITLE, 0, 3);
	mapping!(APP_ID, 0, 0, 0);
	mapping!(URL, 0, 10, 4, 2);
	mapping!(ICON, 0, 1, 3, 2);
	mapping!(DEVELOPER, 0, 14);
	mapping!(CURRENCY, 0, 8, 1, 0, 1);
	mapping!(PRICE, 0, 8, 1, 0, 0);
	mapping!(FREE, 0, 8, 1, 0, 0);
	mapping!(SUMMARY, 0, 13, 1);
	mapping!(SCORE_TEXT, 0, 4, 0);
	mapping!(SCORE, 0, 4, 1);
	mapping!(DEVELOPER_ID, 0, 14);

	#[cfg(test)]
	pub fn assert_valid(&self) {
		use crate::constants::BASE_URL;

		let base = Url::parse(BASE_URL).unwrap();
		base.join(&self.url)
			.expect("url is not valid");
		Url::parse(&self.icon).expect("icon is not valid");
		assert_eq!(
			self.score.is_some(),
			self.score_text.is_some(),
			"score and score_text should be both present or both absent"
		);
	}
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppItemCategory {
	pub name: String,
	pub id: Option<String>,
}

#[expect(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IAppItemFullDetail {
	pub app_id: String,
	pub url: Url,
	pub title: String,
	pub description: String,
	pub description_html: String,
	pub summary: String,
	pub installs: String,
	pub min_installs: f64,
	pub max_installs: f64,
	pub score: f64,
	pub score_text: String,
	/// number of ratings, not the average score
	pub ratings: f64,
	pub reviews: f64,
	pub histogram: [f32; 5],
	pub price: f64,
	pub original_price: Option<f64>,
	pub discount_end_date: Option<String>,
	pub free: bool,
	pub currency: String,
	pub price_text: String,
	pub available: bool,
	pub offers_iap: bool,
	pub iap_range: String,
	// forgotten about in js
	// pub size: String,
	pub android_version: String,
	pub android_version_text: String,
	pub developer: String,
	pub developer_id: String,
	pub developer_internal_id: String,
	pub developer_email: String,
	pub developer_website: String,
	pub developer_address: Option<String>,
	pub developer_legal_name: String,
	pub developer_legal_email: String,
	pub developer_legal_address: String,
	pub developer_legal_phone_number: String,
	pub genre: String,
	pub genre_id: String,
	pub categories: Vec<AppItemCategory>,
	pub icon: String,
	pub header_image: String,
	pub screenshots: Vec<String>,
	pub video: String,
	pub video_image: String,
	pub preview_video: String,
	pub content_rating: String,
	pub content_rating_description: serde_json::Value,
	pub ad_supported: bool,
	pub released: String,
	pub updated: u32,
	pub version: String,
	pub recent_changes: String,
	pub comments: Vec<String>,
	pub preregister: bool,
	// pub early_access_enabled: bool,
	// pub is_available_in_play_pass: bool,
}

impl IAppItemFullDetail {
	mapping!(TITLE, "ds:5", 1, 2, 0, 0);
	mapping!(DESCRIPTION, "ds:5", 1, 2);
	mapping!(DESCRIPTION_HTML, "ds:5", 1, 2);
	mapping!(SUMMARY, "ds:5", 1, 2, 73, 0, 1);
	mapping!(INSTALLS, "ds:5", 1, 2, 13, 0);
	mapping!(MIN_INSTALLS, "ds:5", 1, 2, 13, 1);
	mapping!(MAX_INSTALLS, "ds:5", 1, 2, 13, 2);
	mapping!(SCORE, "ds:5", 1, 2, 51, 0, 1);
	mapping!(SCORE_TEXT, "ds:5", 1, 2, 51, 0, 0);
	mapping!(RATINGS, "ds:5", 1, 2, 51, 2, 1);
	mapping!(REVIEWS, "ds:5", 1, 2, 51, 3, 1);
	mapping!(HISTOGRAM, "ds:5", 1, 2, 51, 1);
	mapping!(PRICE, "ds:5", 1, 2, 57, 0, 0, 0, 0, 1, 0, 0);
	mapping!(ORIGINAL_PRICE, "ds:5", 1, 2, 57, 0, 0, 0, 0, 1, 0, 0);
	mapping!(DISCOUNT_END_DATE, "ds:5", 1, 2, 57, 0, 0, 0, 0, 14, 1);
	pub const FREE: Mapping = Self::PRICE;
	mapping!(CURRENCY, "ds:5", 1, 2, 57, 0, 0, 0, 0, 1, 0, 1);
	mapping!(PRICE_TEXT, "ds:5", 1, 2, 57, 0, 0, 0, 0, 1, 0, 2);
	mapping!(AVAILABLE, "ds:5", 1, 2, 18, 0);
	mapping!(IAP_RANGE, "ds:5", 1, 2, 19, 0);
	mapping!(ANDROID_VERSION, "ds:5", 1, 2, 140, 1, 1, 0, 0, 1);
	mapping!(ANDROID_VERSION_FALLBACK, "ds:5", 1, 2, 0x0, "141", 1, 1, 0, 0, 1);
	pub const ANDROID_VERSION_TEXT: Mapping = Self::ANDROID_VERSION;
	pub const ANDROID_VERSION_TEXT_FALLBACK: Mapping = Self::ANDROID_VERSION_FALLBACK;
	mapping!(ANDROID_MAX_VERSION, "ds:5", 1, 2, 140, 1, 1, 0, 1, 1);
	mapping!(ANDROID_MAX_VERSION_FALLBACK, "ds:5", 1, 2, 0x0, "141", 1, 1, 0, 1, 1);
	mapping!(DEVELOPER, "ds:5", 1, 2, 68, 0);
	mapping!(DEVELOPER_ID, "ds:5", 1, 2, 68, 1, 4, 2);
	mapping!(DEVELOPER_EMAIL, "ds:5", 1, 2, 69, 1, 0);
	mapping!(DEVELOPER_WEBSITE, "ds:5", 1, 2, 69, 0, 5, 2);
	mapping!(DEVELOPER_ADDRESS, "ds:5", 1, 2, 69, 2, 0);
	mapping!(DEVELOPER_LEGAL_NAME, "ds:5", 1, 2, 69, 4, 0);
	mapping!(DEVELOPER_LEGAL_EMAIL, "ds:5", 1, 2, 69, 4, 1, 0);
	mapping!(DEVELOPER_LEGAL_ADDRESS, "ds:5", 1, 2, 69, 4, 2, 0);
	mapping!(DEVELOPER_LEGAL_PHONE_NUMBER, "ds:5", 1, 2, 69, 4, 3);
	mapping!(PRIVACY_POLICY, "ds:5", 1, 2, 99, 0, 5, 2);
	mapping!(DEVELOPER_INTERNAL_ID, "ds:5", 1, 2, 68, 1, 4, 2);
	mapping!(GENRE, "ds:5", 1, 2, 79, 0, 0, 0);
	mapping!(GENRE_ID, "ds:5", 1, 2, 79, 0, 0, 2);
	mapping!(CATEGORIES, "ds:5", 1, 2, 118);
	mapping!(ICON, "ds:5", 1, 2, 95, 0, 3, 2);
	mapping!(HEADER_IMAGE, "ds:5", 1, 2, 96, 0, 3, 2);
	mapping!(SCREENSHOTS, "ds:5", 1, 2, 78, 0);
	// called on each screenshot element
	mapping!(SCREENSHOT_MAP, 3, 2);
	mapping!(VIDEO, "ds:5", 1, 2, 100, 0, 0, 3, 2);
	mapping!(VIDEO_IMAGE, "ds:5", 1, 2, 100, 1, 0, 3, 2);
	mapping!(PREVIEW_VIDEO, "ds:5", 1, 2, 100, 1, 2, 0, 2);
	mapping!(CONTENT_RATING, "ds:5", 1, 2, 9, 0);
	mapping!(CONTENT_RATING_DESCRIPTION, "ds:5", 1, 2, 9, 2, 1);
	mapping!(AD_SUPPORTED, "ds:5", 1, 2, 48, 0);
	mapping!(RELEASED, "ds:5", 1, 2, 10, 0);
	mapping!(UPDATED, "ds:5", 1, 2, 145, 0, 1, 0);
	mapping!(UPDATED_FALLBACK, "ds:5", 1, 2, 0x0, "146", 0, 1, 0);
	mapping!(VERSION, "ds:5", 1, 2, 140, 0, 0, 0);
	mapping!(VERSION_FALLBACK, "ds:5", 1, 2, 0x0, "141", 0, 0, 0);
	mapping!(RECENT_CHANGES, "ds:5", 1, 2, 144, 1, 1);
	mapping!(RECENT_CHANGES_FALLBACK, "ds:5", 1, 2, 0x0, "145", 1, 1);
	// TODO: Comments
	mapping!(PREREGISTER, "ds:5", 1, 2, 18, 0);
	mapping!(EARLY_ACCESS_ENABLED, "ds:5", 1, 2, 18, 2);
	mapping!(IS_AVAILABLE_IN_PLAY_PASS, "ds:5", 1, 2, 62);
}
