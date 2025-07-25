{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShells.default = mkShell rec {
          buildInputs = [
            (rust-bin.nightly.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" "miri" ];
              targets = [ "x86_64-unknown-linux-gnu" "thumbv8m.main-none-eabihf" ];
            })
            probe-rs
            picotool
          ];
        };
      });
}
