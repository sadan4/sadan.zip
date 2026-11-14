{
	description = "sadan.zip";
	inputs = {
		nixpkgs = {
			url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst";
		};
	};

	outputs = {
		self,
		nixpkgs,
		...
	}: let
		inherit (nixpkgs) lib;
		inherit (lib.attrsets) genAttrs;

		pkgsFor = system:
			import nixpkgs {
				inherit system;
				config.allowUnfree = true;
			};

		forLinuxSystems = fn: genAttrs ["x86_64-linux" "aarch64-linux"] (system: fn (pkgsFor system));

		# nixpkgs-unstable has dropped x86_64-darwin
		forAllSystems = fn:
			genAttrs ["x86_64-linux" "aarch64-linux" "aarch64-darwin"] (
				system: fn (pkgsFor system)
			);
	in {
		packages =
			forAllSystems (
				pkgs: let
					available =
						lib.filterAttrs (
							_: v: lib.isDerivation v && lib.meta.availableOn pkgs.stdenv.hostPlatform v
						) (
							import ./default.nix {
								inherit pkgs;
								rev = self.rev or self.dirtyRev or null;
							}
						);
				in
					available
					// lib.optionalAttrs (available ? reporter) {default = available.reporter;}
			);
		nixosModules.default = import ./nix/nixos-module.nix;
		formatter = forAllSystems (pkgs: pkgs.alejandra);
		devShells =
			forLinuxSystems (pkgs: {
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
								llvmPackages_latest.clang-tools
								llvmPackages_latest.clang
								llvmPackages_latest.libclang
							];
							hardeningDisable = ["all"];
							shellHook = ''
                export CC_wasm32_unknown_unknown="${clang-unwrapped}/bin/clang";
                export CFLAGS_wasm32_unknown_unknown="-I ${clang-unwrapped.lib}/lib/clang/21/include";
                export LIBCLANG_PATH="${pkgs.llvmPackages_latest.libclang.lib}/lib";
                export CC="${clang}/bin/clang";
                export CXX="${clang}/bin/clang++";
                # runtime libs for cargo-built test binaries (no nix rpath):
                # libstdc++ (clang -lstdc++)
                export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
										pkgs.stdenv.cc.cc.lib
									]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}";
              '';
						};
				});
	};
}
