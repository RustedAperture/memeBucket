{
  description = "MemeBucket development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer

            # Node.js toolchain
            nodejs_22

            # Databases and tools
            sqlite
            sqlx-cli

            # GitHub CLI
            gh

            # Runtime dependencies
            ffmpeg

            # Build tools and system dependencies
            pkg-config
            openssl
            gcc
            gnumake

            # Tauri (desktop app) CLI and Linux runtime deps
            cargo-tauri
            glib
            gtk3
            libsoup_3
            webkitgtk_4_1
            librsvg
            libayatana-appindicator
            xdotool # provides libxdo, needed by the enigo crate
            glib-networking # provides the GIO TLS backend the webview needs for HTTPS
          ];

          # libayatana-appindicator is dlopen'd at runtime by the tray-icon
          # feature rather than linked, so it needs to be on LD_LIBRARY_PATH
          # explicitly instead of relying on the linker's automatic rpath.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.libayatana-appindicator ];

          # webkitgtk's networking (libsoup/GIO) discovers its TLS backend via
          # this module path rather than normal linking; without it HTTPS
          # requests inside the webview fail with "TLS support is not available".
          GIO_EXTRA_MODULES = "${pkgs.glib-networking}/lib/gio/modules";

          shellHook = ''
            echo "===================================================="
            echo "  Welcome to the MemeBucket development environment!"
            echo "===================================================="
            echo "Available tools:"
            echo "  - Rust: $(rustc --version)"
            echo "  - Node: $(node --version)"
            echo "  - SQLite: $(sqlite3 --version)"
            echo "  - FFmpeg: $(ffmpeg -version | head -n 1)"
            echo "  - Tauri CLI: $(cargo tauri --version)"
            echo "===================================================="
          '';
        };
      }
    );
}
