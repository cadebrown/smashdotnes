#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

DX_VERSION="${DX_VERSION:-0.7.2}"
CC65_TAG="${CC65_TAG:-V2.19}"

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

  local cc65_tag="$CC65_TAG"
  local cc65_ver="${cc65_tag#V}"
  cc65_ver="${cc65_ver#v}"
  local cc65_dir_name="cc65-${cc65_ver}"

  local tmp
  tmp="$(mktemp -d)"
  local src_root="$tmp/$cc65_dir_name"

  if ! (curl -fsSL "https://github.com/cc65/cc65/archive/refs/tags/${cc65_tag}.tar.gz" | tar -xzf - -C "$tmp"); then
    echo "[cf-build] WARN: cc65 tag '${cc65_tag}' not found, resolving latest tag"
    cc65_tag="$(curl -fsSL 'https://api.github.com/repos/cc65/cc65/tags?per_page=1' | sed -n 's/.*"name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
    if [[ -z "$cc65_tag" ]]; then
      echo "[cf-build] ERROR: could not resolve a valid cc65 release tag"
      exit 1
    fi
    cc65_ver="${cc65_tag#V}"
    cc65_ver="${cc65_ver#v}"
    cc65_dir_name="cc65-${cc65_ver}"
    src_root="$tmp/$cc65_dir_name"
    curl -fsSL "https://github.com/cc65/cc65/archive/refs/tags/${cc65_tag}.tar.gz" | tar -xzf - -C "$tmp"
  fi

  if [[ ! -d "$src_root" ]]; then
    src_root="$(find "$tmp" -mindepth 1 -maxdepth 1 -type d | head -n1)"
  fi
  if [[ -z "$src_root" || ! -d "$src_root" ]]; then
    echo "[cf-build] ERROR: failed to locate extracted cc65 source directory"
    exit 1
  fi

  make -C "$src_root" -j"$(getconf _NPROCESSORS_ONLN || echo 2)"
  make -C "$src_root" PREFIX="$HOME/.local" install

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
