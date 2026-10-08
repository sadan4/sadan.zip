pub mod dom;
mod jsx;

use std::{borrow::Cow, collections::HashMap};

use ast_parser::{
	AstParser as _,
	ast_kind::IntoAstKind,
	exts::{ExpressionExt, Functionish, MemberExpressionExt},
};
use explorer_types::SpannedId;
use oxc::{
	ast::{
		AstKind,
		ast::{
			Argument,
			AssignmentExpression,
			AssignmentOperator,
			BinaryOperator,
			BindingIdentifier,
			BindingPattern,
			Expression,
			LogicalOperator,
			MemberExpression,
			ObjectExpression,
			ObjectPropertyKind,
			Statement,
			SwitchCase,
			UnaryOperator,
		},
	},
	semantic::{NodeId, SymbolId},
	span::{GetSpan, Span},
	syntax::GetNodeId,
};
use parser_diag::{PResult, ParserDiagnostic, err};
use smol_str::{SmolStr, ToSmolStr};
use tracing::{Instrument, debug, info_span, warn};

use crate::{
	WebpackAstParser,
	export_map::{ExportMapKey, ExportRange, ExportValue},
	parser::{
		types::Importer,
		value_tracking::{ConstantValue, SpannedValue},
	},
	sync::UnsafeFuture,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
	pub name: Option<SmolStr>,
	pub defined_in: SpannedId,
	pub span: Span,
	pub node: dom::Node,
	/// Parts of the markup that could not be read statically, so are missing
	/// from [`Self::node`]
	pub issues: Vec<Issue>,
}

/// Something in an icon's markup that could not be read statically
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
	/// Location in the module the icon is defined in
	pub span: Span,
	pub msg: Cow<'static, str>,
}

/// Attributes whose value may be a color passed in as a prop
const COLOR_ATTRS: &[&str] = &["fill", "stroke", "color", "stopColor"];

/// State threaded through [`WebpackAstParser::jsx_to_dom`]
struct DomCx<'ast> {
	issues: Vec<Issue>,
	/// Bindings of the icon component's props, with their default if they
	/// have one, see [`WebpackAstParser::component_props`]
	props: HashMap<SymbolId, Option<&'ast Expression<'ast>>>,
}

impl DomCx<'_> {
	fn issue(
		&mut self,
		span: &impl GetSpan,
		msg: impl Into<Cow<'static, str>>,
	) {
		self.issues.push(Issue {
			span: span.span(),
			msg: msg.into(),
		});
	}

	/// Records `diag` as an issue at its first label, or at `fallback` if it
	/// has none
	fn diag(&mut self, diag: ParserDiagnostic, fallback: &impl GetSpan) {
		let span = diag
			.labels
			.first()
			.map_or_else(|| fallback.span(), |(span, _)| *span);
		self.issues.push(Issue {
			span,
			msg: diag.msg,
		});
	}
}

/// Icon scraping
impl<'ast> WebpackAstParser<'ast> {
	pub async fn scrape_icons(&self) -> Option<PResult<Vec<Icon>>> {
		let export = self.default_icon_props_export()?;
		let fut = self.impl_scrape_icons(export);
		// SAFETY: see send + sync impl for WebpackAstParser
		let fut = unsafe { UnsafeFuture::new(fut) };
		Some(fut.await)
	}
	#[expect(clippy::future_not_send)]
	async fn impl_scrape_icons(&self, export: SmolStr) -> PResult<Vec<Icon>> {
		let importers = self
			.find_importers(vec![ExportMapKey::Named(export)])
			.await?;
		let mut ret = Vec::new();
		for Importer {
			parser,
			export_name,
			imported_id,
			module_id: parser_id,
		} in importers
		{
			let parser = parser.parser();
			async {
				let uses =
					parser.get_raw_uses_of_import(imported_id, &export_name);
				ret.reserve(uses.len());
				for u in uses {
					match parser
						.try_get_icon_from_default_props_use(u)
						.await
					{
						Ok(icon) => ret.push(icon),
						Err(e) => warn!(
							"Failed to scrape icon from module {parser_id}, {:?}",
							e.with_local_source(
								parser.source,
								&format!("{parser_id}.js")
							)
						),
					}
				}
			}
			.instrument(info_span!("scrape_icons", %parser_id))
			.await;
		}
		Ok(ret)
	}

