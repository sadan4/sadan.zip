{
	description = "A basic flake with a shell";
	inputs = {
		nixpkgs = {
			url = "github:NixOS/nixpkgs/nixos-25.11";
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
                clang = pkgs.llvmPackages_21.clang-unwrapped;
			in {
				devShells.default =
					pkgs.mkShell {
						packages = with pkgs; [
                            emscripten
                            wasm-bindgen-cli
                            clang
                            msgpack-tools
						];
                        hardeningDisable = ["all"];
						shellHook = ''
                            export CC_wasm32_unknown_unknown="clang";
                            export CFLAGS_wasm32_unknown_unknown="-I ${clang.lib}/lib/clang/21/include";
						'';
					};
			}
		);
}
