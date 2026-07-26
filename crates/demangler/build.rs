fn watch(path: &str) {
	println!("cargo:rerun-if-changed={path}");
}
fn link(lib: &str) {
	println!("cargo:rustc-link-lib={lib}");
}
fn main() {
	cxx_build::bridge("src/lib.rs")
		.file("src/lib.cpp")
		.std("c++23")
		.compile("demangler-rs");
	link("LLVMDemangle");
	watch("src/lib.cpp");
	watch("src/lib.hpp");
}