	#[expect(clippy::future_not_send)]
	async fn jsx_to_dom(
		&self,
		jsx: &jsx::Call<'ast>,
		cx: &mut DomCx<'ast>,
	) -> PResult<dom::Node> {
		let tag = match &jsx.tag {
			jsx::Tag::Dom(tag) => tag.value.to_smolstr(),
			jsx::Tag::Component(c) => {
				return Err(err(*c, "JSX component tags are not supported"));
			}
		};
		let mut props = HashMap::new();
		let mut children = Vec::new();
		for prop in &jsx.props.properties {
			// TODO: resolve object spread values
			let ObjectPropertyKind::ObjectProperty(prop) = prop else {
				cx.issue(prop, "JSX spread props are not supported");
				continue;
			};
			let key = if let Some(k) = &prop.key.static_name() {
				k.to_smolstr()
			} else {
				let key = prop.key.as_expression().unwrap();
				let cv = match self.resolve_constant_value(key).await {
					Ok(cv) => cv,
					Err(e) => {
						cx.diag(e, key);
						continue;
					}
				};
				let ConstantValue::String(s) = cv.value else {
					cx.issue(
						&cv,
						"Resolved constant value of jsx prop key is not a string",
					);
					continue;
				};
				SmolStr::from(s)
			};
			if key == "children" {
				self.collect_children(&prop.value, cx, &mut children)
					.await;
				continue;
			}
			// checked before resolving, so a prop with a default is not
			// mistaken for that default
			if COLOR_ATTRS.contains(&key.as_str())
				&& let Some(sym) = self.color_param(&prop.value, &cx.props)
			{
				let default = self.color_param_default(sym, cx).await;
				props.insert(
					key,
					dom::AttrValue::ColorParam {
						span: prop.value.span(),
						default,
					},
				);
				continue;
			}
			match self
				.resolve_constant_value(&prop.value)
				.await
			{
				Ok(v) => {
					props.insert(key, dom::AttrValue::Constant(v));
				}
				Err(e) => cx.diag(e, &prop.value),
			}
		}
		Ok(dom::Node {
			attrs: props,
			children,
			tag,
		})
	}

	/// Converts the `children` prop of a jsx call, recursing into arrays
	///
	/// `null`, `undefined` and booleans render nothing, so are skipped
	#[expect(clippy::future_not_send)]
	async fn collect_children(
		&self,
		expr: &'ast Expression<'ast>,
		cx: &mut DomCx<'ast>,
		out: &mut Vec<dom::Child>,
	) {
		let expr = expr.get_inner_expression();
		if let Expression::ArrayExpression(arr) = expr {
			for el in &arr.elements {
				let Some(el) = el.as_expression() else {
					cx.issue(el, "Spread or hole among JSX children");
					continue;
				};
				Box::pin(self.collect_children(el, cx, out)).await;
			}
			return;
		}
		if let Some(jsx) = Self::is_jsx_call(expr.into_ast_kind()) {
			match Box::pin(self.jsx_to_dom(&jsx, cx)).await {
				Ok(node) => out.push(dom::Child::Element(node)),
				Err(e) => cx.diag(e, expr),
			}
			return;
		}
		match self.resolve_constant_value(expr).await {
			Ok(v) => match v.value {
				ConstantValue::Null
				| ConstantValue::Undefined
				| ConstantValue::Boolean(_) => {}
				ConstantValue::String(_) | ConstantValue::Number(_) => {
					out.push(dom::Child::Text(v));
				}
				_ => cx.issue(&v, "JSX child is not a renderable constant"),
			},
			Err(e) => cx.diag(e, expr),
		}
	}

