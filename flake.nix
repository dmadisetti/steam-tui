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
              proton-caller
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
            buildInputs = [
              steamcmd
            ];
            # NOTE: Copied from pkgs.
            preFixup = ''
              mv $out/bin/steam-tui $out/bin/.steam-tui-unwrapped
              cat > $out/bin/steam-tui <<EOF
              #!${runtimeShell}
              export PATH=${steamcmd}/bin:\$PATH
              exec ${steam-run}/bin/steam-run $out/bin/.steam-tui-unwrapped '\$@'
              EOF
              chmod +x $out/bin/steam-tui
            '';
            checkFlags = [
              "--skip=impure"
            ];
            PKG_CONFIG_PATH = "${openssl.dev}/lib/pkgconfig";
            cargoLock = {
              lockFileContents = builtins.readFile ./Cargo.lock;
            };
          };

        packages.default = steam-tui;
      });
}
