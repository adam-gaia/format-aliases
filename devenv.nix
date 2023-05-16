{ inputs
, pkgs
, lib
, treefmt-nix
, ...
}: {
  packages = with pkgs; [
    rust-bin.nightly.latest.default
    clippy
    rust-analyzer
    cargo-nextest
    cargo-limit
    cargo-audit
    cargo-watch
    cargo-expand
    just
    statix
    (treefmt-nix.lib.mkWrapper pkgs (import ./treefmt.nix))
  ];
}
