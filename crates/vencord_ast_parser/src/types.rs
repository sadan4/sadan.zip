pub mod find;

use anyhow::Result;
use derive_more::{Eq, PartialEq};
use itertools::Itertools;
use memchr::memmem::Finder;
use oxc::{ast::ast::RegExpFlags, span::Span};
use regress::{Flags, Regex};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Surface-level info about a plugin's `definePlugin({...})` declaration —
/// just enough for the LSP to anchor a code lens and ship a command
/// payload, without exposing the full AST.
#[derive(Debug, Clone, Copy)]
pub struct PluginInfo<'ast> {
	pub name: &'ast str,
	/// Source span covering the `definePlugin` object literal.
	pub span: Span,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Patch {
	pub plugin_id: Option<u16>,
	pub all: bool,
	pub no_warn: bool,
	pub find: MatchLike,
	pub replacement: Vec<Replacement>,
	/// Source span covering the patch object literal (or the spread array
	/// element it was synthesized from). Suitable for UI anchoring.
	#[serde(deserialize_with = "deserialize_span")]
	pub span: Span,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Replacement {
	pub match_: MatchLike,
	pub replace: ReplaceLike,
	pub no_warn: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplaceLike {
	pub v: Replacer,
	#[serde(deserialize_with = "deserialize_span")]
	pub s: Span,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Replacer {
	Str(String),
	Template(TemplateEvaluator),
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateEvaluator {
	pub(crate) lits: Vec<String>,
	pub(crate) captures: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatchLike {
	pub v: Match,
	#[serde(deserialize_with = "deserialize_span")]
	pub s: Span,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Match {
	#[serde(with = "FinderDef")]
	Str(Finder<'static>),
	Regex(MatchRegex),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchRegex {
	pub pattern: String,
	#[serde(with = "RegExpFlagsDef")]
	pub flags: RegExpFlags,
	#[serde(skip)]
	#[eq(skip)]
	pub regex: Option<Result<Regex, regress::Error>>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RegExpFlags")]
struct RegExpFlagsDef {
	#[serde(getter = "RegExpFlags::bits")]
	bits: <oxc::ast::ast::RegExpFlags as bitflags::Flags>::Bits,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Finder<'static>")]
struct FinderDef {
	#[serde(getter = "finder_get_needle")]
	needle: Box<str>,
}

impl TemplateEvaluator {
	pub fn make_replacer<'this: 's, 's: 'this>(
		&'this self,
		src: &'s str,
	) -> impl 'this + 's + Fn(&regress::Match) -> String {
		|m| self.do_replace(src, m)
	}
	pub fn do_replace(&self, src: &str, m: &regress::Match) -> String {
		let lits = self.lits.iter().map(String::as_str);
		let caps = self.captures.iter().map(|&i| {
			let range = m
				.group(i as _)
				.expect("capture group out of range");
			&src[range]
		});
		debug_assert_eq!(self.lits.len(), self.captures.len() + 1);
		lits.interleave_shortest(caps).collect()
	}
}

/// This is a re-implementation (read: copy-paste) of [`regress::Regex::expand_replacement`] as it is private
fn process_string_replacement(
	repl: &str,
	text: &str,
	m: &regress::Match,
) -> String {
	const MAX_CAPTURE_GROUPS: usize = u16::MAX as _;
	let mut output = String::with_capacity(m.end() - m.start());
	let mut chars = repl.chars().peekable();
	while let Some(ch) = chars.next() {
		if ch == '$' {
			match chars.peek() {
				Some('$') => {
					// $$ -> literal $
					chars.next();
					output.push('$');
				}
				Some(&digit) if digit.is_ascii_digit() => {
					// Parse the group number
					let mut group_num = 0;
					while let Some(&digit) = chars.peek() {
						if digit.is_ascii_digit() {
							chars.next();
							group_num = group_num * 10
								+ (digit as u32 - '0' as u32) as usize;
							// Limit to reasonable group numbers to avoid overflow
							if group_num > MAX_CAPTURE_GROUPS {
								break;
							}
						} else {
							break;
						}
					}

					// Get the matched text for this group
					if let Some(range) = m.group(group_num) {
						output.push_str(&text[range]);
					}
					// If group doesn't exist or didn't match, add nothing
				}
				Some('{') => {
					// Handle ${name} syntax for named groups
					chars.next(); // consume '{'
					let mut name = String::new();
					let mut found_closing_brace = false;

					for ch in chars.by_ref() {
						if ch == '}' {
							found_closing_brace = true;
							break;
						}
						name.push(ch);
					}

					if found_closing_brace {
						if let Some(range) = m.named_group(&name) {
							output.push_str(&text[range]);
						}
					} else {
						// Malformed ${...}, treat as literal
						output.push_str("${");
						output.push_str(&name);
					}
				}
				_ => {
					// Just a $ at end or followed by non-digit
					output.push('$');
				}
			}
		} else {
			output.push(ch);
		}
	}
	output
}

impl Replacer {
	pub fn do_replace(&self, src: &str, m: &regress::Match) -> String {
		match self {
			Self::Str(repl) => process_string_replacement(repl, src, m),
			Self::Template(repl) => repl.do_replace(src, m),
		}
	}
}

impl Patch {
	pub const fn plugin_id(&self) -> u16 {
		self.plugin_id
			.expect("Plugin ID not set")
	}
}

impl From<FinderDef> for Finder<'static> {
	fn from(value: FinderDef) -> Self {
		Finder::new(value.needle.as_bytes()).into_owned()
	}
}

#[expect(clippy::fallible_impl_from, reason = "TODO")]
impl From<RegExpFlagsDef> for RegExpFlags {
	fn from(value: RegExpFlagsDef) -> Self {
		Self::from_bits(value.bits).unwrap()
	}
}

impl Hash for MatchRegex {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.pattern.hash(state);
		self.flags.hash(state);
	}
}

impl MatchRegex {
	pub fn make_regex(&mut self) {
		if self.regex.is_some() {
			return;
		}
		let f = |f| self.flags.contains(f);
		self.regex = Some(Regex::with_flags(
			&self.pattern,
			Flags {
				icase: f(RegExpFlags::I),
				multiline: f(RegExpFlags::M),
				dot_all: f(RegExpFlags::S),
				unicode: f(RegExpFlags::U),
				unicode_sets: f(RegExpFlags::V),
				no_opt: false,
			},
		));
	}

	pub const fn regex(&self) -> &Result<Regex, regress::Error> {
		self.regex
			.as_ref()
			.expect("Regex not compiled")
	}
}

impl Match {
	#[must_use]
	pub const fn as_regex(&self) -> Option<&MatchRegex> {
		if let Self::Regex(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Hash for Match {
	fn hash<H: Hasher>(&self, state: &mut H) {
		core::mem::discriminant(self).hash(state);
		match self {
			Self::Str(s) => s.needle().hash(state),
			Self::Regex(s) => s.hash(state),
		}
	}
}

impl PartialEq for Match {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Str(l0), Self::Str(r0)) => l0.needle() == r0.needle(),
			(Self::Regex(l0), Self::Regex(r0)) => l0 == r0,
			_ => false,
		}
	}
}

impl Eq for Match {}

/// [`Span`] only implements [`Serialize`] (as a `{ start, end }` map), not
/// [`Deserialize`], so we provide the inverse here.
fn deserialize_span<'de, D>(
	deserializer: D,
) -> std::result::Result<Span, D::Error>
where
	D: serde::Deserializer<'de>,
{
	#[derive(Deserialize)]
	struct SpanData {
		start: u32,
		end: u32,
	}
	let SpanData { start, end } = SpanData::deserialize(deserializer)?;
	Ok(Span::new(start, end))
}

fn finder_get_needle(finder: &Finder<'_>) -> Box<str> {
	str::from_utf8(finder.needle())
		.expect("finder is not a utf8 string")
		.into()
}
