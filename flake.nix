{
  description = "Rust dev environment with GCC and libudev";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system;
          overlays = overlays;
        };

        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rust
            pkgs.gcc
            pkgs.pkg-config
            pkgs.systemd
            pkgs.openssl
            pkgs.python313
            pkgs.uv
            pkgs.maturin
            pkgs.zlib # numpy
            pkgs.stdenv.cc.cc.lib # numpy
            pkgs.chromium # for when python needs plotly saving support
          ];
          shellHook = ''
            export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.zlib}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.openssl}/lib
            export PATH=${pkgs.chromium}/bin:$PATH
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
          '';

        };
        packages.default = pkgs.callPackage ./default.nix { };

        packages.rex = self.packages.${system}.default;

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/rex";
        };

      }
    );
}
