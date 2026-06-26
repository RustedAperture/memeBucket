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

            # Runtime dependencies
            ffmpeg

            # Build tools and system dependencies
            pkg-config
            openssl
            gcc
            gnumake
          ];

          shellHook = ''
            echo "===================================================="
            echo "  Welcome to the MemeBucket development environment!"
            echo "===================================================="
            echo "Available tools:"
            echo "  - Rust: $(rustc --version)"
            echo "  - Node: $(node --version)"
            echo "  - SQLite: $(sqlite3 --version)"
            echo "  - FFmpeg: $(ffmpeg -version | head -n 1)"
            echo "===================================================="
          '';
        };
      }
    );
}
