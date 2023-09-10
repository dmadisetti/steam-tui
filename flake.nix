{
  description = "steam-tui flake to manage projects + builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/master";
  };

  outputs = { self, nixpkgs, ... }@inputs:
    let
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.platforms.unix;

      nixpkgsFor = forAllSystems (system: import nixpkgs {
        inherit system;
        config = {
          allowUnfreePredicate = pkg: builtins.elem (nixpkgs.lib.getName pkg) [
            "steam"
            "steamcmd"
            "steam-original"
            "steam-run"
          ];
        };
      });
    in
    {
      packages = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
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

          default = self.packages.${system}.steam-tui;
        });

      devShells = forAllSystems (system:
        let pkgs = nixpkgsFor.${system}; in {
          default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self.packages.${system};
            buildInputs = with pkgs; [
              # build
              rustfmt
              rustc
              cargo
              clippy
              rustup

              # steam
              steam
              steam-run

              # misc
              wine
              proton-caller
              python3
            ];
            STEAM_RUN_WRAPPER = "${pkgs.steam-run}/bin/steam-run";
          };
        });
    };
}
