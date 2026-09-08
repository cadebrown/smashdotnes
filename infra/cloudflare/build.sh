#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

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

  local jobs
  jobs="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)"
  make -C "$src_root" -j"$jobs" bin QUIET=1
  make -C "$src_root" -j"$jobs" nes QUIET=1

  make -C "$src_root/src" PREFIX="$HOME/.local" install

  local data_root="$HOME/.local/share/cc65"
  install -d "$data_root/asminc" "$data_root/cfg" "$data_root/include" "$data_root/lib"
  cp -R "$src_root/asminc/." "$data_root/asminc/"
  cp -R "$src_root/cfg/." "$data_root/cfg/"
  cp -R "$src_root/include/." "$data_root/include/"
  install -m0644 "$src_root/lib/nes.lib" "$data_root/lib/nes.lib"
  export CC65_HOME="$data_root"

  rm -rf "$tmp"
}

configure_cc65_env() {
  if [[ -n "${CC65_HOME:-}" ]]; then
    return
  fi

  local installed_home="$HOME/.local/share/cc65"
  if [[ -d "$installed_home/include" && -f "$installed_home/lib/nes.lib" ]]; then
    export CC65_HOME="$installed_home"
    echo "[cf-build] Using CC65_HOME=$CC65_HOME"
    return
  fi

  if ! command -v cl65 >/dev/null 2>&1; then
    return
  fi

  local target_path
  target_path="$(cl65 --print-target-path 2>/dev/null || true)"
  if [[ -z "$target_path" ]]; then
    return
  fi

  local cc65_home
  cc65_home="$(dirname "$target_path")"
  if [[ ! -d "$cc65_home/include" ]]; then
    return
  fi

  export CC65_HOME="$cc65_home"
  echo "[cf-build] Using CC65_HOME=$CC65_HOME"
}

main() {
  install_rust_toolchain
  export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

  install_cc65
  configure_cc65_env

  echo "[cf-build] Building SmashDotNES site artifact"
  cd "$ROOT"
  make ci-pages
}

main "$@"
