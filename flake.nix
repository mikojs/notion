{
  description = "This library provides a simple interface to work with Notion pages, databases, and data sources while enforcing configurable permissions.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        { pkgs, self', ... }:
        {
          packages = {
            miko-notion = pkgs.rustPlatform.buildRustPackage {
              pname = "notion";
              version = "0.1.0";
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;

              buildFeatures = [
                "mcp"
              ];
            };

            default = self'.packages.miko-notion;
          };
        };

      flake.overlays.default = final: prev: {
        miko-notion = self.packages.${final.system}.miko-notion;
      };
    };
}
