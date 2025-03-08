{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    pebble = {
      url = "github:jbellerb/pebble.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
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
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          devShells.default = inputs.pebble.pebbleEnv."${system}" {
            nativeBuildInputs = [
              (pkgs.rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
                targets = [
                  "thumbv7m-none-eabi"
                  "thumbv7em-none-eabi"
                ];
              })
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
