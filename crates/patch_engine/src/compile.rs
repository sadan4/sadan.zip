use oxc::ast::ast::RegExpFlags;
use regress::escape;
use vencord_ast_parser::{Match, MatchRegex, Patch};

/// Compiles every regex in `patches`, rewriting string `match:`es into escaped
/// regexes on the way.
///
/// This has to run before [`apply_patch`](crate::apply_patch) or
/// [`matches_module`](crate::matches_module):
/// [`MatchRegex::regex`] panics on a pattern that was never compiled.
///
/// A string `match:` becomes a flagless regex over the escaped needle
pub fn compile_patch_regexes<'a>(
	patches: impl IntoIterator<Item = &'a mut Patch>,
) {
	for patch in patches {
		if let Match::Regex(r) = &mut patch.find.v {
			r.make_regex();
		}
		for replacement in &mut patch.replacement {
			if let Match::Str(s) = &replacement.match_.v {
				replacement.match_.v = Match::Regex(MatchRegex {
					// we only ever create a finder with a utf8 string
					// so this should never error
					pattern: escape(str::from_utf8(s.needle()).unwrap()),
					flags: RegExpFlags::empty(),
					regex: None,
					// this is from a plain string so it has no capture groups
					capture_spans: Vec::new(),
				});
			}
			match &mut replacement.match_.v {
				Match::Regex(r) => r.make_regex(),
				// we just rewrote any Match::Str above
				Match::Str(_) => unreachable!(),
			}
		}
	}
}
