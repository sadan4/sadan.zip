{
	pkgs ? import <nixpkgs> {},
	rev ? null,
}: {
	# packages
	reporter = pkgs.callPackage ./nix/reporter.nix {};
	explorer-server = pkgs.callPackage ./nix/explorer-server.nix {inherit rev;};

	# modules
	module = import ./nix/nixos-module.nix;
}
