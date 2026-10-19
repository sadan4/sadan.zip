use explorer_types::{BuildList, BundleMetadata, Channel, TModuleId};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast as _, prelude::wasm_bindgen};

use crate::{
	constants::LIST_BUILDS_ENDPOINT,
	err::{BadCast, Error, Result},
	util::{fetch, fut::JsPromiseExt as _},
};

#[wasm_bindgen]
#[derive(Clone)]
pub struct Meta(BundleMetadata);

impl From<BundleMetadata> for Meta {
	fn from(value: BundleMetadata) -> Self {
		Self(value)
	}
}

#[wasm_bindgen]
#[expect(
	clippy::missing_const_for_fn,
	reason = "wasm bindgen does not support const fn"
)]
impl Meta {
	#[wasm_bindgen(getter)]
	pub fn build_hash(&self) -> String {
		self.0.build_hash.clone()
	}
	#[wasm_bindgen(getter)]
	pub fn build_number(&self) -> u32 {
		self.0.build_number
	}
	#[wasm_bindgen(getter)]
	pub fn first_seen(&self) -> u64 {
		self.0.first_seen
	}
	#[wasm_bindgen(getter)]
	pub fn entry_point(&self) -> Option<TModuleId> {
		self.0.entry_point.map(Into::into)
	}
	/// The release channels this build was seen on.
	#[wasm_bindgen(getter)]
	pub fn channels(&self) -> Vec<String> {
		self.0
			.channel
			.iter()
			.map(|channel| {
				match channel {
					Channel::Stable => "stable",
					Channel::Canary => "canary",
				}
				.to_owned()
			})
			.collect()
	}
	#[wasm_bindgen]
	pub fn sort_newest_first(a: &Self, b: &Self) -> i8 {
		a.0.first_seen.cmp(&b.0.first_seen) as i8
	}
}

/// One page of builds, newest first
#[wasm_bindgen]
pub struct BuildsPage {
	builds: Box<[Meta]>,
	next_cursor: Option<u64>,
}

#[wasm_bindgen]
#[expect(
	clippy::missing_const_for_fn,
	reason = "wasm bindgen does not support const fn"
)]
impl BuildsPage {
	#[wasm_bindgen(getter)]
	pub fn builds(&self) -> Box<[Meta]> {
		self.builds.clone()
	}
	/// Pass as `before` to [`get_builds_page`] for the next page. `undefined`
	/// on the last page
	#[wasm_bindgen(getter)]
	pub fn next_cursor(&self) -> Option<u64> {
		self.next_cursor
	}
}

/// Fetches the page of builds first seen before `before`, or the newest
/// builds if it's `undefined`
#[wasm_bindgen]
pub async fn get_builds_page(before: Option<u64>) -> Result<BuildsPage> {
	let url = match before {
		Some(before) => format!("{LIST_BUILDS_ENDPOINT}?before={before}"),
		None => LIST_BUILDS_ENDPOINT.to_owned(),
	};
	// TODO: streaming decode
	let arr_buf = fetch(&url)
		.await?
		.array_buffer()?
		.fut()
		.await?
		.dyn_into::<ArrayBuffer>()
		.map_err(|_| BadCast::ArrayBuffer)?;
	// this request is NOT zstd compressed as of now, it is just raw MessagePack data
	let mpk_data_buf = Uint8Array::new(&arr_buf).to_vec();
	let data: BuildList = rmp_serde::from_slice(&mpk_data_buf)?;
	let builds = data
		.builds
		.into_iter()
		.map(|zstd_raw_meta| -> Result<_> {
			let mpk_raw_meta =
				zstd::Decoder::new(&*zstd_raw_meta).map_err(Error::Zstd)?;
			Ok(Meta(rmp_serde::from_read(mpk_raw_meta)?))
		})
		.collect::<Result<_>>()?;
	Ok(BuildsPage {
		builds,
		next_cursor: data.next_cursor,
	})
}
