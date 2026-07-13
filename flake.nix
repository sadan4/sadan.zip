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
					in
						pkgs.mkShell {
							packages = with pkgs; [
								emscripten
								wasm-bindgen-cli
								msgpack-tools
								mold
								clang_21
							];
							hardeningDisable = ["all"];
							shellHook = ''
								export CC_wasm32_unknown_unknown="${clang-unwrapped}/bin/clang";
								export CFLAGS_wasm32_unknown_unknown="-I ${clang-unwrapped.lib}/lib/clang/21/include";
							'';
						};
				});
	};
}
