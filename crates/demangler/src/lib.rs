#[cxx::bridge]
#[allow(clippy::semicolon_outside_block)]
pub mod ffi {

	unsafe extern "C++" {
		include!("demangler/src/lib.hpp");
		/// Will return a nullptr if demangling fails
		fn demangle_all(input: &str) -> UniquePtr<CxxString>;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	fn demangle(input: &str) -> String {
		let result = ffi::demangle_all(input);
		if result.is_null() {
			String::from(input)
		} else {
			String::from(result.to_str().unwrap())
		}
	}
	#[test]
	fn demangle_test() {
		// https://github.com/llvm/llvm-project/blob/main/llvm/unittests/Demangle/DemangleTest.cpp
		assert_eq!(demangle("_"), "_");
		assert_eq!(demangle("_Z3fooi"), "foo(int)");
		assert_eq!(demangle("__Z3fooi"), "foo(int)");
		assert_eq!(
			demangle("___Z3fooi_block_invoke"),
			"invocation function for block in foo(int)"
		);
		assert_eq!(
			demangle("____Z3fooi_block_invoke"),
			"invocation function for block in foo(int)"
		);
		assert_eq!(demangle("?foo@@YAXH@Z"), "void __cdecl foo(int)");
		assert_eq!(demangle("foo"), "foo");
		assert_eq!(demangle("_RNvC3foo3bar"), "foo::bar");
		assert_eq!(demangle("__RNvC3foo3bar"), "foo::bar");
		assert_eq!(demangle("_Dmain"), "D main");

		// Regression test for demangling of optional template-args for vendor
		// extended type qualifier (https://bugs.llvm.org/show_bug.cgi?id=48009)
		assert_eq!(
			demangle("_Z3fooILi79EEbU7_ExtIntIXT_EEi"),
			"bool foo<79>(int _ExtInt<79>)"
		);
	}
}
