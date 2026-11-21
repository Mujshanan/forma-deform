#!/usr/bin/env bash
set -euo pipefail

# Reusable engines and orchestration must not acquire AniCraft host/ABI dependencies.
reusable_crates=(
  forma-geometry
  smart-controller-core
  form-deform-core
  forma-deformation-integration
)

for crate in "${reusable_crates[@]}"; do
  if cargo tree --edges normal --package "$crate" | grep -Fq 'anicraft-plugin-sdk'; then
    echo "Adapter boundary violation: reusable crate '$crate' depends on anicraft-plugin-sdk." >&2
    exit 1
  fi
done

# Every host-facing plugin must enter AniCraft through the vendored SDK.
plugin_crates=(
  anicraft-bone-query-plugin
  anicraft-form-deform-plugin
  anicraft-smart-controller-plugin
)

for plugin in "${plugin_crates[@]}"; do
  if ! cargo tree --edges normal --package "$plugin" | grep -Fq 'anicraft-plugin-sdk'; then
    echo "Adapter boundary violation: plugin '$plugin' does not depend on anicraft-plugin-sdk." >&2
    exit 1
  fi
done

echo "Adapter dependency boundaries verified."
