#[doc(hidden)]
#[macro_export]
macro_rules! __assert_implies {
	($cond:expr => $implies_that:expr) => {
		$crate::macros::assert_implies!($cond => $implies_that, "")
	};
	($cond:expr => $implies_that:expr, $($arg:tt)+) => {
		if $cond {
			let implies_that = $implies_that;
			if !implies_that {
				assert!(false, "{} implies that {} should be true. {}", stringify!($cond), stringify!($implies_that), format!($($arg)+));
			}
		}
	};
	(let $cond:pat = $val:expr => $implies_that:expr) => {
		$crate::macros::assert_implies!(let $cond = $val => $implies_that, "");
	};
	(let $cond:pat = $val:expr => $implies_that:expr, $($arg:tt)+) => {
		if let $cond = $val {
			let implies_that = $implies_that;
			if !implies_that {
				assert!(false, "let {} = {} implies that {} should be true. {}", stringify!($cond), stringify!($val), stringify!($implies_that), format!($($arg)+));
			}
		}
	};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __debug_assert_implies {
	($cond:expr => $implies_that:expr) => {
		$crate::macros::debug_assert_implies!($cond => $implies_that, "")
	};
	($cond:expr => $implies_that:expr, $($arg:tt)+) => {
		if cfg!(debug_assertions) {
			$crate::macros::assert_implies!($cond => $implies_that, $($arg)+);
		}
	};
	(let $cond:pat = $val:expr => $implies_that:expr) => {
		$crate::macros::debug_assert_implies!(let $cond = $val => $implies_that, "");
	};
	(let $cond:pat = $val:expr => $implies_that:expr, $($arg:tt)+) => {
		if cfg!(debug_assertions) {
			$crate::macros::assert_implies!(let $cond = $val => $implies_that, $($arg)+);
		}
	};
}

/// Asserts that one condition implies another.
///
/// If `$cond` evaluates to `true`, then `$implies_that` must evaluate to
/// `true` as well. If `$cond` is `false`, nothing is implied; therefore,
/// `$implies_that` is never evaluated, so it is free to rely on pattern
/// matching in `$cond`.
///
/// See [`debug_assert_implies!`] for a version that is only checked when
/// `cfg!(debug_assertions)` are enabled.
///
/// # Panics
///
/// Panics if the `$cond` is true/matches, but `$implies_that` is false.
///
/// # Examples
///
/// ```
/// use stdx::macros::assert_implies;
///
/// let items = vec![1, 2, 3];
/// let len = items.len();
///
/// assert_implies!(!items.is_empty() => len > 0);
///
/// `$cond` `(items.is_empty())` is false, so `items[99] == 0` is never evaluated.
/// assert_implies!(items.is_empty() => items[99] == 0);
///
/// // pattern matching
/// let first = items.first().copied();
/// assert_implies!(let Some(n) = first => n > 0, "{n} should be positive");
/// ```
///
/// A violated implication panics:
///
/// ```should_panic
/// use stdx::macros::assert_implies;
///
/// let n = 4;
/// assert_implies!(n % 2 == 0 => n > 10, "{n} is even but too small");
/// ```
#[doc(inline)]
pub use __assert_implies as assert_implies;
/// Asserts that one condition implies another, but only when
/// `debug_assertions` are enabled.
///
/// This is the debug-only counterpart to [`assert_implies!`] and accepts
/// exactly the same forms.
///
/// In builds with `cfg!(debug_assertions)` disabled, the code is type-checked,
/// but compiled out at runtime.
///
/// # Panics
///
/// See [`assert_implies!`]
///
/// # Examples
///
/// ```
/// use stdx::macros::debug_assert_implies;
///
/// // An expensive value
/// let cached: Option<u32> = Some(7);
/// // if `true`, `cached` must be `Some(1..)`
/// let dirty = false;
/// 
/// // ...
///
/// // A clean cache must hold a value, and that value is never zero.
/// debug_assert_implies!(!dirty => cached.is_some());
/// debug_assert_implies!(let Some(v) = cached => v != 0, "zero cached");
/// ```
#[doc(inline)]
pub use __debug_assert_implies as debug_assert_implies;
