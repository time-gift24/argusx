# Browser Debug CLI Design

## Goal

Add a temporary CLI under the `tool` crate to diagnose browser-tool connection failures against an already-running Chrome instance and a fresh headless launch.

## Scope

The CLI should answer these questions with explicit output:

1. Is the configured remote debugging port reachable?
2. Does the CDP HTTP surface respond on `/json/version` and `/json/list`?
3. Can `chromiumoxide::Browser::connect()` attach to that endpoint?
4. If attach fails, can a fresh headless Chrome launch still succeed?

## Non-Goals

- No integration into desktop commands or runtime bootstrap.
- No production-facing UX.
- No attempt to fix browser-tool behavior yet.

## Approach

Implement a standalone binary in `tool/src/bin/browser_debug.rs` with two modes:

- `connect`: probe an existing Chrome devtools endpoint on `localhost:<port>`
- `launch`: launch a fresh Chrome through `chromiumoxide`, optionally headless

The CLI will print structured diagnostics for each phase:

- TCP reachability
- HTTP endpoint responses
- discovered page targets
- `chromiumoxide` attach/launch result
- current page list / URLs after connection

## Why this shape

This keeps the debugging surface independent from the current `BrowserTool` control flow while still reusing the same browser stack (`chromiumoxide`). That makes it useful for isolating whether the bug is in:

- Chrome startup assumptions
- remote-debugging-port expectations
- `Browser::connect` behavior
- our higher-level tool/session logic

## Verification

- Add unit tests for CLI argument parsing / mode selection.
- Build the binary with `cargo test -p tool browser_debug` and `cargo run -p tool --bin browser_debug -- --help`.
