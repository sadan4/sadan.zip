{
	lib,
	git,
	rustPlatform,
	rev ? null,
}:
rustPlatform.buildRustPackage (finalAttrs: {
		pname = "explorer_server";
		version = "0.1.0";

		env = lib.optionalAttrs (rev != null) {
			GIT_HASH = rev;
		};

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

		nativeBuildInputs = [
			git
		];

		strictDeps = true;

		buildPhase = ''
			cargo build --release --package explorer_server
		'';

		checkPhase = ''
			runHook preCheck
			cargo test --release --package explorer_server --offline
			runHook postCheck
		'';

		installPhase = ''
			mkdir -p $out/bin
			cp target/release/explorer_server $out/bin/
		'';

		meta = {
			description = "";
			homepage = "https://github.com/sadan4/sadan.zip/";
			license = lib.licenses.agpl3Only;
			mainProgram = "explorer_server";
		};
	})
