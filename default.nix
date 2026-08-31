{
	pkgs ? import <nixpkgs> {},
	rev ? null,
}: {
	# packages
	reporter = pkgs.callPackage ./nix/reporter.nix {};
	explorer-server = pkgs.callPackage ./nix/explorer-server.nix {inherit rev;};
	pretty-printer = pkgs.callPackage ./nix/pretty-printer.nix {};

	# modules
	module = import ./nix/nixos-module.nix;
}
