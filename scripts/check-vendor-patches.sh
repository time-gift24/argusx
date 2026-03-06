#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH_DIR="patches/eventsource-stream"

if [[ ! -d "$PATCH_DIR" ]]; then
  echo "No vendor patch directory found at $PATCH_DIR"
  exit 0
fi

shopt -s nullglob
patches=("$PATCH_DIR"/*.patch)
shopt -u nullglob

if [[ ${#patches[@]} -eq 0 ]]; then
  echo "No patch files found in $PATCH_DIR"
  exit 0
fi

for patch in "${patches[@]}"; do
  echo "Checking applied patch: $patch"
  # Reverse-check ensures the patch exactly matches current tree state.
  git apply --check --reverse "$patch"
done

echo "All vendor patches are applied cleanly."
