# Desktop Window Size Design

Date: 2026-03-06
Status: Approved for implementation
Scope: Increase the default Tauri desktop window size for large-monitor usage without changing startup behavior to maximized

## 1. Goal

Make the desktop app open at a sensible size for 27-inch and 31.5-inch displays:

- replace the current small `800x600` default window
- start centered on screen
- keep the app as a window, not maximized
- prevent users from shrinking it back to a cramped size

## 2. Locked Decisions

1. Window sizing policy
- default size becomes `1728x1080`
- minimum size becomes `1440x900`
- the window starts centered
- do not enable maximized startup

2. Scope policy
- change only Tauri window config
- do not modify frontend layout code
- do not add runtime window resize logic

## 3. Scope

### 3.1 Files to update

- `desktop/src-tauri/tauri.conf.json`
- a new config regression test under `desktop/`

### 3.2 Files to keep unchanged

- desktop page layout and shell components
- Tauri Rust runtime code
- app startup routing

## 4. UX Notes

- The first launch should feel like a desktop workbench, not a small web preview.
- A larger minimum size is necessary because the current product intentionally targets large monitors.
- Centering matters because a larger default window feels awkward if it opens anchored to one side.

## 5. Success Criteria

- the default desktop window is `1728x1080`
- the minimum desktop window is `1440x900`
- the window opens centered
- no maximized startup behavior is introduced
