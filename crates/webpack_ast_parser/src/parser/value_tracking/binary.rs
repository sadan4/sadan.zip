//! Constant folding of binary expressions, following the ECMAScript spec for
//! primitive operands

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use oxc::ast::ast::BinaryOperator;
use oxc_ecmascript::{StringToBigInt, StringToNumber, ToInt32, ToUint32};

use super::{
	ConstantValue,
	primitive::{FoldResult, Numeric, Primitive},
};

/// The largest bigint, in bits, that folding is allowed to produce, so
/// something like `2n ** 4294967295n` can't exhaust memory
const MAX_BIGINT_BITS: u64 = 1 << 20;

/// Folds `lhs <op> rhs`, returning a human readable reason on failure
pub(super) fn fold_binary<T>(
	op: BinaryOperator,
	lhs: &ConstantValue<T>,
	rhs: &ConstantValue<T>,
) -> FoldResult<ConstantValue<T>> {
	let (l, r) = (Primitive::new(lhs), Primitive::new(rhs));
	let bool = |b| Ok(ConstantValue::Boolean(b));
	match op {
		BinaryOperator::StrictEquality => return bool(strict_equals(l, r)?),
		BinaryOperator::StrictInequality => {
			return bool(!strict_equals(l, r)?);
		}
		BinaryOperator::Equality => return bool(loose_equals(l, r)?),
		BinaryOperator::Inequality => return bool(!loose_equals(l, r)?),
		// TODO: support basic in?
		BinaryOperator::In | BinaryOperator::Instanceof => {
			return Err(format!(
				"the `{}` operator is not supported for constant values",
				op.as_str()
			)
			.into());
		}
		_ => {}
	}
	let (Some(l), Some(r)) = (l, r) else {
		return Err(format!(
			"the `{}` operator is only supported on primitive values",
			op.as_str()
		)
		.into());
	};
	match op {
		BinaryOperator::LessThan => bool(is_less_than(l, r)? == Some(true)),
		BinaryOperator::GreaterThan => bool(is_less_than(r, l)? == Some(true)),
		BinaryOperator::LessEqualThan => {
			bool(is_less_than(r, l)? == Some(false))
		}
		BinaryOperator::GreaterEqualThan => {
			bool(is_less_than(l, r)? == Some(false))
		}
		BinaryOperator::Addition
			if matches!(l, Primitive::String(_))
				|| matches!(r, Primitive::String(_)) =>
		{
			let mut s = l.to_js_string().into_owned();
			s.push_str(&r.to_js_string());
			Ok(ConstantValue::String(s))
		}
		_ => match (l.to_numeric()?, r.to_numeric()?) {
			(Numeric::Number(a), Numeric::Number(b)) => {
				Ok(ConstantValue::from(number_op(op, a, b)))
			}
			(Numeric::BigInt(a), Numeric::BigInt(b)) => {
				bigint_op(op, a, b).map(ConstantValue::BigInt)
			}
			_ => Err("cannot mix BigInt and other types, use explicit \
			          conversions"
				.into()),
		},
	}
}

/// <https://tc39.es/ecma262/#sec-isstrictlyequal>
///
/// [`None`] operands are objects
fn strict_equals(
	l: Option<Primitive>,
	r: Option<Primitive>,
) -> FoldResult<bool> {
	match (l, r) {
		(Some(l), Some(r)) => Ok(strict_equals_primitive(l, r)),
		(None, None) => Err("cannot compare the identity of objects".into()),
		_ => Ok(false),
	}
}

/// <https://tc39.es/ecma262/#sec-islooselyequal>
///
/// [`None`] operands are objects
fn loose_equals(
	l: Option<Primitive>,
	r: Option<Primitive>,
) -> FoldResult<bool> {
	match (l, r) {
		(Some(l), Some(r)) => Ok(loose_equals_primitive(l, r)),
		(None, None) => Err("cannot compare the identity of objects".into()),
		(Some(Primitive::Undefined | Primitive::Null), None)
		| (None, Some(Primitive::Undefined | Primitive::Null)) => Ok(false),
		_ => Err("loosely comparing an object with a primitive is not \
		          supported"
			.into()),
	}
}

