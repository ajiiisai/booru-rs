{
  description = "Rust development shell for booru-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forEachSystem (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          msrv = manifest.package.rust-version;
          rustVersion =
            if builtins.length (pkgs.lib.splitString "." msrv) == 2
            then "${msrv}.0"
            else msrv;
          rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          };
        in
        {
          packages.dev-tools = pkgs.buildEnv {
            name = "booru-rs-dev-tools";
            paths = [ rust pkgs.cargo-nextest ];
          };

          default = pkgs.mkShell {
            packages = [ rust pkgs.cargo-nextest ];
          };
        });
    };
}
