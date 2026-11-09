{
	lib,
	rustPlatform,
	pkg-config,
	aws-lc,
	oniguruma,
	zstd,
	rev ? "git",
}: let
	crate-name = "reporter";
in
	rustPlatform.buildRustPackage (finalAttrs: {
			pname = crate-name;
			version = "0.1.0-${rev}";

			__structuredAttrs = true;
			strictDeps = true;

			src =
				lib.fileset.toSource {
					root = ../.;
					fileset =
						lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ../.)) (
							lib.fileset.unions [
								../Cargo.toml
								../Cargo.lock
								../crates
							]
						);
				};

			cargoLock = {
				lockFile = ../Cargo.lock;
				outputHashes = import ./cargo-output-hashes.nix;
			};

			nativeBuildInputs = [
				pkg-config
			];

			buildInputs = [
				aws-lc
				oniguruma
				zstd
			];

			env = {
				GIT_HASH = rev;
				RUSTC_BOOTSTRAP = 1;
			};

			buildPhase = ''
        cargo build --release --package ${crate-name}
      '';

			useNextest = true;

			cargoTestFlags = [
				"-E"
				"deps(${crate-name})"
			];

			installPhase = ''
        mkdir -p $out/bin
        cp target/release/${crate-name} $out/bin/
      '';

			meta = {
				description = "";
				homepage = "https://github.com/sadan4/sadan.zip/";
				license = lib.licenses.agpl3Only;
				mainProgram = crate-name;
			};
		})
