use ast_parser::exts::{PropertyKeyExt, StringLiteralExt};
use oxc::{
	ast::ast::{
		ArrowFunctionExpression,
		BinaryExpression,
		BinaryOperator,
		ConditionalExpression,
		Expression,
		FormalParameters,
		Function,
		LogicalOperator,
		ObjectExpression,
		ObjectProperty,
		ObjectPropertyKind,
		Statement,
		StringLiteral,
		TemplateLiteral,
		VariableDeclaration,
	},
	ast_visit::{Visit, walk::walk_object_expression},
	span::GetSpan,
};
use parser_diag::{PResult, err};
use serde::Serialize;
use tracing::warn;

use crate::WebpackAstParser;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
pub enum EndpointParam {
	Required,
	/// `default` is the string value if the default is a string literal,
	/// otherwise the source text of the default expression
	Optional {
		default: String,
	},
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
pub enum Segment {
	Literal {
		value: String,
	},
	/// Index into [`Endpoint::params`]
	Param {
		index: usize,
	},
	/// A param passed through the global `escape` function
	EscapedParam {
		index: usize,
	},
	/// A value read from `window`, eg: `window.GLOBAL_ENV.WEBAPP_ENDPOINT`
	///
	/// `path` is the source text of the member expression, including window
	Global {
		path: String,
	},
	/// `consequent` is used when the param is not nullish, `absent` otherwise
	Conditional {
		param: usize,
		consequent: Vec<Self>,
		alternate: Vec<Self>,
	},
}

#[derive(Debug, Serialize)]
pub struct Endpoint {
	pub name: String,
	pub value: Vec<Segment>,
	pub params: Vec<EndpointParam>,
	/// Whether the endpoint is a function, as opposed to a plain string.
	///
	/// Needed to tell `FOO: "/foo"` apart from `FOO: () => "/foo"`
	pub is_function: bool,
}

impl Endpoint {
	// TODO: come up with better name
	pub fn r#static(name: String, value: String) -> Self {
		Self {
			name,
			value: vec![Segment::Literal { value }],
			params: vec![],
			is_function: false,
		}
	}
}

struct Visitor<'ast> {
	cur: Option<&'ast ObjectExpression<'ast>>,
}

impl<'ast> Visit<'ast> for Visitor<'ast> {
	fn visit_object_expression(&mut self, obj: &ObjectExpression<'ast>) {
		let want_keys = [
			"GUILD_CHANNELS",
			"GUILD_MEMBER_NICK",
			"MESSAGE_ACK",
			"USERS",
			"ME",
		];
		let found = want_keys
			.into_iter()
			.all(|k| obj.find_property(k).is_some());
		if found {
			if self.cur.is_some() {
				warn!("found duplicate endpoint objects");
			}
			self.cur = Some(self.alloc(obj));
		}
		walk_object_expression(self, obj);
	}
}

/// Names of the params of the endpoint function, in order
struct ParamNames<'ast>(Vec<&'ast str>);

impl ParamNames<'_> {
	fn index_of(&self, expr: &Expression) -> Option<usize> {
		let Expression::Identifier(ident) = expr.without_parentheses() else {
			return None;
		};
		self.0
			.iter()
			.position(|name| *name == ident.name.as_str())
	}
}

fn push_literal(segments: &mut Vec<Segment>, lit: &str) {
	if lit.is_empty() {
		return;
	}
	if let Some(Segment::Literal { value }) = segments.last_mut() {
		value.push_str(lit);
	} else {
		segments.push(Segment::Literal {
			value: lit.to_owned(),
		});
	}
}

/// Whether `expr` is a static member expression chain rooted at `window`
fn is_window_access(expr: &Expression) -> bool {
	match expr.without_parentheses() {
		Expression::StaticMemberExpression(member) => {
			is_window_access(&member.object)
		}
		other => other.is_specific_id("window"),
	}
}

