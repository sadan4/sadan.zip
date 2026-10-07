use super::*;
use macros::test;

use crate::parser::{
	icons::{Issue, dom},
	value_tracking::{ConstantSpannedValue, ConstantValue},
};

fn icon_props_export(func: &str) -> Option<SmolStr> {
	let alloc = Allocator::new();
	let source =
		format!("0,function(e, t, n) {{ n.d(t, {{ _: () => r }}); {func} }}");
	let p = WebpackAstParser::try_new(&alloc, &source).unwrap();
	p.default_icon_props_export()
}

fn is_icon_props(func: &str) -> bool {
	icon_props_export(func).is_some()
}

const DISCORD: &str = r#"function r(e) {
	let t = null != e["aria-label"];
	return e["aria-hidden"] = e["aria-hidden"] ?? !t,
	e.role = e.role ?? "img",
	e
}"#;

#[test]
fn matches_discord_impl() {
	assert_eq!(icon_props_export(DISCORD).as_deref(), Some("_"));
}

#[test]
fn matches_swapped_null_check_and_access_styles() {
	assert!(is_icon_props(
		r#"function r(e) {
			let t = e["aria-label"] != null;
			return e["aria-hidden"] = e["aria-hidden"] ?? !t,
			e["role"] = e.role ?? "img",
			e
		}"#
	));
}

#[test]
fn rejects_wrong_role() {
	assert!(!is_icon_props(&DISCORD.replace("\"img\"", "\"button\"")));
}

#[test]
fn rejects_wrong_return() {
	assert!(!is_icon_props(&DISCORD.replace("\te\n}", "\tt\n}")));
}

#[test]
fn rejects_negating_other_var() {
	assert!(!is_icon_props(&DISCORD.replace("?? !t", "?? !e")));
}

#[test]
fn rejects_other_object() {
	assert!(!is_icon_props(
		&DISCORD.replace("e.role = e.role", "t.role = t.role")
	));
}

#[test]
fn rejects_multiple_exports() {
	let alloc = Allocator::new();
	let source = format!(
		"0,function(e, t, n) {{ n.d(t, {{ _: () => r, x: () => r }}); {DISCORD} }}"
	);
	let p = WebpackAstParser::try_new(&alloc, &source).unwrap();
	assert!(p.default_icon_props_export().is_none());
}

/// Shaped like a Discord icon component, `n(200)._` is the icon props
/// function
const ICON_MODULE: &str = r#"// Webpack Module 1
0,function(e, t, n) {
	n.d(t, { Icon: () => l });
	var r = n(100), o = n(200), c = n(300);
	let l = e => {
		let {
			size: t = 24,
			width: w = t,
			height: h = t,
			color: i = r.colors.INTERACTIVE,
			secondaryColor: s = "transparent",
			colorClass: u = "",
			...d
		} = e;
		return (0, c.jsx)("svg", {
			...(0, o._)(d),
			width: w,
			height: h,
			fill: "none",
			viewBox: "0 0 24 24",
			children: [
				(0, c.jsx)("path", {
					fill: "string" == typeof i ? i : i.css,
					d: "M0 0h24",
				}),
				(0, c.jsx)("circle", { fill: s, r: 2 }),
				"label",
				null,
				!1,
				x ? (0, c.jsx)("g", {}) : null,
			],
		});
	};
}"#;

fn attr<'a>(node: &'a dom::Node, name: &str) -> &'a dom::AttrValue {
	node.attrs
		.get(name)
		.unwrap_or_else(|| panic!("no `{name}` on <{}>: {node:#?}", node.tag))
}

fn constant(value: &dom::AttrValue) -> &ConstantSpannedValue {
	let dom::AttrValue::Constant(v) = value else {
		panic!("expected a constant, got {value:#?}");
	};
	&v.value
}

#[test]
async fn icon_dom_resolves_color_params_and_children() {
	let alloc = Allocator::new();
	let p = WebpackAstParser::try_new(&alloc, ICON_MODULE).unwrap();
	let [props_use] = p
		.get_raw_uses_of_import(200.into(), &[ExportMapKey::Named("_".into())])
		.try_into()
		.unwrap();
	let icon = p
		.try_get_icon_from_default_props_use(props_use)
		.await
		.unwrap();
	let svg = &icon.node;

	assert_eq!(svg.tag, "svg");
	assert_eq!(
		constant(attr(svg, "fill")),
		&ConstantValue::String("none".into())
	);
	assert_eq!(
		constant(attr(svg, "viewBox")),
		&ConstantValue::String("0 0 24 24".into())
	);

	let [
		dom::Child::Element(path),
		dom::Child::Element(circle),
		dom::Child::Text(label),
	] = svg.children.as_slice()
	else {
		panic!("unexpected children: {:#?}", svg.children);
	};
	assert_eq!(label.value, ConstantValue::String("label".into()));

	// a design token default is the icon's main color
	assert_eq!(path.tag, "path");
	assert!(matches!(
		attr(path, "fill"),
		dom::AttrValue::ColorParam { default: None, .. }
	));
	assert_eq!(
		constant(attr(path, "d")),
		&ConstantValue::String("M0 0h24".into())
	);

	// a string default is kept
	assert_eq!(circle.tag, "circle");
	let dom::AttrValue::ColorParam {
		default: Some(default),
		..
	} = attr(circle, "fill")
	else {
		panic!("expected a color param with a default: {circle:#?}");
	};
	assert_eq!(default.value, ConstantValue::String("transparent".into()));

	// the props spread and the conditional child are recorded, at the part
	// that failed to resolve, `null` and `!1` are not
	let src_of = |issue: &Issue| {
		&ICON_MODULE[issue.span.start as usize..issue.span.end as usize]
	};
	let issue_srcs = icon
		.issues
		.iter()
		.map(src_of)
		.collect::<Vec<_>>();
	assert!(issue_srcs.contains(&"...(0, o._)(d)"), "{:#?}", icon.issues);
	assert!(issue_srcs.contains(&"x"), "{:#?}", icon.issues);
	assert!(
		!issue_srcs
			.iter()
			.any(|s| *s == "null" || *s == "!1"),
		"{:#?}",
		icon.issues
	);
}

#[test]
async fn icon_dom_to_html() {
	let alloc = Allocator::new();
	let p = WebpackAstParser::try_new(&alloc, ICON_MODULE).unwrap();
	let [props_use] = p
		.get_raw_uses_of_import(200.into(), &[ExportMapKey::Named("_".into())])
		.try_into()
		.unwrap();
	let icon = p
		.try_get_icon_from_default_props_use(props_use)
		.await
		.unwrap();

	assert_eq!(
		icon.node.to_html(),
		r#"<svg fill="none" viewBox="0 0 24 24"><path d="M0 0h24" fill="currentColor"/><circle fill="transparent" r="2"/>label</svg>"#
	);
}
