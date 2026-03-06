# eventsource-stream patches

This directory stores local modifications on top of the vendored
`vendor/eventsource_stream` upstream mirror.

## Current patch

- `0001-nom8-compat.patch`: updates parser usage for `nom` 8 API compatibility.

## Verify

From repo root:

```bash
./scripts/check-vendor-patches.sh
```

## Regenerate patch

If you update upstream sync commit and patch commit, regenerate with:

```bash
git format-patch --stdout <upstream_sync_commit>..<patch_commit> -- vendor/eventsource_stream \
  > patches/eventsource-stream/0001-nom8-compat.patch
```
