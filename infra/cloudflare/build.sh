#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

DX_VERSION="${DX_VERSION:-0.7.2}"
CC65_VERSION="${CC65_VERSION:-2.20}"

install_rust_toolchain() {
  if command -v cargo >/dev/null 2>&1; then
    return
  fi

  echo "[cf-build] Installing Rust toolchain"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal
}

install_cc65() {
  if command -v cl65 >/dev/null 2>&1; then
    return
  fi

  echo "[cf-build] Installing cc65"

  if command -v apt-get >/dev/null 2>&1; then
    if command -v sudo >/dev/null 2>&1; then
      if ! (sudo apt-get update && sudo apt-get install -y cc65); then
        echo "[cf-build] WARN: apt install failed, falling back to source build"
      fi
    else
      if ! (apt-get update && apt-get install -y cc65); then
        echo "[cf-build] WARN: apt install failed, falling back to source build"
      fi
    fi
  fi

  if command -v cl65 >/dev/null 2>&1; then
    return
  fi

  local tmp
  tmp="$(mktemp -d)"

  curl -fsSL "https://github.com/cc65/cc65/archive/refs/tags/V${CC65_VERSION}.tar.gz" \
    | tar -xzf - -C "$tmp"

  make -C "$tmp/cc65-${CC65_VERSION}" -j"$(getconf _NPROCESSORS_ONLN || echo 2)"
  make -C "$tmp/cc65-${CC65_VERSION}" PREFIX="$HOME/.local" install

  rm -rf "$tmp"
}

install_dx() {
  local install=0

  if ! command -v dx >/dev/null 2>&1; then
    install=1
  else
    local found
    found="$(dx --version 2>/dev/null | awk '{print $2}')"
    if [[ "$found" != "$DX_VERSION" ]]; then
      install=1
    fi
  fi

  if [[ "$install" -eq 1 ]]; then
    echo "[cf-build] Installing Dioxus CLI $DX_VERSION"
    cargo install dioxus-cli --version "$DX_VERSION" --locked
  fi
}

main() {
  install_rust_toolchain
  export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

  rustup target add wasm32-unknown-unknown
  install_cc65
  install_dx

  echo "[cf-build] Building SmashDotNES site artifact"
  cd "$ROOT"
  make ci-pages
}

main "$@"
