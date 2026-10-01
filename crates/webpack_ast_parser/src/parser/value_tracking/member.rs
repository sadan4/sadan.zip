//! Reading properties of constant values

use std::{borrow::Cow, fmt};

use ordered_float::NotNan;
use oxc::syntax::number::ToJsString;
use oxc_ecmascript::StringToNumber;

use super::{ConstantPropertyKey, ConstantValue, primitive::FoldResult};

impl fmt::Display for ConstantPropertyKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::String(s) => f.write_str(s),
			Self::Number(n) => f.write_str(&n.to_js_string()),
		}
	}
}

impl ConstantPropertyKey {
	/// The key for the property named `s`
	///
	/// Keys that are numbers in their canonical string form (`"1"`, not
	/// `"01"`) are [`Self::Number`], so `{ 1: a }` and `{ "1": a }` are the
	/// same key, as they are in JS
	pub(super) fn from_string(s: &str) -> Self {
		let n = s.string_to_number();
		if !n.is_nan() && n.to_js_string() == s {
			Self::from_number(n)
		} else {
			Self::String(s.to_owned())
		}
	}
	/// The key for the property named by `n`, as in `{ 1: a }` or `a[1]`
	pub(super) fn from_number(n: f64) -> Self {
		// `+ 0.` turns `-0` into `0`, both are the key `"0"`
		NotNan::new(n + 0.)
			.map_or_else(|_| Self::String("NaN".to_owned()), Self::Number)
	}
}

/// Properties every object inherits from `Object.prototype`, which are not
/// constant values
const OBJECT_PROTOTYPE_PROPS: &[&str] = &[
	"__proto__",
	"constructor",
	"hasOwnProperty",
	"isPrototypeOf",
	"propertyIsEnumerable",
	"toLocaleString",
	"toString",
	"valueOf",
];

/// The result of reading a property
pub(super) enum Property<'v, T> {
	/// A value that already exists in the object, such as an object property
	Existing(&'v T),
	/// A value that was computed from the object, such as an array's length
	New(ConstantValue<T>),
}

/// Reads `value[key]`, returning a human readable reason on failure
pub(super) fn get_property<'v, T>(
	value: &'v ConstantValue<T>,
	key: &ConstantPropertyKey,
) -> FoldResult<Property<'v, T>> {
	let len = |len: usize| {
		#[expect(clippy::cast_precision_loss, reason = "lengths are small")]
		Ok(Property::New(ConstantValue::from(len as f64)))
	};
	let is =
		|name: &str| matches!(key, ConstantPropertyKey::String(s) if s == name);
	let undefined = || Ok(Property::New(ConstantValue::Undefined));
	match value {
		ConstantValue::Object(props) => {
			if let Some(v) = props.get(key) {
				Ok(Property::Existing(v))
			} else if OBJECT_PROTOTYPE_PROPS
				.iter()
				.any(|&p| is(p))
			{
				Err(format!(
					"`{key}` is inherited from `Object.prototype`, which is \
					 not supported"
				)
				.into())
			} else {
				undefined()
			}
		}
		ConstantValue::Array(elts) => match key {
			// holes and indices past the end are `undefined`
			ConstantPropertyKey::Number(n) => index(**n)
				.and_then(|i| elts.get(i))
				.map_or_else(undefined, |v| Ok(Property::Existing(v))),
			_ if is("length") => len(elts.len()),
			ConstantPropertyKey::String(_) => Err(unsupported_method(key)),
		},
		// JS strings are UTF-16
		ConstantValue::String(s) => match key {
			ConstantPropertyKey::Number(n) => {
				let Some(unit) =
					index(**n).and_then(|i| s.encode_utf16().nth(i))
				else {
					return undefined();
				};
				let c = char::from_u32(unit.into()).ok_or(
					"indexing half of a surrogate pair is not supported",
				)?;
				Ok(Property::New(ConstantValue::String(c.into())))
			}
			_ if is("length") => len(s.encode_utf16().count()),
			ConstantPropertyKey::String(_) => Err(unsupported_method(key)),
		},
		ConstantValue::Set(s) if is("size") => len(s.len()),
		ConstantValue::Set(_) => Err(unsupported_method(key)),
		ConstantValue::Undefined => {
			Err(format!("cannot read property `{key}` of undefined").into())
		}
		ConstantValue::Null => {
			Err(format!("cannot read property `{key}` of null").into())
		}
		ConstantValue::Number(_)
		| ConstantValue::NaN
		| ConstantValue::BigInt(_)
		| ConstantValue::Boolean(_) => Err(format!(
			"reading `{key}` of a number, bigint or boolean is not supported"
		)
		.into()),
	}
}

fn unsupported_method(key: &ConstantPropertyKey) -> Cow<'static, str> {
	format!("`{key}` is not supported, methods are not constant values").into()
}

/// The array index `n` is, if it is one
fn index(n: f64) -> Option<usize> {
	#[expect(
		clippy::cast_possible_truncation,
		clippy::cast_sign_loss,
		clippy::cast_precision_loss,
		reason = "checked to be a non-negative integer in range"
	)]
	(n >= 0. && n.fract() == 0. && n < usize::MAX as f64).then_some(n as usize)
}
