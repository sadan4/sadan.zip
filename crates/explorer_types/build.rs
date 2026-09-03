fn main() {
	let protoc = protoc_bin_vendored::protoc_bin_path()
		.expect("failed to locate vendored protoc binary");
	// SAFETY: build scripts run single-threaded, and nothing else reads or
	// writes process env vars concurrently with this call.
	unsafe { std::env::set_var("PROTOC", protoc) };
	tonic_prost_build::configure()
		.type_attribute(".", "#[derive(::serde::Serialize, ::serde::Deserialize, ::typesize::derive::TypeSize)]")
		// the JSON files inside the downloadable 7z archives are consumed
		// by JS, which expects camelCase keys
		.type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
		// and a hex string, not an array of bytes
		.field_attribute(
			".explorer_types.BundleMetadata.build_hash",
			"#[serde(with = \"crate::proto::build_hash_hex\")]",
		)
		// FIXME: add tests asserting these impls
		.type_attribute("google.protobuf.Timestamp", "#[derive(PartialOrd, Ord)]")
		.compile_well_known_types(true)
		.build_client(true)
		.build_server(true)
		.build_transport(true)
		.bytes(".")
		.compile_protos(&["proto/explorer_types.proto"], &["proto/"])
		.expect("failed to compile explorer_types.proto");
	println!("cargo:rerun-if-changed=proto/explorer_types.proto");
	println!("cargo:rerun-if-changed=build.rs");
}
