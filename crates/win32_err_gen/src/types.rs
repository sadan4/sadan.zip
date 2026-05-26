use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct W32Error {
	pub code: u32,
	pub message: String,
	pub desc: String,
}
