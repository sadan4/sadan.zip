{
	description = "sadan.zip";
	inputs = {
		nixpkgs = {
			url = "github:NixOS/nixpkgs/nixos-unstable";
		};
	};

	outputs = {
		self,
		nixpkgs,
		...
	}: let
		inherit (nixpkgs) lib;
		inherit (lib.attrsets) genAttrs;

		forAllSystems = fn:
			genAttrs ["x86_64-linux" "aarch64-linux"] (
				system:
					fn (
						import nixpkgs {
							inherit system;
							config.allowUnfree = true;
						}
					)
			);
	in {
		packages =
			forAllSystems (
				pkgs:
					lib.filterAttrs (_: lib.isDerivation) (
						import ./default.nix {
							inherit pkgs;
							rev = self.rev or self.dirtyRev or null;
						}
					)
					// {default = self.packages.${pkgs.stdenv.hostPlatform.system}.reporter;}
			);
		nixosModules.default = import ./nix/nixos-module.nix;
		devShells =
			forAllSystems (pkgs: {
					default = let
						inherit (pkgs.llvmPackages_21) clang-unwrapped;
						inherit (pkgs.llvmPackages_latest) clang;
					in
						pkgs.mkShell {
							packages = with pkgs; [
								emscripten
								wasm-bindgen-cli
								msgpack-tools
								# explorer_server's cache_matrix tests run against both
								redis
								valkey
								mold
								pkg-config
								openssl
								libgit2
								libqalculate.dev
								llvmPackages_latest.clang-tools
								llvmPackages_latest.clang
								llvmPackages_latest.libclang
								llvmPackages_latest.libllvm
								libiberty
                                protobuf
                                protoc-gen-js
							];
							buildInputs = with pkgs; [
								fontconfig
                                libpng
								freetype
							];
							hardeningDisable = ["all"];
							shellHook = ''
								export CC_wasm32_unknown_unknown="${clang-unwrapped}/bin/clang";
								export CFLAGS_wasm32_unknown_unknown="-I ${clang-unwrapped.lib}/lib/clang/21/include";
								export LIBCLANG_PATH="${pkgs.llvmPackages_latest.libclang.lib}/lib";
								# skia-bindings source build needs clang (gcc rejects skia's --target= flags)
								export CC="${clang}/bin/clang";
								export CXX="${clang}/bin/clang++";
								# runtime libs for cargo-built test binaries (no nix rpath):
								# libstdc++ (clang -lstdc++), libqalculate, skia's C deps
								export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
									pkgs.stdenv.cc.cc.lib
									pkgs.libqalculate
									pkgs.fontconfig
									pkgs.libpng
									pkgs.freetype
								]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}";
							'';
						};
				});
	};
}
