use std::str::Utf8Error;

use bytes::Bytes;

#[derive(Debug)]
pub struct ByteStr(Bytes);

impl TryFrom<Bytes> for ByteStr {
	type Error = Utf8Error;

	fn try_from(value: Bytes) -> std::result::Result<Self, Self::Error> {
		if let Err(e) = str::from_utf8(&value) {
			Err(e)
		} else {
			Ok(Self(value))
		}
	}
}

impl AsRef<str> for ByteStr {
	fn as_ref(&self) -> &str {
		// SAFETY: ByteStr can only be constructed from valid UTF-8 bytes
		unsafe { str::from_utf8_unchecked(&self.0) }
	}
}

impl std::fmt::Display for ByteStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.as_ref().fmt(f)
	}
}
