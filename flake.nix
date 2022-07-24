{
  description = "Flake to manage projects + builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
      in
      rec {
        devShells.default = with pkgs;
          pkgs.mkShell {
            packages = [
              # build
              rustfmt
              rustc
              cargo
              clippy
              rustup

              # deps
              pkg-config
              openssl

              # steam
              steam
              steamcmd
              steam-run

              # misc
              wine
              python3
              lutris
            ];
            STEAM_RUN_WRAPPER = "${steam-run}/bin/steam-run";
          };

        steam-tui = with pkgs;
          rustPlatform.buildRustPackage rec {
            name = "steam-tui-dev";
            pname = "steam-tui";
            src = ./.;
            nativeBuildInputs = [
              openssl
              pkgconfig
            ];
            packages = [
              steamcmd
            ];
            PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";
            cargoLock = {
              lockFileContents = builtins.readFile ./Cargo.lock;
            };
          };

        packages.default = steam-tui;
      });
}
