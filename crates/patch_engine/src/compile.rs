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

#[cfg(test)]
mod tests {
	use vencord_ast_parser::{Allocator, VencordAstParser};

	use super::compile_patch_regexes;

	const SRC: &str = r#"
import definePlugin from "@utils/types";

export default definePlugin({
	name: "Test",
	patches: [{
		find: "someString",
		replacement: {
			match: "abc",
			replace: "def",
		},
	}],
});
"#;

	/// Compiling rewrites a string `match` into a regex, which changes the
	/// patch's identity. Anything looking a patch up by
	/// [`Patch::content_hash`] has to do it before compiling.
	#[test]
	fn compiling_changes_the_content_hash() {
		let alloc = Allocator::new();
		let parser =
			VencordAstParser::try_new(&alloc, SRC, Some("plugins/test.ts"))
				.expect("the source parses");
		let mut patches = parser
			.patches(true)
			.expect("the plugin has patches");
		assert_eq!(patches.len(), 1);
		let before = patches[0].content_hash();
		compile_patch_regexes(&mut patches);
		assert_ne!(
			before,
			patches[0].content_hash(),
			"if this ever holds, the lookup order no longer matters"
		);
	}
}
