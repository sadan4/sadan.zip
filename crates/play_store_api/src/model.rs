use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::mapping;

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
	pub url: String,
	pub title: String,
	pub description: String,
	pub description_html: String,
	pub summary: String,
	pub installs: String,
	pub min_installs: u32,
	pub max_installs: u32,
	pub score: u32,
	pub score_text: String,
	pub ratings: u32,
	pub reviews: u32,
	pub histogram: [f32; 5],
	pub price: u32,
	pub original_price: Option<u32>,
	pub discount_end_date: Option<String>,
	pub free: bool,
	pub currency: String,
	pub price_text: String,
	pub available: bool,
	pub offers_iap: bool,
	pub iap_range: String,
	pub size: String,
	pub android_version: String,
	pub android_version_text: String,
	pub developer: String,
	pub developer_id: String,
	pub developer_internal_id: String,
	pub developer_email: String,
	pub developer_website: String,
	pub developer_address: String,
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
	pub content_rating: String,
	pub content_rating_description: String,
	pub ad_supported: bool,
	pub released: String,
	pub updated: u32,
	pub version: String,
	pub recent_changes: String,
	pub comments: Vec<String>,
	pub has_early_access: bool,
	pub preregister: bool,
	pub is_available_in_play_pass: bool,
}
