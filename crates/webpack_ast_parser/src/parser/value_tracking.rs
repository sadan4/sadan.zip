use std::{
	borrow::Cow,
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

use ast_parser::AstParser as _;
use explorer_types::{ModuleId, SpannedId};
use num_bigint::BigInt;
use ordered_float::NotNan;
use oxc::{
	ast::{
		ast::{
			ArrayExpressionElement,
			AssignmentOperator,
			Expression,
			IdentifierReference,
			LogicalOperator,
			UnaryOperator,
		},
		match_expression,
	},
	semantic::SymbolId,
	span::{GetSpan, Span},
};
use parser_diag::{PResult, ParserDiagnostic};
use tracing::error;

use crate::WebpackAstParser;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantPropertyKey {
	String(String),
	Number(NotNan<f64>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantValue<T> {
	Number(NotNan<f64>),
	NaN,
	BigInt(num_bigint::BigInt),
	Boolean(bool),
	String(String),
	Array(Vec<T>),
	Object(BTreeMap<ConstantPropertyKey, T>),
	Set(BTreeSet<T>),
	Undefined,
	Null,
}

impl<T> ConstantValue<T> {
	fn is_falsy(&self) -> bool {
		match self {
			Self::Number(n) => *n == 0.,
			Self::BigInt(n) => *n == BigInt::ZERO,
			Self::Boolean(b) => !*b,
			Self::String(s) => s.is_empty(),
			Self::NaN | Self::Undefined | Self::Null => true,
			Self::Array(_) | Self::Object(_) | Self::Set(_) => false,
		}
	}
	fn is_truthy(&self) -> bool {
		!self.is_falsy()
	}
	const fn is_nullish(&self) -> bool {
		matches!(self, Self::Undefined | Self::Null)
	}
}
pub struct ConstantValueCycle(ConstantValue<Self>);
pub type RawConstantValue = ConstantValue<ConstantValueCycle>;
pub type ConstantSpannedValue = ConstantValue<SpannedValue>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpannedValue {
	pub value: ConstantSpannedValue,
	/// The source location of this constant value,
	pub span: Span,
	/// The source module id of this value, so [`Self::span`] can be properly resolved
	pub module_id: ModuleId,
}

impl GetSpan for SpannedValue {
	fn span(&self) -> Span {
		self.span
	}
}

impl<T> From<f64> for ConstantValue<T> {
	fn from(value: f64) -> Self {
		if let Ok(value) = NotNan::new(value) {
			Self::Number(value)
		} else {
			debug_assert!(value.is_nan());
			Self::NaN
		}
	}
}

impl<'ast> WebpackAstParser<'ast> {
	/// <https://262.ecma-international.org/#sec-tostring>
	fn to_string<'a>(&self, v: &'a SpannedValue) -> PResult<Cow<'a, str>> {
		let argument = &v.value;
		// 1. If argument is a String, return argument.
		if let ConstantValue::String(s) = &argument {
			return Ok(Cow::Borrowed(s.as_str()));
		}
		// 2. If argument is a Symbol, throw a TypeError exception.
		// we don't support symbols
		// 3. If argument is undefined, return "undefined".
		if let ConstantValue::Undefined = &argument {
			return Ok(Cow::Borrowed("undefined"));
		}
		// 4. If argument is null, return "null".
		if let ConstantValue::Null = &argument {
			return Ok(Cow::Borrowed("null"));
		}
		// 5. If argument is true, return "true".
		if let ConstantValue::Boolean(true) = &argument {
			return Ok(Cow::Borrowed("true"));
		}
		// 6. If argument is false, return "false".
		if let ConstantValue::Boolean(false) = &argument {
			return Ok(Cow::Borrowed("false"));
		}
		// 7. If argument is a Number, return Number::toString(argument, 10).
		if let ConstantValue::Number(n) = &argument {
			return Ok(Cow::Owned(n.to_string()));
		}
		// we handle NaN separately
		if let ConstantValue::NaN = &argument {
			return Ok(Cow::Borrowed("NaN"));
		}
		// 8. If argument is a BigInt, return BigInt::toString(argument, 10).
		if let ConstantValue::BigInt(n) = &argument {
			return Ok(Cow::Owned(n.to_string()));
		}
		let ret = match argument {
			ConstantValue::Array(_) => Cow::Owned(
				self.array_prototype_join(v, None)
					.unwrap(),
			),
			ConstantValue::Object(_) => Cow::Borrowed("[object Object]"),
			ConstantValue::Set(_) => Cow::Borrowed("[object Set]"),
			// 9. Assert: argument is an Object.
			_ => unreachable!(),
		};
		Ok(ret)
	}
	/// <https://262.ecma-international.org/#sec-array.prototype.join>
	fn array_prototype_join(
		&self,
		arr: &SpannedValue,
		separator: Option<&str>,
	) -> PResult<String> {
		let ConstantValue::Array(elts) = &arr.value else {
			return Err(self.local_err(
				arr,
				"array.prototype.join called on non-array value",
			));
		};
		let mut ret = String::new();
		let sep = separator.unwrap_or(",");
		for (i, elt) in elts.iter().enumerate() {
			if i != 0 {
				ret.push_str(sep);
			}
			let str = self.to_string(elt)?;
			ret.push_str(&str);
		}
		Ok(ret)
	}
	async fn remote_err(
		&self,
		span: &impl GetSpan,
		m_id: ModuleId,
		msg: impl Into<Cow<'static, str>>,
	) -> ParserDiagnostic {
		let ret = try {
			if m_id == self.get_module_id()?.id {
				return self.local_err(span, msg);
			}
			let parser = self
				.try_get_module_parser(SpannedId::unspanned(m_id))
				.await
				.map_err(|e| {
					parser_diag::err_ns("failed to get module parser").s(e)
				})?;
			let source = Arc::clone(parser.get_source());
			let mut err = parser_diag::err(span, msg);
			err.txt = Some(Arc::new(source));
			err
		};
		match ret {
			Ok(d) => d,
			Err(e) => {
				error!("Failed to create remote error: {e:?}");
				e
			}
		}
	}
	fn local_err(
		&self,
		span: &impl GetSpan,
		msg: impl Into<Cow<'static, str>>,
	) -> ParserDiagnostic {
		let source = self.get_arc_source();
		let mut err = parser_diag::err(span, msg);
		err.txt = Some(Arc::new(source));
		err
	}
	/// Resolves the only value `ident` is ever set to
	///
	/// `resolving` holds the symbols currently being resolved, to catch
	/// cycles like `let x; x = x + 1;`
	#[expect(clippy::future_not_send)]
	async fn resolve_ident(
		&self,
		ident: &'ast IdentifierReference<'ast>,
		resolving: &mut Vec<SymbolId>,
	) -> PResult<SpannedValue> {
		let Some(sym) = self
			.sema
			.scoping()
			.get_reference(ident.reference_id())
			.symbol_id()
		else {
			let value = match ident.name.as_str() {
				"undefined" => ConstantValue::Undefined,
				"NaN" => ConstantValue::NaN,
				"Infinity" => ConstantValue::from(f64::INFINITY),
				_ => {
					return Err(
						self.local_err(ident, "unresolved global identifier")
					);
				}
			};
			return Ok(SpannedValue {
				value,
				span: ident.span,
				module_id: self.get_module_id()?.id,
			});
		};
		let decl_err = || {
			self.local_err(
				&self.sema.scoping().symbol_span(sym),
				"declared here",
			)
		};
		if resolving.contains(&sym) {
			return Err(self
				.local_err(ident, "identifier is defined in terms of itself")
				.s(decl_err()));
		}
		let Some(init) = self.constant_value_of(&sym) else {
			return Err(self
				.local_err(ident, "identifier does not have a single value")
				.s(decl_err()));
		};
		resolving.push(sym);
		let ret = Box::pin(self.resolve_value(init, resolving)).await;
		resolving.pop();
		ret
	}
	#[expect(clippy::future_not_send)]
	async fn resolve_constant_value(
		&self,
		expr: &'ast Expression<'ast>,
	) -> PResult<SpannedValue> {
		self.resolve_value(expr, &mut Vec::new())
			.await
	}
	#[expect(clippy::future_not_send)]
	async fn resolve_value(
		&self,
		expr: &'ast Expression<'ast>,
		resolving: &mut Vec<SymbolId>,
	) -> PResult<SpannedValue> {
		let ret = match expr {
			Expression::BooleanLiteral(lit) => SpannedValue {
				value: ConstantValue::Boolean(lit.value),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::NullLiteral(lit) => SpannedValue {
				value: ConstantValue::Null,
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::NumericLiteral(lit) => SpannedValue {
				value: ConstantValue::from(lit.value),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::BigIntLiteral(lit) => SpannedValue {
				value: ConstantValue::BigInt(lit.value.parse().unwrap()),
				span: lit.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::StringLiteral(str) => SpannedValue {
				value: ConstantValue::String(str.to_string()),
				span: str.span,
				module_id: self.get_module_id()?.id,
			},
			Expression::TemplateLiteral(tmpl) => {
				let mut ret = String::new();
				for (i, quasi) in tmpl.quasis.iter().enumerate() {
					if i != 0 {
						let expr = &tmpl.expressions[i - 1];
						let value =
							Box::pin(self.resolve_value(expr, resolving))
								.await?;
						let str_val = self.to_string(&value)?;
						ret.push_str(&str_val);
					}
					ret.push_str(quasi.value.cooked.as_ref().unwrap());
				}
				SpannedValue {
					value: ConstantValue::String(ret),
					span: tmpl.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::Identifier(ident) => {
				self.resolve_ident(ident, resolving)
					.await?
			}
			Expression::ArrayExpression(array_expression) => {
				let mut elements = Vec::new();
				for element in &array_expression.elements {
					match element {
						ArrayExpressionElement::SpreadElement(
							spread_element,
						) => {
							let spread_value = Box::pin(self.resolve_value(
								&spread_element.argument,
								resolving,
							))
							.await?;
							if let ConstantValue::Array(elts) =
								spread_value.value
							{
								elements.extend(elts);
							} else {
								return Err(self
									.local_err(
										&**spread_element,
										"spread element must be an array",
									)
									.s(self
										.remote_err(
											&spread_value.span,
											spread_value.module_id,
											"spread element value defined here",
										)
										.await));
							}
						}
						ArrayExpressionElement::Elision(hole) => {
							return Err(self.local_err(
								&**hole,
								"elisions are not supported",
							));
						}
						expr @ match_expression!(ArrayExpressionElement) => {
							let expr = expr.as_expression().unwrap();
							let value =
								Box::pin(self.resolve_value(expr, resolving))
									.await?;
							elements.push(value);
						}
					}
				}

				SpannedValue {
					value: ConstantValue::Array(elements),
					span: array_expression.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::AssignmentExpression(e) => match e.operator {
				AssignmentOperator::Assign => {
					let value =
						Box::pin(self.resolve_value(&e.right, resolving))
							.await?;
					SpannedValue {
						value: value.value,
						span: e.span,
						module_id: self.get_module_id()?.id,
					}
				}
				other => {
					return Err(self.local_err(
						&**e,
						format!(
							"assignment operator {other:?} is not supported for constant values"
						),
					));
				}
			},
			Expression::AwaitExpression(await_expression) => {
				todo!("resolve awaited value?");
			}
			Expression::BinaryExpression(bin) => {
				let lhs =
					Box::pin(self.resolve_value(&bin.left, resolving)).await?;
				let rhs =
					Box::pin(self.resolve_value(&bin.right, resolving)).await?;
				match binary::fold_binary(bin.operator, &lhs.value, &rhs.value)
				{
					Ok(value) => SpannedValue {
						value,
						span: bin.span,
						module_id: self.get_module_id()?.id,
					},
					Err(msg) => {
						let rhs_err = self
							.remote_err(
								&rhs.span,
								rhs.module_id,
								"right operand defined here",
							)
							.await;
						let lhs_err = self
							.remote_err(
								&lhs.span,
								lhs.module_id,
								"left operand defined here",
							)
							.await
							.s(rhs_err);
						return Err(self.local_err(&**bin, msg).s(lhs_err));
					}
				}
			}
			Expression::CallExpression(call_expression) => {
				todo!("we can resolve some calls")
			}
			Expression::ChainExpression(chain_expression) => {
				todo!("resolve last")
			}
			Expression::ConditionalExpression(cond) => {
				let test =
					Box::pin(self.resolve_value(&cond.test, resolving)).await?;
				let expr = if test.value.is_truthy() {
					&cond.consequent
				} else {
					&cond.alternate
				};
				Box::pin(self.resolve_value(expr, resolving)).await?
			}
			Expression::LogicalExpression(expr) => {
				let lhs =
					Box::pin(self.resolve_value(&expr.left, resolving)).await?;
				macro_rules! rhs {
					() => {
						Box::pin(self.resolve_value(&expr.right, resolving))
							.await?
					};
				}
				match expr.operator {
					LogicalOperator::Or => {
						if lhs.value.is_truthy() {
							lhs
						} else {
							rhs!()
						}
					}
					LogicalOperator::And => {
						if lhs.value.is_falsy() {
							lhs
						} else {
							rhs!()
						}
					}
					LogicalOperator::Coalesce => {
						if lhs.value.is_nullish() {
							rhs!()
						} else {
							lhs
						}
					}
				}
			}
			Expression::NewExpression(new_expression) => {
				todo!("resolve new set")
			}
			Expression::ObjectExpression(object_expression) => {
				todo!("object expression")
			}
			Expression::ParenthesizedExpression(e) => {
				Box::pin(self.resolve_value(&e.expression, resolving)).await?
			}
			Expression::SequenceExpression(seq) => {
				let last = seq.expressions.last().unwrap();
				Box::pin(self.resolve_value(last, resolving)).await?
			}
			Expression::TaggedTemplateExpression(
				tagged_template_expression,
			) => {
				return Err(self.local_err(
					&**tagged_template_expression,
					"tagged template expressions are not constant values",
				));
			}
			Expression::UnaryExpression(unary) => {
				let value = match (
					unary.operator,
					unary.argument.without_parentheses(),
				) {
					// the operand doesn't need to be constant
					(UnaryOperator::Void, _) => ConstantValue::Undefined,
					(
						UnaryOperator::Typeof,
						Expression::FunctionExpression(_)
						| Expression::ArrowFunctionExpression(_)
						| Expression::ClassExpression(_),
					) => ConstantValue::String("function".to_owned()),
					_ => {
						let arg = Box::pin(
							self.resolve_value(&unary.argument, resolving),
						)
						.await?;
						match unary::fold_unary(unary.operator, &arg.value) {
							Ok(value) => value,
							Err(msg) => {
								return Err(self.local_err(&**unary, msg).s(
									self.remote_err(
										&arg.span,
										arg.module_id,
										"operand defined here",
									)
									.await,
								));
							}
						}
					}
				};
				SpannedValue {
					value,
					span: unary.span,
					module_id: self.get_module_id()?.id,
				}
			}
			Expression::ComputedMemberExpression(
				computed_member_expression,
			) => {
				todo!("resolve computed member expression")
			}
			Expression::StaticMemberExpression(static_member_expression) => {
				todo!("resolve static member expression")
			}
			Expression::RegExpLiteral(regex) => {
				return Err(self.local_err(
					&**regex,
					"regular expressions are not supported as constant values",
				));
			}
			Expression::TemplateLiteral(lit) => {
				return Err(
					self.local_err(
						&**lit,
						"template literals with substitutions are not constant values",
					)
					.s(self.local_err(&lit.expressions[0], "expression here")),
				);
			}
			Expression::ArrowFunctionExpression(_)
			| Expression::FunctionExpression(_) => {
				return Err(self.local_err(
					expr,
					"function expressions are not constant values",
				));
			}
			other => {
				return Err(self.local_err(
					other,
					"unsupported expression type for constant value",
				));
			}
		};
		Ok(ret)
	}
}

mod binary;
mod primitive;
#[cfg(test)]
mod tests;
mod unary;