impl<'ast> WebpackAstParser<'ast> {
	pub fn as_endpoints_module(&self) -> Option<PResult<Vec<Endpoint>>> {
		let obj = self.as_raw_endpoints_module()?;
		let mut ret = Vec::with_capacity(obj.properties.len());
		for prop in &obj.properties {
			let ObjectPropertyKind::ObjectProperty(prop) = prop else {
				return Some(Err(err(
					prop,
					"Endpoint module does not have spread properties. this is a bug in module detection.",
				)));
			};
			ret.push(match self.handle_prop(prop) {
				Ok(p) => p,
				Err(e) => {
					return Some(Err(e));
				}
			});
		}
		Some(Ok(ret))
	}
	fn handle_prop(
		&self,
		prop: &'ast ObjectProperty<'ast>,
	) -> PResult<Endpoint> {
		let name = prop
			.key
			.static_name_utf8()
			.ok_or_else(|| {
				err(&prop.key, "Property does not have a static name")
			})??
			.into_owned();
		match &prop.value {
			Expression::StringLiteral(lit) => {
				Self::handle_string_lit(name, lit)
			}
			Expression::ArrowFunctionExpression(func) => {
				self.handle_arrow_func(name, func)
			}
			Expression::FunctionExpression(func) => {
				self.handle_func(name, func)
			}
			other => Err(err(
				other,
				"Expected string literal or function expression",
			)),
		}
		.map_err(|e| err(prop, "Failed to parse endpoint").s(e))
	}
	fn as_raw_endpoints_module(&self) -> Option<&'ast ObjectExpression<'ast>> {
		let mut visitor = Visitor { cur: None };
		visitor.visit_program(self.prog);
		visitor.cur
	}

	fn handle_func(
		&self,
		name: String,
		func: &'ast Function<'ast>,
	) -> PResult<Endpoint> {
		let body = func
			.body
			.as_ref()
			.ok_or_else(|| err(func, "Function does not have a body"))?;
		self.handle_func_body(name, &func.params, &body.statements)
	}

	fn handle_arrow_func(
		&self,
		name: String,
		func: &'ast ArrowFunctionExpression<'ast>,
	) -> PResult<Endpoint> {
		if let Some(expr) = func.get_expression() {
			let (names, params) = Self::parse_params(&func.params)?;
			let value = self.parse_value(&names, expr)?;
			return Ok(Endpoint {
				name,
				value,
				params,
				is_function: true,
			});
		}
		let body = func
			.get_function_body()
			.ok_or_else(|| err(func, "Arrow function does not have a body"))?;
		self.handle_func_body(name, &func.params, &body.statements)
	}

