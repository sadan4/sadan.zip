//! Tool implementations, kept free of MCP types so they can be tested
//! directly

pub mod css;
pub mod exports;
pub mod intl;
pub mod misc;
pub mod modules;
pub mod patch;

use std::{fmt::Write as _, ops::Range};

use anyhow::{Context as _, Result, anyhow};
use explorer_types::ModuleId;
use memchr::memmem::Finder;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use vencord_ast_parser::patches::{
	canonicalize_intl,
	canonicalize_regex_ident,
};

use crate::store::LoadedBundle;

/// A string or regex find, with Vencord's `\i` and `#{intl::KEY}` shorthands
/// expanded
pub enum Matcher {
	Str(Finder<'static>),
	Regex {
		re: regress::Regex,
		/// The same pattern compiled by the linear time `regex` crate, if it
		/// accepts the syntax
		///
		/// regress backtracks, so a pattern like `\i\.foo` is quadratic over
		/// long identifier-like runs, which minified modules are full of. This
		/// finds where a match starts in linear time, so regress only has to
		/// confirm it there. Its unicode classes are a superset of JS's ascii
		/// ones, so it never rejects a module regress would match.
		prefilter: Option<regex::Regex>,
	},
}

impl Matcher {
	/// `/pattern/flags` is a regex, anything else is a plain substring
	pub fn parse_find(find: &str) -> Result<Self> {
		if let Some(rest) = find.strip_prefix('/')
			&& let Some(end) = rest.rfind('/')
		{
			let (pattern, flags) = (&rest[..end], &rest[end + 1..]);
			return Self::regex(pattern, flags);
		}
		let find =
			canonicalize_intl(find, false, None).map_err(|e| anyhow!("{e}"))?;
		if find.is_empty() {
			return Err(anyhow!("The find is empty"));
		}
		Ok(Self::Str(Finder::new(find.as_bytes()).into_owned()))
	}

	pub fn regex(pattern: &str, flags: &str) -> Result<Self> {
		let pattern = canonicalize_intl(pattern, true, None)
			.map_err(|e| anyhow!("{e}"))?;
		let pattern = canonicalize_regex_ident(&pattern);
		// `g` and `y` only change how a regex is iterated, which find doesn't
		// do, and regress rejects `g`
		let flags = flags.replace(['g', 'y'], "");
		let re = regress::Regex::with_flags(&pattern, flags.as_str())
			.with_context(|| format!("Invalid regex /{pattern}/{flags}"))?;
		let prefilter = regex::RegexBuilder::new(&pattern)
			.case_insensitive(flags.contains('i'))
			.multi_line(flags.contains('m'))
			.dot_matches_new_line(flags.contains('s'))
			.build()
			.ok();
		Ok(Self::Regex { re, prefilter })
	}

	pub fn find(&self, src: &str) -> Option<Range<usize>> {
		self.find_at(src, 0)
	}

	/// The first match starting at or after byte `start`
	pub fn find_at(&self, src: &str, start: usize) -> Option<Range<usize>> {
		match self {
			Self::Str(f) => f
				.find(&src.as_bytes()[start..])
				.map(|at| start + at..start + at + f.needle().len()),
			Self::Regex {
				re,
				prefilter: None,
			} => re
				.find_from(src, start)
				.next()
				.map(|m| m.range()),
			Self::Regex {
				re,
				prefilter: Some(pre),
			} => {
				let hint = pre.find_at(src, start)?;
				re.find_from(src, hint.start())
					.next()
					// the two engines disagree, trust regress
					.or_else(|| re.find_from(src, start).next())
					.map(|m| m.range())
			}
		}
	}

	/// Every non-overlapping match in `src`
	pub fn find_iter<'a>(
		&'a self,
		src: &'a str,
	) -> impl Iterator<Item = Range<usize>> + 'a {
		let mut at = 0;
		std::iter::from_fn(move || {
			if at > src.len() {
				return None;
			}
			let m = self.find_at(src, at)?;
			// step past empty matches so this always makes progress
			at = if m.is_empty() {
				src.ceil_char_boundary(m.end + 1)
			} else {
				m.end
			};
			if m.is_empty() && m.end == src.len() {
				at = src.len() + 1;
			}
			Some(m)
		})
	}

	pub fn is_match(&self, src: &str) -> bool {
		self.find(src).is_some()
	}

	/// Every module in `bundle` this matches, sorted by id
	pub fn matching_modules(&self, bundle: &LoadedBundle) -> Vec<ModuleId> {
		let mut ids = bundle
			.modules
			.par_iter()
			.filter(|(_, src)| self.is_match(src))
			.map(|(&id, _)| id)
			.collect::<Vec<_>>();
		ids.sort_unstable();
		ids
	}
}

/// Formats a list of module ids, capped at `max`
pub fn id_list(ids: &[ModuleId], max: usize) -> String {
	let mut out = ids
		.iter()
		.take(max)
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(", ");
	if ids.len() > max {
		write!(out, ", … ({} more)", ids.len() - max).unwrap();
	}
	if out.is_empty() {
		out.push_str("(none)");
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn string_find() {
		let m = Matcher::parse_find("foo.bar").unwrap();
		assert_eq!(m.find("x.foo.bar"), Some(2..9));
		assert!(!m.is_match("foo_bar"));
	}

	#[test]
	fn regex_find_expands_ident() {
		let m = Matcher::parse_find(r"/\i\.isStaff\(\)/").unwrap();
		assert!(m.is_match("if(e.isStaff())"));
		assert!(!m.is_match("if(isStaff())"));
	}

	#[test]
	fn intl_find_is_hashed() {
		let m = Matcher::parse_find("#{intl::USER_SETTINGS}").unwrap();
		let Matcher::Str(f) = m else {
			panic!("expected a string find")
		};
		let needle = std::str::from_utf8(f.needle()).unwrap();
		assert!(!needle.contains("intl"), "{needle}");
	}

	#[test]
	fn regex_without_prefilter_still_matches() {
		// lookbehind is JS only, so this has no prefilter
		let m = Matcher::parse_find(r"/(?<=\.)isStaff/").unwrap();
		assert!(matches!(
			m,
			Matcher::Regex {
				prefilter: None,
				..
			}
		));
		assert!(m.is_match("e.isStaff()"));
		assert!(!m.is_match("isStaff()"));
	}

	#[test]
	fn iterates_matches() {
		let m = Matcher::parse_find("ab").unwrap();
		let found = m.find_iter("ab-ab-a").collect::<Vec<_>>();
		assert_eq!(found, [0..2, 3..5]);
		let m = Matcher::regex("x*", "").unwrap();
		assert_eq!(m.find_iter("ab").count(), 3, "empty matches advance");
	}

	#[test]
	fn bad_regex_errors() {
		assert!(Matcher::parse_find("/(/").is_err());
	}
}
