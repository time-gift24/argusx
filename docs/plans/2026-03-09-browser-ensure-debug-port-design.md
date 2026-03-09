# Browser Ensure Debug Port Design

## Goal

Add a first-class browser action, `ensure_debug_port`, that makes an existing Chrome instance reachable over CDP on port `9222` without forcing the user to manually restart the browser. The action must support `macOS` and `Windows`.

## User-facing behavior

`ensure_debug_port` is an explicit browser tool action. It does not run implicitly during normal browser operations.

When invoked, it should:

1. Check whether `127.0.0.1:9222` is already serving a Chrome DevTools endpoint.
2. If yes, return success immediately with diagnostic details.
3. If not, capture the current Chrome session as completely as practical.
4. Restart Chrome with `--remote-debugging-port=9222`.
5. Wait for the CDP endpoint to come up.
6. Let Chrome restore its prior session and reopen any missing tabs from the saved snapshot.
7. Return structured diagnostics describing what happened.

## Scope

Supported:

- `macOS`
- `Windows`
- existing Chrome session snapshot and restore
- ordinary web URLs
- best-effort recovery of `chrome://` pages, extension pages, and pinned tabs

Not guaranteed:

- exact scroll position
- unsaved form state
- transient in-page JS state
- exact window geometry parity across restart

## Architectural approach

The feature belongs inside the browser builtin, not desktop bootstrap. The current browser tool already owns Chrome lifecycle concerns through `ChromeManager`, so `ensure_debug_port` should live alongside `connect_or_launch()` rather than becoming a desktop-only escape hatch.

The implementation should be split into three layers:

1. **Debug port probe**
   - detect TCP reachability
   - verify `/json/version`
   - return quickly if already enabled

2. **Platform session capture / restart**
   - `macOS`: AppleScript capture + quit + relaunch
   - `Windows`: PowerShell capture + stop process + relaunch

3. **Best-effort restore coordinator**
   - wait for CDP readiness
   - compare saved snapshot against current targets
   - reopen missing tabs

## Session snapshot model

We need a browser-session snapshot type that is runtime-only, not persisted in SQLite. It should capture enough data to do best-effort replay:

- browser executable path if discoverable
- windows in order
- tabs in order
- active tab index per window
- URL
- title if available
- whether the tab was pinned if discoverable
- page category: ordinary URL / chrome-internal / extension / unknown

This snapshot is a restore aid, not a truth source.

## Platform-specific strategy

### macOS

Use `osascript` / AppleScript against `Google Chrome`:

- enumerate windows and tabs
- collect URL + title + active tab position
- quit application
- relaunch via `open -na "Google Chrome" --args --remote-debugging-port=9222 ...`

AppleScript is the most pragmatic route here. It is already the platform-native automation surface for Chrome window/tab introspection.

### Windows

Use PowerShell for orchestration:

- enumerate running Chrome process state
- attempt COM/UI scripting to capture tab URLs where possible
- stop Chrome
- relaunch `chrome.exe --remote-debugging-port=9222`

Windows capture will likely be less complete than macOS. The design must tolerate partial snapshots and still rely on Chrome's own session restore as the primary mechanism.

## Restore semantics

The restore order matters:

1. Restart Chrome with the same profile.
2. Wait for CDP readiness.
3. Give Chrome a short grace period to restore its own session.
4. Compare restored targets with the saved snapshot.
5. Open missing tabs in order.

`chrome://` and extension pages should be attempted if the saved URL is present, but failures should be reported, not treated as fatal for the whole action.

## Tool contract

Add a new browser action:

- `ensure_debug_port`

Optional arguments:

- `port` default `9222`
- `timeout_ms` for restart/probe window

Return payload should include:

- `already_enabled`
- `restarted`
- `port`
- `captured_window_count`
- `captured_tab_count`
- `restored_tab_count`
- `skipped_tab_count`
- `warnings`

## Risks

- Platform automation can fail if Chrome is not scriptable or OS permissions block inspection.
- Restoring internal pages and extension pages may be incomplete.
- Relaunching the wrong Chrome executable can switch channels or profiles.

The implementation should bias toward:

- clear diagnostics
- no silent destructive behavior
- preserving user profile/session over perfect replay

## Verification

- Unit tests for action argument parsing and result shaping.
- Platform-gated tests for snapshot command generation.
- Manual verification on `macOS`:
  - open multiple tabs including ordinary URL and `chrome://`
  - invoke `ensure_debug_port`
  - verify `9222` comes up and session mostly restores
- Manual verification on `Windows` with the same scenario.
