//! Constant folding of unary expressions, following the ECMAScript spec

use oxc::ast::ast::UnaryOperator;
use oxc_ecmascript::ToInt32;

use super::{
	ConstantValue,
	primitive::{FoldResult, Numeric, Primitive},
};

/// Folds `<op> value`, returning a human readable reason on failure
pub(super) fn fold_unary<T>(
	op: UnaryOperator,
	value: &ConstantValue<T>,
) -> FoldResult<ConstantValue<T>> {
	match op {
		UnaryOperator::LogicalNot => {
			return Ok(ConstantValue::Boolean(value.is_falsy()));
		}
		UnaryOperator::Typeof => {
			return Ok(ConstantValue::String(type_of(value).to_owned()));
		}
		UnaryOperator::Void => return Ok(ConstantValue::Undefined),
		UnaryOperator::Delete => {
			return Err("the `delete` operator is not supported for constant \
			            values"
				.into());
		}
		_ => {}
	}
	let Some(value) = Primitive::new(value) else {
		return Err(format!(
			"the `{}` operator is only supported on primitive values",
			op.as_str()
		)
		.into());
	};
	Ok(match op {
		UnaryOperator::UnaryPlus => ConstantValue::from(value.to_number()?),
		UnaryOperator::UnaryNegation => match value.to_numeric()? {
			Numeric::Number(n) => ConstantValue::from(-n),
			Numeric::BigInt(n) => ConstantValue::BigInt(-n),
		},
		UnaryOperator::BitwiseNot => match value.to_numeric()? {
			Numeric::Number(n) => {
				ConstantValue::from(f64::from(!n.to_int_32()))
			}
			// `!n` is `-n - 1` for num-bigint, the same as JS
			Numeric::BigInt(n) => ConstantValue::BigInt(!n),
		},
		_ => unreachable!("{op:?} is handled above"),
	})
}

/// <https://tc39.es/ecma262/#sec-typeof-operator>
///
/// Functions are never constant values, so `"function"` is never returned
const fn type_of<T>(value: &ConstantValue<T>) -> &'static str {
	match value {
		ConstantValue::Number(_) | ConstantValue::NaN => "number",
		ConstantValue::BigInt(_) => "bigint",
		ConstantValue::Boolean(_) => "boolean",
		ConstantValue::String(_) => "string",
		ConstantValue::Undefined => "undefined",
		ConstantValue::Null
		| ConstantValue::Array(_)
		| ConstantValue::Object(_)
		| ConstantValue::Set(_) => "object",
	}
}
