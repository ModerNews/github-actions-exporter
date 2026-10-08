{
  description = "github-actions-exporter — webhook-driven GitHub Actions metrics exporter";

  inputs = {
    # Same channel as ~/.nixos so the dev shell's rustc matches the system's.
    nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    # Calls `f` with the package set for each supported system.
    forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    inherit (nixpkgs) lib;
  in {
    packages = forAllSystems (pkgs: rec {
      default = github-actions-exporter;

      github-actions-exporter = pkgs.rustPlatform.buildRustPackage {
        pname = "github-actions-exporter";
        inherit ((lib.importTOML ./Cargo.toml).package) version;
        src = lib.cleanSource ./.;
        cargoLock.lockFile = ./Cargo.lock;

        meta = {
          description = "Webhook-driven GitHub Actions metrics exporter";
          mainProgram = "github-actions-exporter";
          platforms = systems;
        };
      };
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer
        ];
        # rust-analyzer needs the stdlib sources, which aren't in `rustc` here.
        env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    });

    # `nix flake check` — builds the crate and runs `cargo test` via buildRustPackage.
    checks = forAllSystems (pkgs: {
      inherit (self.packages.${pkgs.system}) github-actions-exporter;
    });

    formatter = forAllSystems (pkgs: pkgs.alejandra);
  };
}
