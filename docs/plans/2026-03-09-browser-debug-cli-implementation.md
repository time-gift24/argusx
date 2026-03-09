# Browser Debug CLI Implementation Plan

1. Add a temporary `browser_debug` binary under `tool/src/bin/`.
2. Parse a minimal argument set:
   - subcommand `connect` or `launch`
   - `--port`
   - `--headless` for launch mode
   - optional `--chrome-path`
3. Add a small test-first parsing layer that maps argv into a debug command enum.
4. Implement connect diagnostics:
   - TCP connect
   - GET `/json/version`
   - GET `/json/list`
   - `Browser::connect`
   - page enumeration
5. Implement launch diagnostics:
   - launch via `chromiumoxide::Browser::launch`
   - page enumeration
   - print effective configuration including headless state
6. Run targeted tests and a binary help/build check.
