use crate::{
	diag::{PResult, err},
	hash::hash_message_key,
};
use ast_parser::exts::{ExpressionExt as _, TemplateLiteralExt as _};
use itertools::Itertools;
use oxc::{
	allocator::Vec as OxcVec,
	ast::ast::{
		ArrowFunctionExpression,
		Expression,
		RegExpLiteral,
		Str,
		StringLiteral,
		TemplateLiteral,
	},
	span::Span,
};
use regress::Regex;
use std::{borrow::Cow, fmt::Debug, sync::LazyLock};

#[derive(Debug)]
pub struct RawPatch<'ast> {
	pub all: bool,
	pub no_warn: bool,
	pub predicate: PatchPredicate<'ast>,
	pub find: RawMatchLike<'ast>,
	pub replacement: OxcVec<'ast, RawReplacement<'ast>>,
	/// Source span covering the patch object literal (or, in the spread
	/// `[a, b].map(v => ({ find: v, ... }))` case, the array element the
	/// patch was synthesized from). Used by editors to anchor patch-level
	/// UI like code lenses.
	pub span: Span,
	// I don't think Vencord uses these at all
	// from_build: Option<u32>,
	// to_build: Option<u32>,
}

// TODO: further parse this
#[derive(Copy, Clone)]
pub struct PatchPredicate<'ast>(pub Option<&'ast Expression<'ast>>);

#[derive(Debug, Clone)]
pub struct RawReplacement<'ast> {
	pub match_: RawMatchLike<'ast>,
	pub replace: RawReplace<'ast>,
	pub no_warn: bool,
	pub predicate: PatchPredicate<'ast>,
	// i don't think vencord uses these at all
	// from_build: Option<u32>,
	// to_build: Option<u32>,
}

#[derive(Copy, Clone)]
pub enum RawReplace<'ast> {
	String(&'ast StringLiteral<'ast>),
	Func(&'ast ArrowFunctionExpression<'ast>),
	Template(&'ast TemplateLiteral<'ast>),
	ComputedString(Str<'ast>, Span),
}

#[derive(Copy, Clone)]
pub enum RawMatchLike<'ast> {
	String(&'ast StringLiteral<'ast>),
	Regex(&'ast RegExpLiteral<'ast>),
	Template(&'ast TemplateLiteral<'ast>),
	ComputedString(Str<'ast>, Span),
}

static PATCH_INTL_REGEX: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"#{intl::([\w$+/]*)(?:::(\w+))?}").unwrap());

// FIXME: add tests
pub fn canonicalize_intl(
	s: &str,
	needs_regex_escape: bool,
	loc: impl Into<Option<Span>>,
) -> PResult<Cow<'_, str>> {
	let loc = loc.into().unwrap_or_default();
	// TODO: should this be find iter ascii
	let mut it = PATCH_INTL_REGEX.find_iter(s).peekable();
	if it.peek().is_none() {
		return Ok(Cow::Borrowed(s));
	}
	let mut ret = String::with_capacity(s.len());
	let mut last_end = 0;
	for m in it {
		ret.push_str(&s[last_end..m.start()]);
		last_end = m.end();
		let g_key = m.group(1).unwrap();
		let key = &s[g_key];
		let is_raw = m
			.group(2)
			.is_some_and(|g| &s[g] == "raw");
		let key = if is_raw {
			key.chars()
				.collect_array()
				.ok_or_else(|| err(&loc, "Raw intl key has invalid len"))?
		} else {
			hash_message_key(key)
		};
		let has_special_chars = {
			let mut it = key.iter();
			let first_char = it.next().unwrap();
			// instead of matching !is_ident_start start, we can match is_not_ident_start
			// because we know the only invalid chars this will ever contain
			// See: ./hash.rs
			matches!(first_char, '0'..='9' | '+' | '/')
				|| it.any(|&c| c == '+' || c == '/')
		};

		let is_hash_type = m
			.group(2)
			.is_some_and(|g| &s[g] == "hash");

		if needs_regex_escape {
			ret.push_str("(?:");
			if !is_hash_type {
				ret.push('\\');
			}
		}
		if !is_hash_type {
			if has_special_chars {
				ret.push('[');
				ret.push('"');
			} else {
				ret.push('.');
			}
		}
		for c in key {
			if needs_regex_escape && c == '+' {
				ret.push('\\');
			}
			ret.push(c);
		}

		if !is_hash_type && has_special_chars {
			ret.push('"');
			if needs_regex_escape {
				ret.push('\\');
			}
			ret.push(']');
		}

		if needs_regex_escape {
			ret.push(')');
		}
	}

	ret.push_str(&s[last_end..]);

	Ok(Cow::Owned(ret))
}

