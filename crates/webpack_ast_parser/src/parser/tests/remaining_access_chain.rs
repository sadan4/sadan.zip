use super::*;
use macros::test;

fn k(s: &'static str) -> ExportMapKey {
	SmolStr::new_static(s).into()
}

/// Runs [`WebpackAstParser::match_remaining_access_chain`] on every inline
/// `wreq(m_id).foo` in `body`, returning `(matched, remaining)` for each
fn remaining<'a>(
	body: &str,
	m_id: u32,
	export_names: &'a [ExportMapKey],
) -> (String, (&'a [ExportMapKey], &'a [ExportMapKey])) {
	let alloc = Allocator::new();
	let source = format!("0,function(e, t, n) {{ {body} }}");
	let p = WebpackAstParser::try_new(&alloc, &source).unwrap();
	let wreq = p.wreq().unwrap();
	let v = p
		.refs(wreq)
		.filter_map(|wreq_ref| {
			p.match_wreq_require_call(wreq_ref, ModuleId(m_id))
		})
		.filter_map(|call| {
			p.p(call.node_id())
				.as_static_member_expression()
		})
		.map(|access| {
			let (sme, chain) =
				p.match_remaining_access_chain(access, export_names);
			(sme.map_or_default(|x| x.property.to_string()), chain)
		})
		.collect_vec();
	if v.len() != 1 {
		panic!("expected exactly one wreq call, found {v:?}");
	}
	v.into_iter().next().unwrap()
}

#[test]
fn full_match() {
	let names = [k("A"), k("foo")];
	let res = remaining("n(123).A.foo;", 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "foo",
	    (
	        [
	            Named(
	                "A",
	            ),
	            Named(
	                "foo",
	            ),
	        ],
	        [],
	    ),
	)
	"#);
}

#[test]
fn chain_longer_than_export_names() {
	let names = [k("A")];
	let res = remaining("n(123).A.foo.bar;", 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "A",
	    (
	        [
	            Named(
	                "A",
	            ),
	        ],
	        [],
	    ),
	)
	"#);
}

#[test]
fn chain_shorter_than_export_names() {
	let names = [k("A"), k("foo")];
	let res = remaining("let i; (i = n(123).A).foo;", 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "A",
	    (
	        [
	            Named(
	                "A",
	            ),
	        ],
	        [
	            Named(
	                "foo",
	            ),
	        ],
	    ),
	)
	"#);
}

#[test]
fn mismatch_first() {
	let names = [k("A"), k("foo")];
	let res = remaining("n(123).B.foo;", 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "",
	    (
	        [],
	        [
	            Named(
	                "A",
	            ),
	            Named(
	                "foo",
	            ),
	        ],
	    ),
	)
	"#);
}

#[test]
fn mismatch_in_middle() {
	let names = [k("A"), k("foo"), k("bar")];
	let res = remaining("n(123).A.baz.bar;", 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "A",
	    (
	        [
	            Named(
	                "A",
	            ),
	        ],
	        [
	            Named(
	                "foo",
	            ),
	            Named(
	                "bar",
	            ),
	        ],
	    ),
	)
	"#);
}

#[test]
fn stops_at_computed_access() {
	let names = [k("A"), k("foo")];
	let res = remaining(r#"n(123).A["foo"];"#, 123, &names);
	assert_debug_snapshot!(res, @r#"
	(
	    "A",
	    (
	        [
	            Named(
	                "A",
	            ),
	        ],
	        [
	            Named(
	                "foo",
	            ),
	        ],
	    ),
	)
	"#);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "doesn't make sense for export names to be empty")]
fn empty_export_names() {
	_ = remaining("n(123).A;", 123, &[]);
}
