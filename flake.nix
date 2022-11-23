{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      rustVersion = pkgs.rust-bin.stable.latest.default;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustVersion;
        rustc = rustVersion;
      };

      rustbuild_dpsa4fl-janus-tasks = rustPlatform.buildRustPackage {
        pname =
          "dpsa4fl-janus-tasks"; # make this what ever your cargo.toml package.name is
        version = "0.1.0";
        src = ./.; # the folder with the cargo.toml
        cargoLock.lockFile = ./Cargo.lock;
        cargoLock.outputHashes = {};
        #   "janus_aggregator-0.2.0" = "sha256-+mj6QwjfpAR92+0UoLJnnZGhKS4W66gELBNEHs86P/M=";
        # };
        cargoBuildFlags = ""; #"-p janus_aggregator";
        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [
          pkgs.openssl
        ];
        doCheck = false;
      };

    in {

      packages = {
        binary_dpsa4fl-janus-tasks = rustbuild_dpsa4fl-janus-tasks;
      };
      defaultPackage = rustbuild_dpsa4fl-janus-tasks;

      devShell = pkgs.mkShell {
        nativeBuildInputs = [ pkgs.bashInteractive pkgs.pkg-config ];
        buildInputs = with pkgs; [
          # maturin
          # poetry
          # python39Packages.pipx
          openssl
          # postgresql_14
        ];
      };
    });
}
