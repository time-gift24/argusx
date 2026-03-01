# Annotation Frontend QA Checklist (2026-03-01)

## Core Flow

- [ ] Plain field annotation: click `case_title` and confirm right panel shows `case_title` in location field.
- [ ] Quill delayed trigger: select text in paragraph field, wait 300ms, confirm right panel switches to rich-text location.
- [ ] Autosave on switch: edit dynamic field payload, switch target, verify previous annotation remains in `draft` state.
- [ ] Duplicate location behavior: click same location twice, verify enters existing annotation edit context (no duplicate draft).
- [ ] Orphaned visibility: after text drift + mismatch, annotation status becomes `orphaned` and is visible in UI/state.

## Data & Rules

- [ ] Rule catalog remote success path uses `remote` source.
- [ ] Rule catalog remote failure path falls back to local catalog.
- [ ] Required dynamic fields block submit until valid.

## Release Gate

- [ ] `pnpm lint`
- [ ] `pnpm test`
- [ ] `pnpm build`
