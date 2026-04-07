use anyhow::Result;
use derive_more::{Eq, PartialEq};
use itertools::Itertools;
use memchr::memmem::Finder;
use oxc::{ast::ast::RegExpFlags, span::Span};
use regress::{Flags, Regex};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Patch {
	pub plugin_id: Option<u16>,
	pub all: bool,
	pub no_warn: bool,
	pub find: MatchLike,
	pub replacement: Vec<Replacement>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Replacement {
	pub match_: MatchLike,
	pub replace: ReplaceLike,
	pub no_warn: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ReplaceLike {
	pub v: Replacer,
	pub s: Span,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Replacer {
	Str(String),
	Template(TemplateEvaluator),
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct TemplateEvaluator {
	pub(crate) lits: Vec<String>,
	pub(crate) captures: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct MatchLike {
	pub v: Match,
	pub s: Span,
}

#[derive(Debug, Serialize)]
pub enum Match {
	#[serde(with = "FinderDef")]
	Str(Finder<'static>),
	Regex(MatchRegex),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct MatchRegex {
	pub pattern: String,
	#[serde(with = "RegExpFlagsDef")]
	pub flags: RegExpFlags,
	#[serde(skip)]
	#[eq(skip)]
	pub regex: Option<Result<Regex>>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "RegExpFlags")]
struct RegExpFlagsDef {
	#[serde(getter = "RegExpFlags::bits")]
	bits: <oxc::ast::ast::RegExpFlags as bitflags::Flags>::Bits,
}

#[derive(Serialize)]
#[serde(remote = "Finder")]
struct FinderDef {
	#[serde(getter = "finder_get_needle")]
	needle: Box<str>,
}

impl TemplateEvaluator {
	pub fn make_replacer<'a: 'b, 'b: 'a>(
		&'a self,
		src: &'b str,
	) -> impl 'a + 'b + Fn(&regress::Match) -> String {
		|m| {
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
		self.regex = Some(
			Regex::with_flags(
				&self.pattern,
				Flags {
					icase: f(RegExpFlags::I),
					multiline: f(RegExpFlags::M),
					dot_all: f(RegExpFlags::S),
					unicode: f(RegExpFlags::U),
					unicode_sets: f(RegExpFlags::V),
					no_opt: false,
				},
			)
			.map_err(Into::into),
		);
	}

	pub const fn regex(&self) -> &Result<Regex> {
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

fn finder_get_needle(finder: &Finder<'_>) -> Box<str> {
	str::from_utf8(finder.needle())
		.expect("finder is not a utf8 string")
		.into()
}
