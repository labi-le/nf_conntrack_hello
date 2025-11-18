{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc
  ];
}
