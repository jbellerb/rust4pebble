{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pebble = {
      url = "github:jbellerb/pebble.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-parts,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];

      perSystem =
        {
          pkgs,
          inputs',
          system,
          ...
        }:
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.fenix.overlays.default ];
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs =
              [
                (pkgs.fenix.combine (
                  builtins.concatMap (target: [ pkgs.fenix.targets."${target}".stable.rust-std ]) [
                    "thumbv7m-none-eabi"
                    "thumbv7em-none-eabi"
                  ]
                ))
              ]
              ++ (with inputs'.pebble.packages; [
                pebble-qemu
                pebble-tool
              ]);

            CFLAGS = "";
          };
        };
    };
}