	/// Collects the bindings of the props of the component enclosing
	/// `node_id`, mapped to their default value if they have one
	///
	/// Covers the props parameter itself, destructuring it in the parameter
	/// list, and destructuring it with a top level `let`:
	/// ```js
	/// e => {
	///     let { color: i = token, secondaryColor: s = "transparent" } = e;
	/// }
	/// ```
	fn component_props(
		&self,
		node_id: NodeId,
	) -> HashMap<SymbolId, Option<&'ast Expression<'ast>>> {
		let mut props = HashMap::new();
		let Some((params, body)) = self.find_parent(node_id, |n| match n {
			AstKind::Function(f) => Some((&*f.params, f.body.as_deref())),
			AstKind::ArrowFunctionExpression(f) => {
				Some((&*f.params, f.body.as_function_body()))
			}
			_ => None,
		}) else {
			return props;
		};
		for param in &params.items {
			Self::collect_prop_bindings(&param.pattern, &mut props);
		}
		let Some(body) = body else {
			return props;
		};
		for stmt in &body.statements {
			let Statement::VariableDeclaration(decl) = stmt else {
				continue;
			};
			for declarator in &decl.declarations {
				let is_props = declarator
					.init
					.as_ref()
					.and_then(|init| {
						init.get_inner_expression()
							.as_identifier()
					})
					.and_then(|init| self.sym_id_of(init))
					.is_some_and(|sym| props.contains_key(&sym));
				if is_props {
					Self::collect_prop_bindings(&declarator.id, &mut props);
				}
			}
		}
		props
	}

	fn collect_prop_bindings(
		pattern: &'ast BindingPattern<'ast>,
		out: &mut HashMap<SymbolId, Option<&'ast Expression<'ast>>>,
	) {
		match pattern {
			BindingPattern::BindingIdentifier(id) => {
				out.insert(id.symbol_id(), None);
			}
			BindingPattern::ObjectPattern(obj) => {
				for prop in &obj.properties {
					match &prop.value {
						BindingPattern::BindingIdentifier(id) => {
							out.insert(id.symbol_id(), None);
						}
						BindingPattern::AssignmentPattern(with_default) => {
							if let BindingPattern::BindingIdentifier(id) =
								&with_default.left
							{
								out.insert(
									id.symbol_id(),
									Some(&with_default.right),
								);
							}
						}
						_ => {}
					}
				}
			}
			_ => {}
		}
	}

	/// If `expr` is built only from the component's props, returns the prop
	/// it is made of
	///
	/// Matches identifiers, `.css`-style member reads, `c || "currentColor"`
	/// and the `"string" == typeof c ? c : c.css` choice between a color
	/// string and a design token
	fn color_param(
		&self,
		expr: &Expression<'ast>,
		props: &HashMap<SymbolId, Option<&'ast Expression<'ast>>>,
	) -> Option<SymbolId> {
		match expr.get_inner_expression() {
			Expression::Identifier(id) => self
				.sym_id_of(&**id)
				.filter(|sym| props.contains_key(sym)),
			Expression::StaticMemberExpression(mem) => {
				self.color_param(&mem.object, props)
			}
			Expression::ConditionalExpression(cond) => {
				if !Self::is_typeof_test(&cond.test) {
					return None;
				}
				let sym = self.color_param(&cond.consequent, props)?;
				self.color_param(&cond.alternate, props)?;
				Some(sym)
			}
			Expression::LogicalExpression(logical) => {
				let sym = self.color_param(&logical.left, props)?;
				let right_ok = self
					.color_param(&logical.right, props)
					.is_some()
					|| logical
						.right
						.get_inner_expression()
						.as_string_literal()
						.is_some_and(|s| s.value == "currentColor");
				right_ok.then_some(sym)
			}
			_ => None,
		}
	}

