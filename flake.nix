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

          devShells.default = inputs.pebble.pebbleEnv."${system}" {
            nativeBuildInputs = [
              (pkgs.fenix.combine (
                [ pkgs.fenix.stable.toolchain ]
                ++ (builtins.concatMap (target: [ pkgs.fenix.targets."${target}".stable.rust-std ]) [
                  "thumbv7m-none-eabi"
                  "thumbv7em-none-eabi"
                ])
              ))
            ];

            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = ''
              -I${inputs'.pebble.packages.arm-embedded-toolchain}/arm-none-eabi/include \
              -I${inputs'.pebble.packages.arm-embedded-toolchain}/lib/gcc/arm-none-eabi/4.7.4/include
            '';
          };
        };
    };
}
