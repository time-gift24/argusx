# eventsource-stream patches

This directory stores local modifications on top of the vendored
`vendor/eventsource_stream` upstream mirror.

## Current patch

- `0001-local-compat-and-parser-fixes.patch`: carries the full local delta on top of the
  vendored upstream mirror, including `nom` 8 compatibility updates and the empty-line SSE
  parsing fix required to flush complete events.

## Verify

From repo root:

```bash
./scripts/check-vendor-patches.sh
```

## Regenerate patch

If you update upstream sync commit and patch commit, regenerate with:

```bash
rm -f patches/eventsource-stream/*.patch
git format-patch --stdout <upstream_sync_commit>..<latest_patch_commit> -- vendor/eventsource_stream \
  > patches/eventsource-stream/0001-local-compat-and-parser-fixes.patch
```
