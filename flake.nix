# Mycelix Health - Decentralized Healthcare on Holochain
# MVP Core: patient, provider, records, prescriptions, consent, bridge
#
# Note: This flake is self-contained because mycelix-health is a git submodule.
# It cannot import ../nix/modules/holochain-base.nix (path escapes flake boundary).
#
# Usage:
#   nix develop              # Enter dev shell
#   nix develop .#ci         # CI environment (minimal)
{
  description = "Mycelix-Health - Decentralized Healthcare on Holochain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    holonix = {
      url = "github:holochain/holonix/main-0.6";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, holonix, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };

        holochainPackages = holonix.packages.${system};

        # Rust toolchain with WASM target (inlined from holochain-base.nix)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rust-src" "rust-analyzer" "clippy" ];
        };

        # Bindgen configuration
        libclangPath = "${pkgs.llvmPackages.libclang.lib}/lib";
        clangResourceDir = "${pkgs.llvmPackages.clang.cc}/lib/clang/${pkgs.lib.versions.major pkgs.llvmPackages.clang.version}/include";
        bindgenArgs = builtins.concatStringsSep " " [
          "-I${pkgs.glibc.dev}/include"
          "-I${clangResourceDir}"
        ];

        # Environment variables for C/C++ compilation
        envVars = {
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
          HC_ADMIN_PORT = "4444";
          HC_APP_PORT = "8888";
          LIBCLANG_PATH = libclangPath;
          BINDGEN_EXTRA_CLANG_ARGS = bindgenArgs;
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
          PKG_CONFIG_PATH = pkgs.lib.concatStringsSep ":" [
            "${pkgs.openssl.dev}/lib/pkgconfig"
          ];
        };

        # Common build inputs (no Tauri deps — health is a hApp, not desktop app)
        commonBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          openssl
          openssl.dev
          cmake
          gnumake
          llvmPackages.libclang
          llvmPackages.clang
          glibc.dev
          stdenv.cc
          just
          jq
          holochainPackages.holochain
          holochainPackages.lair-keystore
          holochainPackages.hc
        ];

      in {
        devShells = {
          default = pkgs.mkShell ({
            name = "mycelix-health";
            buildInputs = commonBuildInputs ++ [ pkgs.nodejs_20 ];

            inherit (envVars)
              RUST_BACKTRACE RUST_LOG
              HC_ADMIN_PORT HC_APP_PORT
              LIBCLANG_PATH BINDGEN_EXTRA_CLANG_ARGS
              OPENSSL_DIR OPENSSL_LIB_DIR OPENSSL_INCLUDE_DIR
              PKG_CONFIG_PATH;

            shellHook = ''
              # Fix NixOS GCC 15 #include_next <stdlib.h> failure
              export NIX_CFLAGS_COMPILE="$(echo "$NIX_CFLAGS_COMPILE" | grep -oP '\-frandom-seed=\S+')"
              export NIX_CFLAGS_COMPILE_FOR_TARGET="$NIX_CFLAGS_COMPILE"

              echo ""
              echo "========================================"
              echo "  Mycelix Health"
              echo "  Holochain Development Environment"
              echo "========================================"
              echo ""
              echo "Holochain: $(holochain --version 2>/dev/null || echo 'loading...')"
              echo "hc:        $(hc --version 2>/dev/null || echo 'loading...')"
              echo "Rust:      $(rustc --version)"
              echo "Cargo:     $(cargo --version)"
              echo ""
              echo "MVP Core (7 zomes):"
              echo "  patient/        - Patient records & sovereignty"
              echo "  provider/       - Healthcare provider management"
              echo "  records/        - Medical record lifecycle"
              echo "  prescriptions/  - Prescription management"
              echo "  consent/        - Patient consent framework"
              echo "  bridge/         - Cross-domain health bridge"
              echo "  shared/         - Common types & utilities"
              echo ""
              echo "Commands:"
              echo "  cargo build                                     - Build library crates"
              echo "  cargo build --release --target wasm32-unknown-unknown - Build WASM zomes"
              echo "  cargo test                                      - Run unit tests"
              echo ""
            '';
          });

          ci = pkgs.mkShell {
            name = "mycelix-health-ci";
            buildInputs = with pkgs; [
              holochainPackages.holochain
              holochainPackages.hc
              rustToolchain
              pkg-config
              openssl
              openssl.dev
              llvmPackages.libclang
              llvmPackages.clang
              glibc.dev
              stdenv.cc
            ];

            inherit (envVars)
              LIBCLANG_PATH BINDGEN_EXTRA_CLANG_ARGS
              OPENSSL_DIR OPENSSL_LIB_DIR OPENSSL_INCLUDE_DIR;
          };
        };

        packages = {
          zomes = pkgs.stdenv.mkDerivation {
            name = "mycelix-health-zomes";
            src = ./.;
            nativeBuildInputs = [ rustToolchain pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            buildPhase = ''
              export HOME=$TMPDIR
              cargo build --release --target wasm32-unknown-unknown
            '';
            installPhase = ''
              mkdir -p $out/lib
              find target/wasm32-unknown-unknown/release -name "*.wasm" -exec cp {} $out/lib/ \;
            '';
          };
        };
      }
    );
}
