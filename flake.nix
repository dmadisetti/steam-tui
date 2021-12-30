{
  description = "Flake to manage python workspace";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, mach-nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            pkgs.openssl
            pkgs.rustc
            pkgs.rustfmt
          ];
          packages = [
            # app packages
            pkgs.cargo
            pkgs.rustup
            pkgs.openssl
            pkgs.pkg-config
            pkgs.steam
            pkgs.steamcmd
            pkgs.wine
            pkgs.python3
            pkgs.lutris
          ];
        };
        defaultPackage =
          # Notice the reference to nixpkgs here.
          with import nixpkgs
            {
              system = system;
              config.allowUnfree = true;
            };
          stdenv.mkDerivation {
            name = "steam-tui";
            src = self;
            buildInputs = [
              # app packages
              cargo
              steamcmd
              openssl
            ];
            installPhase = ''
              cargo build
            '';
          };
      });
}