	/// Handles a function body of the shape
	/// ```js
	/// let n = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : "jpg";
	/// return `/guilds/${e}/icons/${t}.${n}`
	/// ```
	/// where there are zero or more optional param declarations
	fn handle_func_body(
		&self,
		name: String,
		formal_params: &'ast FormalParameters<'ast>,
		stmts: &'ast [Statement<'ast>],
	) -> PResult<Endpoint> {
		let (mut names, mut params) = Self::parse_params(formal_params)?;
		let Some((Statement::ReturnStatement(ret), decls)) = stmts.split_last()
		else {
			return Err(err(
				formal_params,
				"Expected function body to end in a return statement",
			));
		};
		for decl in decls {
			let Statement::VariableDeclaration(decl) = decl else {
				return Err(err(
					decl,
					"Expected optional param declaration in endpoint function",
				));
			};
			let (name, param) =
				self.parse_optional_param(decl, names.0.len())?;
			names.0.push(name);
			params.push(param);
		}
		let arg = ret
			.argument
			.as_ref()
			.ok_or_else(|| err(&**ret, "Return statement has no argument"))?;
		let value = self.parse_value(&names, arg)?;
		Ok(Endpoint {
			name,
			value,
			params,
			is_function: true,
		})
	}

	fn parse_params(
		formal_params: &'ast FormalParameters<'ast>,
	) -> PResult<(ParamNames<'ast>, Vec<EndpointParam>)> {
		if let Some(rest) = &formal_params.rest {
			return Err(err(&**rest, "Rest params are not supported"));
		}
		let names = formal_params
			.items
			.iter()
			.map(|param| {
				if param.initializer.is_some() {
					return Err(err(param, "Param defaults are not supported"));
				}
				param
					.pattern
					.get_binding_identifier()
					.map(|ident| ident.name.as_str())
					.ok_or_else(|| err(param, "Param is not an identifier"))
			})
			.collect::<PResult<Vec<_>>>()?;
		let params = vec![EndpointParam::Required; names.len()];
		Ok((ParamNames(names), params))
	}

	/// Parses
	/// ```js
	/// let n = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : "jpg";
	/// ```
	/// where `2` must equal `index`
	fn parse_optional_param(
		&self,
		decl: &'ast VariableDeclaration<'ast>,
		index: usize,
	) -> PResult<(&'ast str, EndpointParam)> {
		let [declarator] = decl.declarations.as_slice() else {
			return Err(err(decl, "Expected exactly one declarator"));
		};
		let name = declarator
			.id
			.get_binding_identifier()
			.ok_or_else(|| err(&declarator.id, "Expected an identifier"))?
			.name
			.as_str();
		let Some(Expression::ConditionalExpression(cond)) =
			declarator.init.as_ref()
		else {
			return Err(err(
				declarator,
				"Expected optional param to be initialized with a conditional",
			));
		};
		let arg_index =
			Self::arguments_index(&cond.consequent).ok_or_else(|| {
				err(&cond.consequent, "Expected `arguments[N]` expression")
			})?;
		if arg_index != index {
			return Err(err(
				&cond.consequent,
				format!(
					"Expected optional param to be argument {index}, found {arg_index}"
				),
			));
		}
		let default = match cond.alternate.without_parentheses() {
			Expression::StringLiteral(lit) => lit.as_utf8()?.to_owned(),
			other => other
				.span()
				.source_text(self.source)
				.to_owned(),
		};
		Ok((name, EndpointParam::Optional { default }))
	}

	/// Returns `N` for `arguments[N]`
	fn arguments_index(expr: &Expression) -> Option<usize> {
		let Expression::ComputedMemberExpression(member) =
			expr.without_parentheses()
		else {
			return None;
		};
		if !member
			.object
			.is_specific_id("arguments")
		{
			return None;
		}
		let Expression::NumericLiteral(lit) = &member.expression else {
			return None;
		};
		#[expect(
			clippy::cast_possible_truncation,
			clippy::cast_sign_loss,
			reason = "checked to be a small non-negative integer"
		)]
		(lit.value.fract() == 0.0 && (0.0..1024.0).contains(&lit.value))
			.then_some(lit.value as usize)
	}

	fn parse_value(
		&self,
		names: &ParamNames<'ast>,
		expr: &'ast Expression<'ast>,
	) -> PResult<Vec<Segment>> {
		let mut ret = Vec::new();
		self.parse_segments(names, expr, &mut ret)?;
		Ok(ret)
	}

	fn parse_segments(
		&self,
		names: &ParamNames<'ast>,
		expr: &'ast Expression<'ast>,
		out: &mut Vec<Segment>,
	) -> PResult<()> {
		let expr = expr.without_parentheses();
		if let Some(index) = names.index_of(expr) {
			out.push(Segment::Param { index });
			return Ok(());
		}
		match expr {
			Expression::StringLiteral(lit) => {
				push_literal(out, lit.as_utf8()?);
			}
			Expression::NumericLiteral(lit) => {
				push_literal(out, lit.span.source_text(self.source));
			}
			Expression::TemplateLiteral(tmpl) => {
				self.parse_template(names, tmpl, out)?;
			}
			// `${window.GLOBAL_ENV.WEBAPP_ENDPOINT}`
			Expression::StaticMemberExpression(_) if is_window_access(expr) => {
				out.push(Segment::Global {
					path: expr
						.span()
						.source_text(self.source)
						.to_owned(),
				});
			}
			// `${escape(t)}`
			Expression::CallExpression(call)
				if call.callee.is_specific_id("escape") =>
			{
				let [arg] = call.arguments.as_slice() else {
					return Err(err(
						&**call,
						"Expected one argument to `escape`",
					));
				};
				let index = arg
					.as_expression()
					.and_then(|arg| names.index_of(arg))
					.ok_or_else(|| {
						err(arg, "Expected a param passed to `escape`")
					})?;
				out.push(Segment::EscapedParam { index });
			}
			// `${n ?? 0}`
			Expression::LogicalExpression(logical)
				if logical.operator == LogicalOperator::Coalesce =>
			{
				let param = names
					.index_of(&logical.left)
					.ok_or_else(|| {
						err(
							&logical.left,
							"Expected a param on the left of `??`",
						)
					})?;
				out.push(Segment::Conditional {
					param,
					consequent: vec![Segment::Param { index: param }],
					alternate: self.parse_value(names, &logical.right)?,
				});
			}
			// `null != n ? `/${n}` : ""`
			Expression::ConditionalExpression(cond) => {
				out.push(self.parse_conditional(names, cond)?);
			}
			other => {
				return Err(err(
					other,
					"Unsupported expression in endpoint value",
				));
			}
		}
		Ok(())
	}

	fn parse_template(
		&self,
		names: &ParamNames<'ast>,
		tmpl: &'ast TemplateLiteral<'ast>,
		out: &mut Vec<Segment>,
	) -> PResult<()> {
		for (i, quasi) in tmpl.quasis.iter().enumerate() {
			if i != 0 {
				self.parse_segments(names, &tmpl.expressions[i - 1], out)?;
			}
			let cooked = quasi
				.value
				.cooked
				.as_ref()
				.ok_or_else(|| {
					err(quasi, "template has an invalid escape sequence")
				})?
				.as_str()
				.ok_or_else(|| err(quasi, "string is not valid utf8"))?;
			push_literal(out, cooked);
		}
		Ok(())
	}

	fn parse_conditional(
		&self,
		names: &ParamNames<'ast>,
		cond: &'ast ConditionalExpression<'ast>,
	) -> PResult<Segment> {
		let Expression::BinaryExpression(test) =
			cond.test.without_parentheses()
		else {
			return Err(err(&cond.test, "Expected a nullish check"));
		};
		let (param, is_present_check) = Self::parse_nullish_check(names, test)
			.ok_or_else(|| {
				err(&**test, "Expected a nullish check on a param")
			})?;
		let (present, absent) = if is_present_check {
			(&cond.consequent, &cond.alternate)
		} else {
			(&cond.alternate, &cond.consequent)
		};
		Ok(Segment::Conditional {
			param,
			consequent: self.parse_value(names, present)?,
			alternate: self.parse_value(names, absent)?,
		})
	}

	/// Parses `null != n`, `n == null`, `void 0 !== n`, etc
	///
	/// Returns the param index and whether the check is true when the param
	/// is present
	fn parse_nullish_check(
		names: &ParamNames<'ast>,
		test: &BinaryExpression,
	) -> Option<(usize, bool)> {
		let is_present_check = match test.operator {
			BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
				true
			}
			BinaryOperator::Equality | BinaryOperator::StrictEquality => false,
			_ => return None,
		};
		let (nullish, param) = if test.left.is_null_or_undefined() {
			(&test.left, &test.right)
		} else {
			(&test.right, &test.left)
		};
		if !nullish.is_null_or_undefined() {
			return None;
		}
		Some((names.index_of(param)?, is_present_check))
	}

	fn handle_string_lit(
		name: String,
		lit: &'ast StringLiteral<'ast>,
	) -> PResult<Endpoint> {
		Ok(Endpoint::r#static(name, lit.as_utf8()?.to_owned()))
	}
}
