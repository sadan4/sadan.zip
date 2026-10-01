//! Primitive JS values and the spec's type conversions on them, shared by
//! the constant folding of operators

use std::borrow::Cow;

use num_bigint::BigInt;
use oxc::syntax::number::ToJsString;
use oxc_ecmascript::StringToNumber;

use super::ConstantValue;

pub(super) type FoldResult<T> = Result<T, Cow<'static, str>>;

/// A borrowed primitive JS value
#[derive(Debug, Clone, Copy)]
pub(super) enum Primitive<'v> {
	Number(f64),
	BigInt(&'v BigInt),
	Boolean(bool),
	String(&'v str),
	Undefined,
	Null,
}

/// The result of `ToNumeric`
pub(super) enum Numeric<'v> {
	Number(f64),
	BigInt(&'v BigInt),
}

impl<'v> Primitive<'v> {
	/// Returns [`None`] for objects
	pub(super) fn new<T>(value: &'v ConstantValue<T>) -> Option<Self> {
		Some(match value {
			ConstantValue::Number(n) => Self::Number(n.into_inner()),
			ConstantValue::NaN => Self::Number(f64::NAN),
			ConstantValue::BigInt(n) => Self::BigInt(n),
			ConstantValue::Boolean(b) => Self::Boolean(*b),
			ConstantValue::String(s) => Self::String(s),
			ConstantValue::Undefined => Self::Undefined,
			ConstantValue::Null => Self::Null,
			ConstantValue::Array(_)
			| ConstantValue::Object(_)
			| ConstantValue::Set(_) => return None,
		})
	}

	/// <https://tc39.es/ecma262/#sec-tonumber>
	pub(super) fn to_number(self) -> FoldResult<f64> {
		Ok(match self {
			Self::Number(n) => n,
			Self::BigInt(_) => {
				return Err("cannot convert a BigInt value to a number".into());
			}
			Self::Boolean(b) => f64::from(u8::from(b)),
			Self::String(s) => s.string_to_number(),
			Self::Undefined => f64::NAN,
			Self::Null => 0.,
		})
	}

	/// <https://tc39.es/ecma262/#sec-tonumeric>
	pub(super) fn to_numeric(self) -> FoldResult<Numeric<'v>> {
		match self {
			Self::BigInt(n) => Ok(Numeric::BigInt(n)),
			other => other.to_number().map(Numeric::Number),
		}
	}

	/// <https://tc39.es/ecma262/#sec-tostring>
	pub(super) fn to_js_string(self) -> Cow<'v, str> {
		match self {
			Self::Number(n) => n.to_js_string().into(),
			Self::BigInt(n) => n.to_string().into(),
			Self::Boolean(b) => if b { "true" } else { "false" }.into(),
			Self::String(s) => s.into(),
			Self::Undefined => "undefined".into(),
			Self::Null => "null".into(),
		}
	}
}