#[expect(clippy::float_cmp)]
fn strict_equals_primitive(l: Primitive, r: Primitive) -> bool {
	match (l, r) {
		(Primitive::Number(a), Primitive::Number(b)) => a == b,
		(Primitive::BigInt(a), Primitive::BigInt(b)) => a == b,
		(Primitive::Boolean(a), Primitive::Boolean(b)) => a == b,
		(Primitive::String(a), Primitive::String(b)) => a == b,
		(Primitive::Undefined, Primitive::Undefined)
		| (Primitive::Null, Primitive::Null) => true,
		_ => false,
	}
}

#[expect(clippy::float_cmp)]
fn loose_equals_primitive(l: Primitive, r: Primitive) -> bool {
	match (l, r) {
		(
			Primitive::Undefined | Primitive::Null,
			Primitive::Undefined | Primitive::Null,
		) => true,
		(Primitive::Undefined | Primitive::Null, _)
		| (_, Primitive::Undefined | Primitive::Null) => false,
		(Primitive::Number(n), Primitive::String(s))
		| (Primitive::String(s), Primitive::Number(n)) => n == s.string_to_number(),
		(Primitive::BigInt(n), Primitive::String(s))
		| (Primitive::String(s), Primitive::BigInt(n)) => s
			.string_to_big_int()
			.is_some_and(|s| *n == s),
		(Primitive::Boolean(b), other) | (other, Primitive::Boolean(b)) => {
			loose_equals_primitive(
				Primitive::Number(f64::from(u8::from(b))),
				other,
			)
		}
		(Primitive::BigInt(b), Primitive::Number(n))
		| (Primitive::Number(n), Primitive::BigInt(b)) => {
			compare_bigint_number(b, n) == Some(Ordering::Equal)
		}
		(l, r) => strict_equals_primitive(l, r),
	}
}

/// <https://tc39.es/ecma262/#sec-islessthan>
///
/// [`None`] is the spec's `undefined`, when either operand is `NaN`
fn is_less_than(l: Primitive, r: Primitive) -> FoldResult<Option<bool>> {
	Ok(match (l, r) {
		// strings are compared by UTF-16 code units, which is not the same
		// ordering as rust's UTF-8 byte comparison
		(Primitive::String(a), Primitive::String(b)) => {
			Some(a.encode_utf16().lt(b.encode_utf16()))
		}
		(Primitive::BigInt(a), Primitive::String(b)) => {
			b.string_to_big_int().map(|b| *a < b)
		}
		(Primitive::String(a), Primitive::BigInt(b)) => {
			a.string_to_big_int().map(|a| a < *b)
		}
		(l, r) => match (l.to_numeric()?, r.to_numeric()?) {
			(Numeric::Number(a), Numeric::Number(b)) => {
				a.partial_cmp(&b).map(Ordering::is_lt)
			}
			(Numeric::BigInt(a), Numeric::BigInt(b)) => Some(a < b),
			(Numeric::BigInt(a), Numeric::Number(b)) => {
				compare_bigint_number(a, b).map(Ordering::is_lt)
			}
			(Numeric::Number(a), Numeric::BigInt(b)) => {
				compare_bigint_number(b, a).map(Ordering::is_gt)
			}
		},
	})
}

/// Exactly compares a bigint with a number, [`None`] if `n` is `NaN`
fn compare_bigint_number(b: &BigInt, n: f64) -> Option<Ordering> {
	if n.is_infinite() {
		return Some(if n > 0. {
			Ordering::Less
		} else {
			Ordering::Greater
		});
	}
	// `None` for NaN
	let trunc = BigInt::from_f64(n.trunc())?;
	Some(b.cmp(&trunc).then_with(|| {
		let fract = n.fract();
		if fract > 0. {
			Ordering::Less
		} else if fract < 0. {
			Ordering::Greater
		} else {
			Ordering::Equal
		}
	}))
}

