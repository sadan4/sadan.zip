{
	lib,
	rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
		pname = "reporter";
		version = "0.1.0";

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
		};

		strictDeps = true;

		buildPhase = ''
			cargo build --release --package reporter
		'';

		checkPhase = ''
			runHook preCheck
			cargo test --release --package reporter --offline
			runHook postCheck
		'';

		installPhase = ''
			mkdir -p $out/bin
			cp target/release/reporter $out/bin/
		'';

		meta = {
			description = "";
			homepage = "https://github.com/sadan4/sadan.zip/";
			license = lib.licenses.agpl3Only;
			mainProgram = "reporter";
		};
	})
