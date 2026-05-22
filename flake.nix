{
  description = "Fast Mermaid diagram renderer in pure Rust - 23 diagram types, 100-1400x faster than mermaid-cli";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        mmdr = pkgs.rustPlatform.buildRustPackage {
          pname = "mmdr";
          version = "0.2.2";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
          meta = {
            description = "Fast Mermaid diagram renderer in pure Rust - 23 diagram types, 100-1400x faster than mermaid-cli";
            homepage = "https://github.com/1jehuang/mermaid-rs-renderer";
            license = pkgs.lib.licenses.mit;
            mainProgram = "mmdr";
          };
        };
      in
      {
        packages.default = mmdr;
        apps.default = {
          type = "app";
          program = "${mmdr}/bin/mmdr";
        };
      }
    );
}
