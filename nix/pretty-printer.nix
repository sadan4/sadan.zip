{
	lib,
	stdenv,
	installShellFiles,
	rustPlatform,
}:
rustPlatform.buildRustPackage (finalAttrs: {
		pname = "pretty_printer";
		version = "0.1.0";

		env.RUSTC_BOOTSTRAP = 1;

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
			installShellFiles
		];

		strictDeps = true;

		buildPhase = ''
			cargo build --release --package pretty_printer
		'';

		checkPhase = ''
			runHook preCheck
			cargo test --release --package pretty_printer --package ast_parser --offline
			runHook postCheck
		'';

		installPhase = ''
			runHook preInstall
			mkdir -p $out/bin
			cp target/release/pretty_printer $out/bin/
			runHook postInstall
		'';

		postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
			installShellCompletion --cmd pretty_printer \
				--bash <($out/bin/pretty_printer --completions bash) \
				--fish <($out/bin/pretty_printer --completions fish) \
				--zsh <($out/bin/pretty_printer --completions zsh)
		'';

		meta = {
			description = "A port of the pretty printer found in chrome's devtools, with byte-for-byte output (excluding bugs).";
			homepage = "https://github.com/sadan4/sadan.zip/tree/web/crates/pretty_printer";
			license = lib.licenses.bsd3;
			mainProgram = "pretty_printer";
		};
	})
