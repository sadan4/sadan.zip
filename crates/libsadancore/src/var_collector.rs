use std::collections::HashSet;

use anyhow::Context as _;
use lightningcss::{
	properties::{
		Property,
		custom::{CustomProperty, CustomPropertyName, Variable},
	},
	stylesheet::{ParserOptions, StyleSheet},
	visitor::{Visit, VisitTypes, Visitor},
};

use crate::err::Result;

type CssError<'a> =
	lightningcss::error::Error<lightningcss::error::ParserError<'a>>;

struct Collector<'a>(&'a mut HashSet<String>);

impl<'ast> Visitor<'ast> for Collector<'_> {
	type Error = !;

	fn visit_types(&self) -> VisitTypes {
		VisitTypes::VARIABLES | VisitTypes::PROPERTIES
	}

	fn visit_variable(
		&mut self,
		var: &mut Variable<'ast>,
	) -> Result<(), Self::Error> {
		if !self.0.contains(var.name.ident.as_ref()) {
			self.0
				.insert(var.name.ident.to_string());
		}
		// the fallback can contain more variables
		var.visit_children(self)
	}
	fn visit_property(
		&mut self,
		property: &mut Property<'ast>,
	) -> Result<(), Self::Error> {
		if let Property::Custom(CustomProperty {
			name: CustomPropertyName::Custom(name),
			value: _,
		}) = property
			&& !self.0.contains(name.as_ref())
		{
			self.0.insert(name.to_string());
		}
		property.visit_children(self)
	}
}

pub fn collect_css_vars(vars: &mut HashSet<String>, css: &str) -> Result<()> {
	let opts = ParserOptions {
		// there are at least 2 css output that cause invalid syntax in discord's bundle
		error_recovery: true,
		..ParserOptions::default()
	};
	let mut ast = StyleSheet::parse(css, opts)
		.map_err(CssError::into_owned)
		.context("Failed to parse css")?;
	ast.visit(&mut Collector(vars))
		.into_ok();
	Ok(())
}

#[cfg(test)]
mod collects_vars {
	use super::*;

	#[test]
	fn defined() {
		let code = r":root { --foo: green; }";
		let mut vars = HashSet::new();
		collect_css_vars(&mut vars, code).unwrap();
		dbg!(&vars);
		assert!(vars.contains("--foo"));
		assert_eq!(vars.len(), 1);
	}

	#[test]
	fn used() {
		let code =
			r".card { color: var(--bar); background-color: var(--baz); }";
		let mut vars = HashSet::new();
		collect_css_vars(&mut vars, code).unwrap();
		dbg!(&vars);
		assert!(vars.contains("--bar"));
		assert!(vars.contains("--baz"));
		assert_eq!(vars.len(), 2);
	}

	#[test]
	fn fallback() {
		let code = r".card { color: var(--a, var(--b, var(--c))); }";
		let mut vars = HashSet::new();
		collect_css_vars(&mut vars, code).unwrap();
		dbg!(&vars);
		assert!(vars.contains("--a"));
		assert!(vars.contains("--b"));
		assert!(vars.contains("--c"));
		assert_eq!(vars.len(), 3);
	}
}
