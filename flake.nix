{
  description = "Auction Sniper in Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, naersk, nixpkgs, flake-utils, flake-compat, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust-bin = pkgs.rust-bin.stable."1.71.0".default.override {
        extensions = [ "rust-analyzer" "clippy" ];
      };
      naersk-lib = naersk.lib."${system}".override {
        cargo = rust-bin;
        rustc = rust-bin;
      };
    in {
      packages.pariter = naersk-lib.buildPackage ./.;

      defaultPackage = self.packages.${system}.pariter;
      defaultApp = self.packages.${system}.pariter;

      # `nix develop`
      devShell = pkgs.mkShell
        {
          inputsFrom = builtins.attrValues self.packages.${system};
          buildInputs = [];
          nativeBuildInputs = [ rust-bin ];
        };
  });
}
