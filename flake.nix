{
  description = "Mycelix-Health - Decentralized Healthcare on Holochain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    holonix = {
      url = "github:holochain/holonix/main-0.3";
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
        };

        # Holochain packages from holonix
        holochainPkgs = holonix.packages.${system};

        # Rust with WASM target for zome development
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

      in
      {
        devShells = {
          default = pkgs.mkShell {
            name = "mycelix-health";

            buildInputs = with pkgs; [
              # Holochain tooling
              holochainPkgs.holochain
              holochainPkgs.hc
              holochainPkgs.lair-keystore

              # Rust toolchain with WASM
              rustToolchain
              pkg-config
              openssl

              # Node.js for SDK development
              nodejs_20
              nodePackages.typescript
              nodePackages.npm

              # Utilities
              jq
              yq
            ];

            shellHook = ''
              echo ""
              echo "🏥 Mycelix-Health Development Environment"
              echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
              echo "  Holochain: $(holochain --version 2>/dev/null || echo 'not found')"
              echo "  Rust:      $(rustc --version 2>/dev/null || echo 'not found')"
              echo "  Node:      $(node --version 2>/dev/null || echo 'not found')"
              echo ""
              echo "📋 Commands:"
              echo "  cargo build --workspace                            # Build all zomes"
              echo "  cargo build --release --target wasm32-unknown-unknown -p <zome>  # Build single zome"
              echo "  hc dna pack dnas/health/                           # Pack DNA"
              echo "  hc app pack workdir/                               # Pack hApp"
              echo ""
              echo "🧪 Testing:"
              echo "  cargo test --workspace                             # Run all tests"
              echo "  cd sdk && npm test                                 # Run SDK tests"
              echo ""
              echo "📦 SDK Development:"
              echo "  cd sdk && npm install && npm run build            # Build SDK"
              echo "  cd sdk && npm run typecheck                       # Type check"
              echo ""

              # Set up cargo home if not set
              export CARGO_HOME="''${CARGO_HOME:-$HOME/.cargo}"

              # Add wasm target optimization
              export RUSTFLAGS="-C link-arg=-zstack-size=500000"
            '';

            RUST_BACKTRACE = 1;
            RUST_LOG = "info";
          };

          # Minimal shell for CI
          ci = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain
              pkg-config
              openssl
              nodejs_20
            ];
          };
        };

        packages = {
          # Build all zomes to WASM
          zomes = pkgs.stdenv.mkDerivation {
            name = "mycelix-health-zomes";
            src = ./.;

            nativeBuildInputs = with pkgs; [
              rustToolchain
              pkg-config
            ];

            buildInputs = with pkgs; [
              openssl
            ];

            buildPhase = ''
              export CARGO_HOME=$(mktemp -d)
              cargo build --release --target wasm32-unknown-unknown --workspace
            '';

            installPhase = ''
              mkdir -p $out/lib
              find target/wasm32-unknown-unknown/release -maxdepth 1 -name "*.wasm" -exec cp {} $out/lib/ \;
            '';

            # Fixed output derivation for caching
            outputHashMode = "recursive";
          };

          # Default is the zomes package
          default = self.packages.${system}.zomes;
        };

        # Checks for CI
        checks = {
          build = self.packages.${system}.zomes;

          fmt = pkgs.runCommand "check-fmt" {
            buildInputs = [ rustToolchain ];
          } ''
            cd ${self}
            cargo fmt --check
            touch $out
          '';
        };
      });
}
