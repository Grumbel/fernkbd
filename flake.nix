# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 Ingo Ruhnke <grumbel@gmail.com>
{
  description = "fernkbd — inject terminal input as virtual keyboard events via uinput/evdev";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        versionBase = nixpkgs.lib.strings.removeSuffix "\n" (builtins.readFile ./VERSION);
        gitRev = "${self.shortRev or self.dirtyShortRev or "dirty"}";
        isDev = nixpkgs.lib.strings.hasInfix "-dev" versionBase;
        version =
          if isDev then
            "${versionBase}.${toString (self.revCount or 0)}+g${gitRev}"
          else
            versionBase;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "fernkbd";
          inherit version;
          src = self;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          # Same full version string the binary reports via --version.
          FERNKBD_VERSION = version;

          postInstall = ''
            install -Dm644 man/fernkbd.1 $out/share/man/man1/fernkbd.1
          '';

          meta = with pkgs.lib; {
            description = "Virtual keyboard that injects terminal input via uinput/evdev";
            homepage = "https://github.com/Grumbel/fernkbd";
            license = licenses.gpl3Plus;
            maintainers = [ ];
            platforms = platforms.linux;
            mainProgram = "fernkbd";
          };
        };

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
          ];
        };
      });
}
