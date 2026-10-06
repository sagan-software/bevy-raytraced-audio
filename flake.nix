{
  description = "Bevy 2D and 3D acoustic propagation with Nix-first workflows";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    bevy_cli = {
      url = "github:TheBevyFlock/bevy_cli";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      treefmt-nix,
      crane,
      rust-overlay,
      bevy_cli,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        bevyLintToolchain = pkgs.rust-bin.fromRustupToolchainFile "${bevy_cli}/rust-toolchain.toml";
        bevyCli = bevy_cli.packages.${system}.default.overrideAttrs (oldAttrs: {
          buildInputs =
            (oldAttrs.buildInputs or [ ]) ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.zlib ];
          postInstall = (oldAttrs.postInstall or "") + ''
            wrapProgram "$out/bin/bevy_lint" \
              --prefix PATH : "${pkgs.lib.makeBinPath [ bevyLintToolchain ]}"
          '';
        });
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = craneLib.cleanCargoSource ./.;
        packageName = "bevy-raytraced-audio";
        coverageThreshold = 91;
        coverageIgnoreRegex = "(^|/)(tests|benches|examples)/";
        supportedFeatures = [
          "bevy_0_17"
          "bevy_0_18"
          "bevy_0_19"
          "bevy_0_20"
        ];
        supportedFeatureCheckCommands = pkgs.lib.concatMapStringsSep "\n" (
          feature:
          "cargo check --locked --package bevy-raytraced-audio-2d --package bevy-raytraced-audio-3d --no-default-features --features '${feature}'"
        ) supportedFeatures;

        bevyNativeBuildInputs = [
          pkgs.cmake
          pkgs.clang
          pkgs.lld
          pkgs.makeWrapper
          pkgs.pkg-config
        ];

        bevyBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
          pkgs.alsa-lib
          pkgs.libxkbcommon
          pkgs.udev
          pkgs.vulkan-loader
          pkgs.wayland
          pkgs.libx11
          pkgs.libxcursor
          pkgs.libxi
          pkgs.libxrandr
        ];

        runtimeLibraryPath = pkgs.lib.makeLibraryPath bevyBuildInputs;
        pkgConfigPath = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" bevyBuildInputs;

        bevyBrpMcp =
          let
            pname = "bevy_brp_mcp";
            version = "0.20.1";
          in
          pkgs.rustPlatform.buildRustPackage {
            inherit pname version;

            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-pFE8vKDwuc9e8viKlidPRnvdC5JlF90/vgApzvJXLyQ=";
            };

            cargoHash = "sha256-rDjhWN1Sc+D0Oi5rZuL80Ifa1BU8dvJlAFYjfINYPDo=";
            doCheck = false;

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.openssl ];

            meta.mainProgram = "bevy_brp_mcp";
          };

        aiSupport = pkgs.callPackage ./ai/default.nix {
          inherit bevyBrpMcp packageName;
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = bevyNativeBuildInputs;
          buildInputs = bevyBuildInputs;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        devCargoArtifacts = craneLib.buildDepsOnly (commonArgs // { CARGO_PROFILE = ""; });
        clippyCargoArtifacts = craneLib.buildDepsOnly (
          commonArgs
          // {
            CARGO_PROFILE = "";
            cargoExtraArgs = "--locked --all-targets --all-features";
            cargoBuildCommand = "true";
            doCheck = false;
          }
        );
        cacheTools = [
          pkgs.python3
          pkgs.sccache
          pkgs.ccache
        ];
        fastEnvironment = ''
          if [ -f scripts/cargo-fast.py ] && [ "''${BEVY_BUILD_CACHE_DISABLE:-0}" != 1 ]; then
            eval "$(python scripts/cargo-fast.py --shell-env)"
          fi
        '';

        coreLibrary = craneLib.mkCargoDerivation (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = packageName;
            version = "0.1.0";
            buildPhaseCargoCommand = "cargo build --package ${packageName} --release --locked";
            doInstallCargoArtifacts = false;
            installPhaseCommand = ''
              mkdir -p "$out/lib"
              install -m 0444 target/release/libbevy_raytraced_audio.rlib "$out/lib/"
              test -s "$out/lib/libbevy_raytraced_audio.rlib"
            '';
          }
        );

        ensureAgentLink = aiSupport.ensureLinks;

        withAgentLink =
          text:
          ''
            set -euo pipefail
            ensure-ai-links
          ''
          + text;

        # dprint accepts local WASM paths: https://dprint.dev/config/#plugins.
        # Fetch pinned plugins before the network-disabled formatting check.
        dprintSettings = builtins.fromJSON (builtins.readFile ./dprint.json);
        dprintPluginHashes = builtins.fromJSON (builtins.readFile ./nix/dprint-plugin-hashes.json);
        dprintOfflineConfig = pkgs.writeText "dprint-offline.json" (
          builtins.toJSON (
            dprintSettings
            // {
              plugins = map (
                url:
                toString (
                  pkgs.fetchurl {
                    inherit url;
                    sha256 = dprintPluginHashes.${url};
                  }
                )
              ) dprintSettings.plugins;
            }
          )
        );

        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            dprint.enable = true;
            nixfmt.enable = true;
            rustfmt.enable = true;
          };
          settings.global.excludes = [
            ".direnv/**"
            ".git/**"
            "target/**"
            "result*/**"
          ];
          settings.formatter.dprint.options = [
            "--allow-no-files"
            "--config"
            (toString dprintOfflineConfig)
          ];
        };

        treefmtCheckEval = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            dprint.enable = true;
            nixfmt.enable = true;
            rustfmt.enable = true;
          };
          settings.global.excludes = [
            ".direnv/**"
            ".git/**"
            "target/**"
            "result*/**"
          ];
          settings.formatter.dprint.options = [
            "--allow-no-files"
            "--config"
            (toString dprintOfflineConfig)
          ];
        };

        formatRepo = pkgs.writeShellApplication {
          name = "format-repo";
          runtimeInputs = [
            ensureAgentLink
            treefmtEval.config.build.wrapper
          ];
          text = withAgentLink ''
            treefmt
          '';
        };

        formatCheck =
          pkgs.runCommand "format-check"
            {
              nativeBuildInputs = [ treefmtCheckEval.config.build.wrapper ];
            }
            ''
              cp -r ${./.} ./repo
              chmod -R +w ./repo
              cd ./repo

              export HOME="$TMPDIR"
              export XDG_CACHE_HOME="$TMPDIR/.cache"
              treefmt --fail-on-change

              touch "$out"
            '';

        clippyCheck = craneLib.cargoClippy (
          commonArgs
          // {
            cargoArtifacts = clippyCargoArtifacts;
            cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
          }
        );

        testCheck = craneLib.cargoNextest (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestExtraArgs = "--workspace --no-tests=pass";
          }
        );

        doctestCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            pname = "${packageName}-doctest";
            version = "0.1.0";
            buildPhaseCargoCommand = "cargo test --workspace --doc --locked";
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        privateDocsCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            pname = "${packageName}-private-docs";
            version = "0.1.0";
            RUSTDOCFLAGS = "-D warnings";
            buildPhaseCargoCommand = "cargo doc --workspace --no-deps --document-private-items --locked";
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        docsBookCheck =
          pkgs.runCommand "${packageName}-docs-book"
            {
              nativeBuildInputs = [ pkgs.mdbook ];
            }
            ''
              cp -r ${./docs} ./docs
              chmod -R +w ./docs
              cd ./docs
              mdbook build --dest-dir "$out"
              test -s "$out/index.html"
            '';

        buildCacheCheck =
          pkgs.runCommand "${packageName}-build-cache-tests"
            {
              nativeBuildInputs = [
                pkgs.python3
                pkgs.git
              ];
            }
            ''
              cp -r ${./scripts} ./scripts
              chmod -R u+w scripts
              patchShebangs scripts
              python -m unittest discover -s scripts -p 'test_*.py'
              touch "$out"
            '';

        benchmarkCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            inherit cargoArtifacts;
            pname = "${packageName}-bench-check";
            version = "0.1.0";
            buildPhaseCargoCommand = "cargo bench --workspace --locked --no-run";
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        featureMatrixCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            pname = "${packageName}-feature-matrix-check";
            version = "0.1.0";
            buildPhaseCargoCommand = supportedFeatureCheckCommands;
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        wasmExamplesCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            pname = "${packageName}-wasm-examples";
            version = "0.1.0";
            buildPhaseCargoCommand = "cargo build --locked --profile wasm-release --target wasm32-unknown-unknown --package bevy-raytraced-audio-examples --examples --features webgl2";
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        coverageReport = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = devCargoArtifacts;
            pname = "${packageName}-coverage";
            version = "0.1.0";
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.cargo-llvm-cov ];
            buildPhaseCargoCommand = ''
              mkdir -p "$out"
              cargo llvm-cov clean --workspace
              cargo llvm-cov --workspace --locked --remap-path-prefix --no-report
              cargo llvm-cov report --html --output-dir "$out" \
                --ignore-filename-regex '${coverageIgnoreRegex}'
              cargo llvm-cov report --lcov --output-path "$out/lcov.info" \
                --ignore-filename-regex '${coverageIgnoreRegex}'
              cargo llvm-cov report --json --output-path "$out/coverage.json" \
                --ignore-filename-regex '${coverageIgnoreRegex}' \
                --skip-functions
              cargo llvm-cov report \
                --fail-under-lines ${toString coverageThreshold} \
                --ignore-filename-regex '${coverageIgnoreRegex}' \
                --show-missing-lines
              test -s "$out/html/index.html"
              test -s "$out/lcov.info"
              test -s "$out/coverage.json"
            '';
            doInstallCargoArtifacts = false;
            installPhaseCommand = "true";
          }
        );

        bevyLintCheck = craneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = null;
            pname = "${packageName}-bevy-lint";
            version = "0.1.0";
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ bevyCli ];
            buildPhaseCargoCommand = ''
              CARGO_TARGET_DIR=target/bevy-lint bevy_lint --workspace --all-targets --locked
            '';
            doInstallCargoArtifacts = false;
            installPhaseCommand = "mkdir -p $out";
          }
        );

        runCoverage = pkgs.writeShellApplication {
          name = "run-coverage";
          runtimeInputs = [
            ensureAgentLink
            pkgs.cargo-llvm-cov
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"

              report_dir="target/llvm-cov"
              mkdir -p "$report_dir"

              cargo llvm-cov clean --workspace
              cargo llvm-cov --workspace --locked --remap-path-prefix --no-report
              cargo llvm-cov report --html --output-dir "$report_dir" \
                --ignore-filename-regex '${coverageIgnoreRegex}'
              cargo llvm-cov report --lcov --output-path "$report_dir/lcov.info" \
                --ignore-filename-regex '${coverageIgnoreRegex}'
              cargo llvm-cov report --json --output-path "$report_dir/coverage.json" \
                --ignore-filename-regex '${coverageIgnoreRegex}' \
                --skip-functions
              cargo llvm-cov report \
                --fail-under-lines ${toString coverageThreshold} \
                --ignore-filename-regex '${coverageIgnoreRegex}' \
                --show-missing-lines

              test -s "$report_dir/html/index.html"
              test -s "$report_dir/lcov.info"
              test -s "$report_dir/coverage.json"
            ''
          );
        };

        runChecks = pkgs.writeShellApplication {
          name = "run-checks";
          runtimeInputs = [
            ensureAgentLink
            treefmtEval.config.build.wrapper
            pkgs.cargo-nextest
            rustToolchain
            bevyCli
            runCoverage
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              treefmt --fail-on-change
              cargo clippy --workspace --all-targets --locked -- --deny warnings
              ${supportedFeatureCheckCommands}
              CARGO_TARGET_DIR=target/bevy-lint bevy_lint --workspace --all-targets --locked
              cargo nextest run --locked --workspace --no-tests=pass
              cargo test --workspace --doc --locked
              RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --locked
              cargo bench --workspace --locked --no-run
              run-coverage
            ''
          );
        };

        runDylint = pkgs.writeShellApplication {
          name = "run-dylint";
          runtimeInputs = [
            pkgs.git
            pkgs.nix
            rustToolchain
          ]
          ++ cacheTools;
          text = fastEnvironment + ''
            exec ${pkgs.bash}/bin/bash ${./scripts/run-dylint.sh}
          '';
        };

        runClippy = pkgs.writeShellApplication {
          name = "run-clippy";
          runtimeInputs = [
            ensureAgentLink
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              cargo clippy --workspace --all-targets --locked -- --deny warnings
            ''
          );
        };

        runFeatureChecks = pkgs.writeShellApplication {
          name = "check-supported-features";
          runtimeInputs = [
            ensureAgentLink
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              ${supportedFeatureCheckCommands}
            ''
          );
        };

        runTests = pkgs.writeShellApplication {
          name = "run-tests";
          runtimeInputs = [
            ensureAgentLink
            pkgs.cargo-nextest
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              cargo nextest run --locked --workspace --no-tests=pass
              cargo test --workspace --doc --locked
            ''
          );
        };

        runBenchmarks = pkgs.writeShellApplication {
          name = "run-benchmarks";
          runtimeInputs = [
            ensureAgentLink
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              cargo bench --package bevy-raytraced-audio --bench propagation --locked -- "$@"
              cargo bench --package bevy-raytraced-audio-public-tests --bench adapter_schedule --locked -- "$@"
            ''
          );
        };

        runExample =
          exampleName:
          pkgs.writeShellApplication {
            name = "run-${exampleName}";
            runtimeInputs = [
              ensureAgentLink
              rustToolchain
            ]
            ++ cacheTools
            ++ bevyNativeBuildInputs
            ++ bevyBuildInputs;
            text = withAgentLink (
              fastEnvironment
              + ''
                export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
                export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
                cargo run --locked --package bevy-raytraced-audio-examples --example '${exampleName}' -- "$@"
              ''
            );
          };

        runWebBuild = pkgs.writeShellApplication {
          name = "build-web-site";
          runtimeInputs = [
            ensureAgentLink
            pkgs.coreutils
            pkgs.gawk
            pkgs.gzip
            pkgs.jq
            pkgs.mdbook
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyNativeBuildInputs
          ++ bevyBuildInputs;
          text = withAgentLink (
            fastEnvironment
            + ''
              export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
              export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
              exec bash scripts/build-web-site.sh
            ''
          );
        };

        runWebServe = pkgs.writeShellApplication {
          name = "serve-web-site";
          runtimeInputs = [
            runWebBuild
            pkgs.python3
            pkgs.findutils
            pkgs.coreutils
          ];
          text = ''
            build-web-site
            site_dir="$(find target -maxdepth 1 -type d -name 'pages-site-local-*' -printf '%T@ %p\n' | sort -nr | head -n 1 | cut -d' ' -f2-)"
            test -n "$site_dir"
            exec python3 -m http.server 8000 --bind 127.0.0.1 --directory "$site_dir"
          '';
        };

        runDefault = runExample "minimal_2d";
        runDev = runExample "minimal_3d";
        runStress2d = runExample "stress_2d";
        runStress3d = runExample "stress_3d";

      in
      {
        packages = {
          default = coreLibrary;
          bevy-brp-mcp = bevyBrpMcp;
          coverage-report = coverageReport;
          "${packageName}" = coreLibrary;
        };

        apps =
          pkgs.lib.mapAttrs
            (name: app: app // { meta.description = "Bevy ray-traced audio ${name} command"; })
            {
              default = flake-utils.lib.mkApp { drv = runDefault; };
              bench = flake-utils.lib.mkApp { drv = runBenchmarks; };
              bevy-brp-mcp = flake-utils.lib.mkApp { drv = bevyBrpMcp; };
              coverage = flake-utils.lib.mkApp { drv = runCoverage; };
              dev = flake-utils.lib.mkApp { drv = runDev; };
              dylint = flake-utils.lib.mkApp { drv = runDylint; };
              demo-2d = flake-utils.lib.mkApp { drv = runDefault; };
              demo-3d = flake-utils.lib.mkApp { drv = runDev; };
              features = flake-utils.lib.mkApp { drv = runFeatureChecks; };
              fmt = flake-utils.lib.mkApp { drv = formatRepo; };
              setup-ai = flake-utils.lib.mkApp { drv = ensureAgentLink; };
              stress-2d = flake-utils.lib.mkApp { drv = runStress2d; };
              stress-3d = flake-utils.lib.mkApp { drv = runStress3d; };
              check = flake-utils.lib.mkApp { drv = runChecks; };
              clippy = flake-utils.lib.mkApp { drv = runClippy; };
              test = flake-utils.lib.mkApp { drv = runTests; };
              web-build = flake-utils.lib.mkApp { drv = runWebBuild; };
              web-serve = flake-utils.lib.mkApp { drv = runWebServe; };
            };

        checks = {
          build-cache = buildCacheCheck;
          fmt = formatCheck;
          clippy = clippyCheck;
          bevy-lint = bevyLintCheck;
          bench = benchmarkCheck;
          coverage = coverageReport;
          doctest = doctestCheck;
          docs = docsBookCheck;
          features = featureMatrixCheck;
          private-docs = privateDocsCheck;
          test = testCheck;
          wasm-examples = wasmExamplesCheck;
          package = coreLibrary;
        };

        formatter = formatRepo;

        devShells.default = pkgs.mkShell {
          packages = [
            bevyBrpMcp
            bevyCli
            pkgs.cmake
            pkgs.clang
            pkgs.cargo-llvm-cov
            pkgs.cargo-nextest
            ensureAgentLink
            pkgs.lld
            pkgs.mdbook
            pkgs.pkg-config
            treefmtEval.config.build.wrapper
            rustToolchain
          ]
          ++ cacheTools
          ++ bevyBuildInputs;
          shellHook = ''
            ${aiSupport.shellHook}
            export PKG_CONFIG_PATH="${pkgConfigPath}:''${PKG_CONFIG_PATH:-}"
            export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
            ${fastEnvironment}
          '';
        };
      }
    );
}
