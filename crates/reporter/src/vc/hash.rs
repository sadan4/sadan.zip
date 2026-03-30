//! taken from <https://github.com/discord/discord-intl/blob/f417360bb7a0295f02895af8faf1dd663a72c003/crates/intl_message_utils/src/lib.rs#L8-L41>
//!
//! MIT License
//!
//! Copyright (c) 2024 Discord, Inc.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

#![allow(clippy::precedence_bits)]

/// The seed used when computing hash keys for message names and other hashed identifiers.
///
/// Ensure this hash seed matches the seed used in `intl/hash.ts`.
static KEY_HASH_SEED: u64 = 0;

/// Lookup table used for quickly creating a base64 representation of a hashed key.
static BASE64_TABLE: &[u8] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_bytes();

/// Returns a consistent, short hash of the given key by first processing it
/// through a sha256 digest, then encoding the first few bytes to base64.
///
/// Note that while this function is _generally_ the only place responsible for
/// hashing a key, there is a mirrored, client-side hash for use at runtime
/// that _must_ match this identically: `packages/intl/hash.ts`.
pub fn hash_message_key(content: &str) -> [char; 6] {
    let hash = xxhash_rust::xxh64::xxh64(content.as_bytes(), KEY_HASH_SEED);
    let input: [u8; 8] = hash.to_ne_bytes();
    // Since we know that we only want 6 characters out of the hash, we can
    // shortcut the base64 encoding to just directly read the bits out into an
    // encoded byte array and directly create a str from that.
    let output: [char; 6] = [
        BASE64_TABLE[(input[0] >> 2) as usize] as char,
        BASE64_TABLE[((input[0] & 0x03) << 4 | input[1] >> 4) as usize] as char,
        BASE64_TABLE[((input[1] & 0x0f) << 2 | input[2] >> 6) as usize] as char,
        BASE64_TABLE[(input[2] & 0x3f) as usize] as char,
        BASE64_TABLE[(input[3] >> 2) as usize] as char,
        BASE64_TABLE[((input[3] & 0x03) << 4 | input[4] >> 4) as usize] as char,
    ];

    output
}