static PATCH_IDENT_REGEX: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"(\\*)\\i").unwrap());

// FIXME: add tests
pub fn canonicalize_regex_ident(s: &str) -> Cow<'_, str> {
	let mut it = PATCH_IDENT_REGEX
		.find_iter(s)
		.peekable();

	if it.peek().is_none() {
		return Cow::Borrowed(s);
	}

	let mut ret = String::with_capacity(s.len());
	let mut last_end = 0;

	for m in it {
		ret.push_str(&s[last_end..m.start()]);
		let g_esc = m.group(1).unwrap();
		if g_esc.len() & 1 == 0 {
			ret.push_str(&s[g_esc]);
			ret.push_str(r"(?:[A-Za-z_$][\w$]*)");
			last_end = m.end();
		} else {
			last_end = m.start() + 1;
		}
	}

	ret.push_str(&s[last_end..]);

	Cow::Owned(ret)
}

pub fn canonicalize_replace_for_regress(s: &mut str) {
	// SAFETY: we only ever operate on single ASCII chars;
	// therefore, the string will always be valid UTF8
	let bts = unsafe { s.as_bytes_mut() };
	let mut it = (0..bts.len()).peekable();
	while let Some(i) = it.next() {
		if bts[i] != b'$' {
			continue;
		}
		let Some(n) = it.peek().copied() else {
			continue;
		};
		match bts[n] {
			b'$' => {
				it.next();
			}
			// regress does not support $& in replacement strings
			// String.prototype.replace does not support $0
			// regex101 supports $&
			b'&' => {
				bts[n] = b'0';
				it.next();
			}
			// regress uses ${name} for named capture groups
			// String.prototype.replace uses $<name> for named capture groups
			// regex101 uses $<name>
			b'<' => {
				it.next();
				let mut found_closing = false;
				let mut end_idx = usize::MAX;
				for i in it.by_ref() {
					if bts[i] == b'>' {
						found_closing = true;
						end_idx = i;
						break;
					}
				}
				if found_closing {
					debug_assert_eq!(bts[n], b'<');
					debug_assert_eq!(bts[end_idx], b'>');
					bts[n] = b'{';
					bts[end_idx] = b'}';
				} else {
					// un-terminated, do nothing
					return;
				}
			}
			_ => {}
		}
	}
}

// TODO: Helper for COW regex

impl<'ast> From<Option<&'ast Expression<'ast>>> for PatchPredicate<'ast> {
	fn from(value: Option<&'ast Expression<'ast>>) -> Self {
		Self(value)
	}
}

impl Debug for PatchPredicate<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("PatchPredicate")
			.field(&self.0.map(Expression::dbg_name))
			.finish()
	}
}

impl Debug for RawReplace<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::String(s) => f
				.debug_tuple("String")
				.field(&s.value)
				.finish(),
			Self::Func(_) => f
				.debug_tuple("Func")
				.field(&"...")
				.finish(),
			Self::Template(t) => f
				.debug_tuple("Template")
				.field(&t.dbg_str())
				.finish(),
			Self::ComputedString(s, _) => f
				.debug_tuple("ComputedString")
				.field(s)
				.finish(),
		}
	}
}

impl Debug for RawMatchLike<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::String(s) => f
				.debug_tuple("String")
				.field(&s.value)
				.finish(),
			Self::Regex(r) => f
				.debug_tuple("Regex")
				.field(&r.regex.pattern.text)
				.finish(),
			Self::Template(t) => f
				.debug_tuple("Template")
				.field(t)
				.finish(),
			Self::ComputedString(s, _) => f
				.debug_tuple("ComputedString")
				.field(s)
				.finish(),
		}
	}
}

#[cfg(test)]
mod tests {
	use insta::assert_snapshot;

	use super::*;

	#[test]
	fn test_canonicalize_intl() {
		let regex_1 = r"(?<=children:\[)(?=.{10,80}tooltip:.{0,100}#{intl::ATTACHMENT_UTILITIES_SPOILER})";
		let canon_1 =
			r"(?<=children:\[)(?=.{10,80}tooltip:.{0,100}(?:\.cuurzA))";
		assert_eq!(canonicalize_intl(regex_1, true, None).unwrap(), canon_1);
	}

	#[test]
	fn test_canonicalize_intl_with_hash() {
		let src = r##""#{intl::APP_TAG::hash}":"##;
		assert_snapshot!(canonicalize_intl(src, false, None).unwrap(), @r#""9RNkeF":"#);
		assert_snapshot!(canonicalize_intl(src, true, None).unwrap(), @r#""(?:9RNkeF)":"#);
	}
}
