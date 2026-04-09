{
	description = "A basic flake with a shell";
	inputs = {
		nixpkgs = {
			url = "github:NixOS/nixpkgs/nixos-unstable";
		};
		systems = {
			url = "github:nix-systems/default";
		};
		flake-utils = {
			url = "github:numtide/flake-utils";
			inputs.systems.follows = "systems";
		};
	};

	outputs = {
		nixpkgs,
		flake-utils,
		...
	}:
		flake-utils.lib.eachDefaultSystem (
			system: let
				pkgs =
					import nixpkgs {
						inherit system;
					};
				clang-unwrapped = pkgs.llvmPackages_21.clang-unwrapped;
			in {
				devShells.default =
					pkgs.mkShell {
						packages = with pkgs; [
							emscripten
							wasm-bindgen-cli
							msgpack-tools
                            mold
                            clang_21
                            (writeShellScriptBin "build-reporter-cli-static" ''
                                nix-shell -p musl --command "export CC=musl-gcc; cargo build -p reporter --release --target x86_64-unknown-linux-musl"
                            '')
						];
						hardeningDisable = ["all"];
						shellHook = ''
							export CC_wasm32_unknown_unknown="${clang-unwrapped}/bin/clang";
							export CFLAGS_wasm32_unknown_unknown="-I ${clang-unwrapped.lib}/lib/clang/21/include";
						'';
					};
			}
		);
}
