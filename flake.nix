{
  description = "Hamr - Extensible launcher for Hyprland and Niri built with Quickshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    hamr-src = {
      url = "github:Stewart86/hamr";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    hamr-src,
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux"];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

    pkgsFor = system:
      import nixpkgs {
        inherit system;
        overlays = [self.overlays.default];
      };
  in {
    overlays.default = final: prev: {
      hamr = final.callPackage ./package.nix {
        src = hamr-src;
        rev = hamr-src.shortRev or "dirty";
      };
    };

    packages = forAllSystems (system: {
      default = (pkgsFor system).hamr;
      hamr = (pkgsFor system).hamr;
    });

    # For `nix run`
    apps = forAllSystems (system: {
      default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/hamr";
      };
    });
  };
}