/// Applies a numeric operator to two numbers
fn number_op(op: BinaryOperator, a: f64, b: f64) -> f64 {
	match op {
		BinaryOperator::Addition => a + b,
		BinaryOperator::Subtraction => a - b,
		BinaryOperator::Multiplication => a * b,
		BinaryOperator::Division => a / b,
		// rust's `%` is `fmod`, which matches JS
		BinaryOperator::Remainder => a % b,
		BinaryOperator::Exponential => exponentiate(a, b),
		// `wrapping_sh*` masks the shift amount to the bit width, as JS does
		BinaryOperator::ShiftLeft => f64::from(
			a.to_int_32()
				.wrapping_shl(b.to_uint_32()),
		),
		BinaryOperator::ShiftRight => f64::from(
			a.to_int_32()
				.wrapping_shr(b.to_uint_32()),
		),
		BinaryOperator::ShiftRightZeroFill => f64::from(
			a.to_uint_32()
				.wrapping_shr(b.to_uint_32()),
		),
		BinaryOperator::BitwiseAnd => f64::from(a.to_int_32() & b.to_int_32()),
		BinaryOperator::BitwiseOR => f64::from(a.to_int_32() | b.to_int_32()),
		BinaryOperator::BitwiseXOR => f64::from(a.to_int_32() ^ b.to_int_32()),
		_ => unreachable!("{op:?} is not a numeric operator"),
	}
}

/// <https://tc39.es/ecma262/#sec-numeric-types-number-exponentiate>
///
/// Differs from IEEE 754 `pow` (which [`f64::powf`] implements) for a `NaN`
/// exponent, and for a base with magnitude 1 and an infinite exponent
fn exponentiate(base: f64, exponent: f64) -> f64 {
	#[expect(clippy::float_cmp)]
	let base_is_one = base.abs() == 1.;
	if exponent.is_nan() || (base_is_one && exponent.is_infinite()) {
		f64::NAN
	} else {
		base.powf(exponent)
	}
}

/// Applies a numeric operator to two bigints
fn bigint_op(op: BinaryOperator, a: &BigInt, b: &BigInt) -> FoldResult<BigInt> {
	Ok(match op {
		BinaryOperator::Addition => a + b,
		BinaryOperator::Subtraction => a - b,
		BinaryOperator::Multiplication => {
			if a.bits() + b.bits() > MAX_BIGINT_BITS {
				return Err("BigInt multiplication result is too large".into());
			}
			a * b
		}
		BinaryOperator::Division | BinaryOperator::Remainder if b.is_zero() => {
			return Err("BigInt division by zero".into());
		}
		// num-bigint truncates towards zero, the same as JS
		BinaryOperator::Division => a / b,
		BinaryOperator::Remainder => a % b,
		BinaryOperator::Exponential => {
			if b.is_negative() {
				return Err("BigInt exponent must be non-negative".into());
			}
			let exp = b
				.to_u32()
				.filter(|&exp| a.bits() * u64::from(exp) <= MAX_BIGINT_BITS)
				.ok_or("BigInt exponentiation result is too large")?;
			a.pow(exp)
		}
		// `a >> b` is `a << -b`
		BinaryOperator::ShiftLeft => bigint_shift(a, b, !b.is_negative())?,
		BinaryOperator::ShiftRight => bigint_shift(a, b, b.is_negative())?,
		BinaryOperator::ShiftRightZeroFill => {
			return Err(
				"BigInts have no unsigned right shift, use >> instead".into()
			);
		}
		BinaryOperator::BitwiseAnd => a & b,
		BinaryOperator::BitwiseOR => a | b,
		BinaryOperator::BitwiseXOR => a ^ b,
		_ => unreachable!("{op:?} is not a numeric operator"),
	})
}

/// Shifts `a` by the magnitude of `b`
fn bigint_shift(a: &BigInt, b: &BigInt, left: bool) -> FoldResult<BigInt> {
	let amount = b.magnitude().to_u64();
	if left {
		let amount = amount
			.filter(|&n| a.is_zero() || a.bits() + n <= MAX_BIGINT_BITS)
			.ok_or("BigInt shift result is too large")?;
		Ok(if a.is_zero() {
			BigInt::ZERO
		} else {
			a << amount
		})
	} else {
		// num-bigint rounds towards negative infinity, the same as JS
		Ok(match amount {
			Some(n) => a >> n,
			None if a.is_negative() => BigInt::from(-1),
			None => BigInt::ZERO,
		})
	}
}
