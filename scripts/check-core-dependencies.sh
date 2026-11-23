#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CORE_CRATES=(
  "form-deform-core"
  "smart-controller-core"
)

fail() {
  printf 'Core dependency policy violation: %s\n' "$1" >&2
  exit 1
}

for crate in "${CORE_CRATES[@]}"; do
  manifest="crates/$crate/Cargo.toml"
  source_dir="crates/$crate/src"

  [[ -f "$manifest" ]] || fail "missing core manifest: $manifest"
  [[ -d "$source_dir" ]] || fail "missing core source directory: $source_dir"

  if grep -nE 'anicraft-plugin-sdk|anicraft_plugin_sdk' "$manifest"; then
    fail "$crate must not depend on the AniCraft plugin SDK"
  fi

  if grep -RInE 'anicraft_plugin_sdk|plugin![[:space:]]*\\(|exporter![[:space:]]*\\(|ease![[:space:]]*\\(' "$source_dir"; then
    fail "$crate source must not import or invoke the plugin ABI"
  fi

  if cargo tree -p "$crate" --edges normal | grep -q 'anicraft-plugin-sdk'; then
    fail "$crate dependency graph contains the AniCraft plugin SDK"
  fi
done

# Adapter crates are the only production consumers of the vendored ABI SDK.
for adapter in plugins/form-deform plugins/smart-controller plugins/bone-query; do
  manifest="$adapter/Cargo.toml"
  [[ -f "$manifest" ]] || fail "missing adapter manifest: $manifest"
  grep -q 'anicraft-plugin-sdk' "$manifest" || fail "$adapter must declare its SDK dependency"
done

printf 'Core dependency boundary checks passed.\n'

# Exercise each native core independently; no WASM target is required here.
cargo test -p form-deform-core
cargo test -p smart-controller-core
