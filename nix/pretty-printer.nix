{
	lib,
	stdenv,
	installShellFiles,
	rustPlatform,
	rev ? null,
}: let
	crate-name = "pretty_printer";
in
	rustPlatform.buildRustPackage (finalAttrs: {
			pname = crate-name;
			version = "0.1.0";

			env =
				lib.optionalAttrs (rev != null) {
					GIT_HASH = rev;
				}
				// {
					RUSTC_BOOTSTRAP = 1;
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
				outputHashes = import ./cargo-output-hashes.nix;
			};

			nativeBuildInputs = [
				installShellFiles
			];

			strictDeps = true;

			buildPhase = ''
				cargo build --release --package ${crate-name}
			'';

			useNextest = true;

			cargoTestFlags = [
				"-E"
				"deps(${crate-name})"
			];

			installPhase = ''
				runHook preInstall
				mkdir -p $out/bin
				cp target/release/${crate-name} $out/bin/
				runHook postInstall
			'';

			postInstall =
				lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
					installShellCompletion --cmd ${crate-name} \
						--bash <($out/bin/${crate-name} --completions bash) \
						--fish <($out/bin/${crate-name} --completions fish) \
						--zsh <($out/bin/${crate-name} --completions zsh)
				'';

			meta = {
				description = "A port of the pretty printer found in chrome's devtools, with byte-for-byte output (excluding bugs).";
				homepage = "https://github.com/sadan4/sadan.zip/tree/web/crates/${crate-name}";
				license = lib.licenses.bsd3;
				mainProgram = crate-name;
			};
		})