	/// `"string" == typeof c`, in either operand order
	fn is_typeof_test(test: &Expression<'ast>) -> bool {
		let Some(bin) = test
			.get_inner_expression()
			.as_binary_expression()
		else {
			return false;
		};
		if !matches!(
			bin.operator,
			BinaryOperator::Equality | BinaryOperator::StrictEquality
		) {
			return false;
		}
		let is_typeof = |e: &Expression<'_>| {
			e.get_inner_expression()
				.as_unary_expression()
				.is_some_and(|u| u.operator == UnaryOperator::Typeof)
		};
		let is_string = |e: &Expression<'_>| {
			e.get_inner_expression()
				.as_string_literal()
				.is_some()
		};
		(is_typeof(&bin.left) && is_string(&bin.right))
			|| (is_string(&bin.left) && is_typeof(&bin.right))
	}

	/// The default of the prop `sym`, if it resolves to a string
	#[expect(clippy::future_not_send)]
	async fn color_param_default(
		&self,
		sym: SymbolId,
		cx: &DomCx<'ast>,
	) -> Option<SpannedValue> {
		let default = (*cx.props.get(&sym)?)?;
		let value = self
			.resolve_constant_value(default)
			.await
			.ok()?;
		matches!(value.value, ConstantValue::String(_)).then_some(value)
	}

	#[expect(clippy::future_not_send)]
	pub(super) async fn try_get_icon_from_default_props_use(
		&self,
		node: AstKind<'ast>,
	) -> PResult<Icon> {
		// find the parent that is an svg jsx call
		let par = self
			.find_parent(node.node_id(), |n| {
				Self::is_jsx_call(n).filter(
					|jsx| matches!(jsx.tag, jsx::Tag::Dom(tag) if tag.value == "svg"),
				)
			})
			.ok_or_else(|| {
				err(
					&node,
					"default props use does not have a parent who is an SVG jsx call",
				)
			})?;
		let mut cx = DomCx {
			issues: Vec::new(),
			props: self.component_props(node.node_id()),
		};
		let dom = self.jsx_to_dom(&par, &mut cx).await?;
		let name = match self.extract_icon_name(&par).await {
			Ok(n) => n,
			Err(e) => {
				let e = e.with_local_source(self.source, "module.js");
				debug!("Failed to extract icon name: {:?}", e);
				None
			}
		};
		Ok(Icon {
			name,
			defined_in: self.get_module_id()?,
			node: dom,
			span: par.span(),
			issues: cx.issues,
		})
	}

	fn functionish_sym_id(&self, func: Functionish) -> Option<SymbolId> {
		match func {
			Functionish::Named(f) if f.id.is_some() => Some(
				self.sym_id_of(f.id.as_ref().unwrap())
					.unwrap(),
			),
			func => {
				if let AstKind::VariableDeclarator(p) = self.p(func.node_id()) {
					p.id.get_binding_identifier()
						.map(BindingIdentifier::symbol_id)
				} else {
					None
				}
			}
		}
	}
	#[expect(clippy::future_not_send)]
	async fn extract_icon_name(
		&self,
		call: &jsx::Call<'ast>,
	) -> PResult<Option<SmolStr>> {
		let func = self
			.find_parent(call.tag.node_id(), |n| match n {
				AstKind::Function(f) => Some(Functionish::Named(f)),
				AstKind::ArrowFunctionExpression(f) => {
					Some(Functionish::Arrow(f))
				}
				_ => None,
			})
			.ok_or_else(|| {
				err(&call.tag, "Failed to find component function from tag")
			})?;
		let sym = self
			.functionish_sym_id(func)
			.ok_or_else(|| err(&func, "could not find binding ident"))?;
		let raw_map = self.get_export_map_raw();
		let exported_key = raw_map
			.exports
			.iter()
			.find_map(|(k, v)| {
				let v = v.try_unwrap_range_ref().ok()?;
				if v.iter()
					.any(|r| self.sym_id_of(r) == Some(sym))
				{
					Some(k.clone())
				} else {
					None
				}
			});
		if let Some(key) = exported_key {
			let users = self
				.find_importers(vec![ExportMapKey::Named(key)])
				.await?;
			for user in users {
				let [ExportMapKey::Named(name)] = user.export_name.as_slice()
				else {
					continue;
				};
				if name.ends_with("Icon") {
					return Ok(Some(name.clone()));
				} else if name.len() > 3 {
					warn!(
						"Icon component exported as {name}, which does not end with 'Icon'"
					);
				}
				let parser = user.parser.parser();
				for node in parser
					.get_raw_uses_of_import(user.imported_id, &user.export_name)
				{
					if let Some(name) = parser.name_from_node(node) {
						return Ok(Some(name));
					}
				}
			}
		} else {
			for node in self.ref_nodes(sym) {
				if let Some(name) = self.name_from_node(node) {
					return Ok(Some(name));
				}
			}
		}
		Ok(None)
	}
	fn name_from_node(&self, node: AstKind<'ast>) -> Option<SmolStr> {
		if let Some(prop) =
			self.find_parent(node.node_id(), AstKind::as_object_property)
		{
			let key = prop.key.static_name()?;
			if key.ends_with("Icon") {
				return Some(key.to_smolstr());
			} else if key.len() > 3 {
				warn!(
					"Icon component exported as {key}, which does not end with 'Icon'"
				);
			}
		} else if let Some(case) =
			self.find_parent(node.node_id(), AstKind::as_switch_case)
			&& let Some(name) = try {
				let sme = case
					.test
					.as_ref()?
					.as_static_member_expression()?;
				let prop_name = sme.property.name.as_str();
				(prop_name.len() > 3).then(|| prop_name.to_smolstr())?
			} {
			return Some(name);
		}
		None
	}
	/// Returns the export name of the default icon props function, if this
	/// module is the default icon props module
	pub fn default_icon_props_export(&self) -> Option<SmolStr> {
		let raw_map = self.get_export_map_raw();
		if raw_map.exports.len() != 1 {
			return None;
		}
		let (k, ExportValue::Range(ExportRange(v, _))) =
			raw_map.exports.iter().next()?
		else {
			return None;
		};
		let func = v.last()?;
		let function =
			self.find_parent(func.node_id(), AstKind::as_function)?;
		let [param] = function.params.items.as_slice() else {
			return None;
		};
		if function.params.rest.is_some() {
			return None;
		}
		let props = param
			.pattern
			.get_binding_identifier()?
			.symbol_id();

		let [
			Statement::VariableDeclaration(decl),
			Statement::ReturnStatement(ret),
		] = function
			.body
			.as_ref()?
			.statements
			.as_slice()
		else {
			return None;
		};

		// Matches
		// ```js
		// function _(e) {
		//     let t = null != e["aria-label"];
		//     return e["aria-hidden"] = e["aria-hidden"] ?? !t,
		//     e.role = e.role ?? "img",
		//     e
		// }
		// ```

		// let t = null != e["aria-label"];
		let [declarator] = decl.declarations.as_slice() else {
			return None;
		};
		let has_label = declarator
			.id
			.get_binding_identifier()?
			.symbol_id();
		let label_check = declarator
			.init
			.as_ref()?
			.get_inner_expression()
			.as_binary_expression()?;
		if label_check.operator != BinaryOperator::Inequality {
			return None;
		}
		let ((Expression::NullLiteral(_), label_access)
		| (label_access, Expression::NullLiteral(_))) = (
			label_check.left.get_inner_expression(),
			label_check.right.get_inner_expression(),
		)
		else {
			return None;
		};
		if !self.is_prop_access(label_access, props, "aria-label") {
			return None;
		}

		// e["aria-hidden"] = e["aria-hidden"] ?? !t, e.role = e.role ?? "img", e
		let [hidden, role, ret_props] = ret
			.argument
			.as_ref()?
			.get_inner_expression()
			.as_sequence_expression()?
			.expressions
			.as_slice()
		else {
			return None;
		};

		let hidden_default =
			self.match_coalesce_assign(hidden, props, "aria-hidden")?;
		let not_label = hidden_default.as_unary_expression()?;
		if not_label.operator != UnaryOperator::LogicalNot {
			return None;
		}
		let not_label_ref = not_label
			.argument
			.get_inner_expression()
			.as_identifier()?;
		if !self.cmp_sym(not_label_ref, &has_label) {
			return None;
		}

		let role_default = self.match_coalesce_assign(role, props, "role")?;
		let role_value = role_default.as_string_literal()?;
		if role_value.value != "img" {
			return None;
		}

		let ret_props = ret_props
			.get_inner_expression()
			.as_identifier()?;
		self.cmp_sym(ret_props, &props)
			.then(|| k.clone())
	}

	fn is_jsx_call(node: AstKind<'ast>) -> Option<jsx::Call<'ast>> {
		let call = node.as_call_expression()?;
		let callee = call
			.callee
			.get_inner_expression()
			.as_sequence_expression()?;
		let [
			Expression::NumericLiteral(zero),
			Expression::StaticMemberExpression(jsx_rt),
		] = callee.expressions.as_slice()
		else {
			return None;
		};
		let rt_name = jsx_rt.property.name.as_str();
		if !jsx_rt.object.is_identifier_reference()
			|| !matches!(rt_name, "jsx" | "jsxs")
			|| zero.value != 0.
		{
			return None;
		}
		let args = call.arguments.as_slice();
		Self::parse_jsx_args(args)
	}

	fn parse_jsx_args(args: &'ast [Argument<'ast>]) -> Option<jsx::Call<'ast>> {
		if !matches!(args.len(), 2 | 3) {
			return None;
		}
		let tag = Self::parse_jsx_tag(args[0].as_expression()?);
		let props = Self::parse_jsx_props(args[1].as_expression()?)?;
		let key = args
			.get(2)
			.and_then(Argument::as_expression);
		Some(jsx::Call { tag, props, key })
	}

	fn parse_jsx_tag(expr: &'ast Expression<'ast>) -> jsx::Tag<'ast> {
		match expr {
			Expression::StringLiteral(lit) => jsx::Tag::Dom(lit),
			_ => jsx::Tag::Component(expr),
		}
	}

	fn parse_jsx_props(
		expr: &'ast Expression<'ast>,
	) -> Option<&'ast ObjectExpression<'ast>> {
		expr.as_object_expression()
	}

	/// Matches `obj[prop] = obj[prop] ?? <default>` and returns `<default>`
	fn match_coalesce_assign(
		&self,
		expr: &'ast Expression<'ast>,
		obj: SymbolId,
		prop: &str,
	) -> Option<&'ast Expression<'ast>> {
		let AssignmentExpression {
			operator: AssignmentOperator::Assign,
			left,
			right,
			..
		} = expr
			.get_inner_expression()
			.as_assignment_expression()?
		else {
			return None;
		};
		if !self.is_member_prop_access(left.as_member_expression()?, obj, prop)
		{
			return None;
		}
		let coalesce = right
			.get_inner_expression()
			.as_logical_expression()?;
		if coalesce.operator != LogicalOperator::Coalesce
			|| !self.is_prop_access(&coalesce.left, obj, prop)
		{
			return None;
		}
		Some(coalesce.right.get_inner_expression())
	}

	/// Whether `expr` is `obj.prop` or `obj["prop"]`
	fn is_prop_access(
		&self,
		expr: &Expression<'ast>,
		obj: SymbolId,
		prop: &str,
	) -> bool {
		expr.get_inner_expression()
			.as_member_expression()
			.is_some_and(|mem| self.is_member_prop_access(mem, obj, prop))
	}

	fn is_member_prop_access(
		&self,
		mem: &MemberExpression<'ast>,
		obj: SymbolId,
		prop: &str,
	) -> bool {
		if mem.optional() || mem.static_property_name() != Some(prop) {
			return false;
		}
		let Some(obj_ref) = mem
			.object()
			.get_inner_expression()
			.as_identifier()
		else {
			return false;
		};
		self.cmp_sym(obj_ref, &obj)
	}
}
