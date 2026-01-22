{
  description = "Mycelix-Health - Decentralized Healthcare on Holochain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    holonix.url = "github:holochain/holonix/main-0.3";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, holonix, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Import shared Holochain base module
        holochainBase = import ../nix/modules/holochain-base.nix {
          inherit pkgs system;
          holochainPackages = holonix.packages.${system};
        };
      in
      {
        devShells = {
          default = holochainBase.mkHolochainShell {
            name = "health";
            extraBuildInputs = with pkgs; [
              nodejs_20
              nodePackages.typescript
            ];
            extraShellHook = ''
              echo "🏥 Mycelix-Health Development Environment"
              echo "   Holochain: $(holochain --version 2>/dev/null || echo 'not found')"
              echo ""
              echo "Commands:"
              echo "  cargo build --release --target wasm32-unknown-unknown  # Build zomes"
              echo "  hc dna pack dna/                                       # Pack DNA"
              echo "  hc app pack .                                          # Pack hApp"
              echo ""
            '';
          };
        };

        packages = {
          # Build all zomes
          zomes = pkgs.stdenv.mkDerivation {
            name = "mycelix-health-zomes";
            src = ./.;
            buildInputs = with pkgs; [
              (rust-bin.stable.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              })
            ];
            buildPhase = ''
              cargo build --release --target wasm32-unknown-unknown
            '';
            installPhase = ''
              mkdir -p $out/lib
              cp target/wasm32-unknown-unknown/release/*.wasm $out/lib/
            '';
          };
        };
      });
}
