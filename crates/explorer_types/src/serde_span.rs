use oxc_span::Span;
use serde::Deserialize;

/// Mirrors the `{ start, end }` map that [`Span`]'s [`Serialize`] impl emits.
#[derive(Deserialize)]
struct SpanData {
	start: u32,
	end: u32,
}

impl From<SpanData> for Span {
	fn from(SpanData { start, end }: SpanData) -> Self {
		Self::new(start, end)
	}
}

/// [`Span`] only implements [`Serialize`] (as a `{ start, end }` map), not
/// [`Deserialize`], so we provide the inverse here.
pub fn deserialize_span<'de, D>(deserializer: D) -> Result<Span, D::Error>
where
	D: serde::Deserializer<'de>,
{
	Ok(SpanData::deserialize(deserializer)?.into())
}
