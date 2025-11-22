{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.pkgsCross.aarch64-multiplatform-musl.stdenv.cc
  ];

  shellHook = ''
    export RUSTUP_HOME="$PWD/.rustup"
    export CARGO_HOME="$PWD/.cargo"
    export PATH="$CARGO_HOME/bin:$PATH"

    if ! rustup toolchain list | grep -q '^stable'; then
      rustup toolchain install stable
    fi
    rustup default stable

    rustup target add aarch64-unknown-linux-musl

    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="${pkgs.pkgsCross.aarch64-multiplatform-musl.stdenv.cc.targetPrefix}cc"
  '';
}
