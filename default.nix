{
	pkgs ? import <nixpkgs> {},
	rev ? null,
}: {
	# packages
	reporter = pkgs.callPackage ./nix/reporter.nix {inherit rev;};
	explorer-server = pkgs.callPackage ./nix/explorer-server.nix {inherit rev;};
	pretty-printer = pkgs.callPackage ./nix/pretty-printer.nix {inherit rev;};

	# modules
	module = import ./nix/nixos-module.nix;
}
