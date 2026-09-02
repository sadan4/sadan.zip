fn main() {
	let protoc = protoc_bin_vendored::protoc_bin_path()
		.expect("failed to locate vendored protoc binary");
	// SAFETY: build scripts run single-threaded, and nothing else reads or
	// writes process env vars concurrently with this call.
	unsafe { std::env::set_var("PROTOC", protoc) };
	prost_build::Config::new()
		.type_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize, ::typesize::derive::TypeSize)]")
		// FIXME: add tests asserting these impls
		.type_attribute("google.protobuf.Timestamp", "#[derive(PartialOrd, Ord)]")
		.compile_well_known_types()
		.compile_protos(&["proto/explorer_types.proto"], &["proto/"])
		.expect("failed to compile explorer_types.proto");
	println!("cargo:rerun-if-changed=proto/explorer_types.proto");
	println!("cargo:rerun-if-changed=build.rs");
}
