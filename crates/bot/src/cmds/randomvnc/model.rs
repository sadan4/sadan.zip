use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct Response {
	pub asn: String,
	pub desktop_name: String,
	pub geo_city: String,
	pub geo_country: String,
	pub geo_state: String,
	pub height: u32,
	pub id: u32,
	pub ip_address: String,
	pub password: String,
	pub port: u16,
	pub rdns_hostname: Option<String>,
	pub scanned_on: i64,
	pub width: u32,
}

#[cfg(test)]
mod tests {
	use super::*;

	fn de(j: &str) -> Response {
		serde_json::from_str(j).unwrap()
	}

	#[test]
	fn test_de() {
		let j = include_str!("./response.json");
		_ = de(j);
	}

	#[test]
	fn test_de_2() {
		let j = include_str!("./response2.json");
		_ = de(j);
	}
}
