{
  description = "Hamr - Extensible launcher for Hyprland and Niri built with Quickshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    supportedSystems = ["x86_64-linux" "aarch64-linux"];
    forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

    pkgsFor = system:
      import nixpkgs {
        inherit system;
        overlays = [self.overlays.default];
      };

    version = builtins.replaceStrings ["\n"] [""] (builtins.readFile ./VERSION);
  in {
    overlays.default = final: prev: {
      hamr = final.callPackage ./package.nix {
        src = self;
        inherit version;
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
