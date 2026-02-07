{
  description = "Hamr - A desktop launcher for Wayland";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use Rust 1.85+ for Edition 2024 support
        rustToolchain = pkgs.rust-bin.stable.latest.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Filter source to only include Rust files (no plugins)
        src = pkgs.lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*\\.py$" path != null)
            || (builtins.match ".*\\.json$" path != null)
            || (builtins.match ".*/plugins/.*" path != null);
        };

        # Build inputs needed for compilation
        buildInputs = with pkgs; [
          gtk4
          gtk4-layer-shell
          glib
          cairo
          pango
          gdk-pixbuf
          graphene
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
          pkgs.darwin.apple_sdk.frameworks.AppKit
        ];

        # Native build inputs (tools needed during build)
        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook4
        ];

        # Runtime dependencies for plugins
        runtimeDeps = with pkgs; [
          python3
          pulseaudio  # Provides paplay for sound notifications
          libqalculate  # For calculator plugin (qalc command)

          # Fonts required for UI
          material-symbols
          nerd-fonts.jetbrains-mono
        ];

        # Common arguments for all crane derivations
        commonArgs = {
          inherit src;
          strictDeps = true;
          inherit buildInputs nativeBuildInputs;

          # GTK4 needs these at build time
          PKG_CONFIG_PATH = "${pkgs.gtk4.dev}/lib/pkgconfig:${pkgs.gtk4-layer-shell}/lib/pkgconfig";
        };

        # Build only dependencies for caching
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "hamr-deps";
        });

        # Read version from Cargo.toml
        version = (pkgs.lib.importTOML ./Cargo.toml).workspace.package.version;

        # Separate derivation for plugins (no Rust toolchain hooks)
        hamr-plugins = pkgs.stdenv.mkDerivation {
          pname = "hamr-plugins";
          inherit version;
          
          src = ./plugins;
          
          installPhase = ''
            mkdir -p $out/share/hamr/plugins
            cp -r $src/* $out/share/hamr/plugins/
          '';
        };

        # Build only the Rust binaries (no plugins here)
        hamr-bin = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "hamr";
          # No postInstall - plugins are in separate derivation
        });

        # Combined package with binaries and plugins
        hamr = pkgs.symlinkJoin {
          name = "hamr";
          paths = [ hamr-bin hamr-plugins ];
          nativeBuildInputs = [ pkgs.wrapGAppsHook4 ];
          
          # Wrap binaries with runtime dependencies
          postBuild = ''
            wrapProgram $out/bin/hamr \
              --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps} \
              --prefix XDG_DATA_DIRS : ${pkgs.material-symbols}/share \
              --prefix XDG_DATA_DIRS : ${pkgs.nerd-fonts.jetbrains-mono}/share
            
            wrapProgram $out/bin/hamr-daemon \
              --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps}
            
            wrapProgram $out/bin/hamr-gtk \
              --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps} \
              --prefix XDG_DATA_DIRS : ${pkgs.material-symbols}/share \
              --prefix XDG_DATA_DIRS : ${pkgs.nerd-fonts.jetbrains-mono}/share
            
            wrapProgram $out/bin/hamr-tui \
              --prefix PATH : ${pkgs.lib.makeBinPath runtimeDeps}
          '';

          meta = with pkgs.lib; {
            description = "A desktop launcher for Wayland compositors";
            homepage = "https://github.com/stewart86/hamr";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.linux;
            mainProgram = "hamr";
          };
        };

        # Clippy check derivation
        hamrClippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

        # Test derivation
        hamrTest = craneLib.cargoTest (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        # Packages
        packages = {
          default = hamr;
          inherit hamr hamr-bin hamr-plugins;
        };

        # Checks run by `nix flake check`
        checks = {
          inherit hamr hamrClippy hamrTest;
        };

        # Development shell
        devShells.default = craneLib.devShell {
          inherit (hamr) buildInputs;

          packages = with pkgs; [
            rustToolchain
            rust-analyzer
            pkg-config
            # Development tools
            cargo-watch
            cargo-edit
          ] ++ runtimeDeps;

          # Set up environment for GTK development
          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.gtk4.dev}/lib/pkgconfig:${pkgs.gtk4-layer-shell}/lib/pkgconfig:$PKG_CONFIG_PATH"
          '';
        };

        # Apps for `nix run`
        apps = {
          default = flake-utils.lib.mkApp {
            drv = hamr;
            name = "hamr";
          };
          hamr-daemon = flake-utils.lib.mkApp {
            drv = hamr;
            name = "hamr-daemon";
          };
          hamr-gtk = flake-utils.lib.mkApp {
            drv = hamr;
            name = "hamr-gtk";
          };
          hamr-tui = flake-utils.lib.mkApp {
            drv = hamr;
            name = "hamr-tui";
          };
        };
      }
    );
}
