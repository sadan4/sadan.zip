{pkgs ? import <nixpkgs> {}}: {
	# packages
	reporter = pkgs.callPackage ./nix/reporter.nix {};
	explorer-server = pkgs.callPackage ./nix/explorer-server.nix {};

	# modules
	module = import ./nix/nixos-module.nix;
}
