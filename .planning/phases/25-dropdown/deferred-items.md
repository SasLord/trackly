# Deferred Items — Phase 25 (dropdown)

## Plan 25-04

- ~~**`ui/src/lib/components/Dropdown.svelte` fails `prettier --check`**~~ — **RESOLVED** at the
  wave-2 gate (`e63ce46`).

  Original note: pre-existing formatting drift, confirmed present at `HEAD~2` (before this
  plan's commits, introduced in Plan 25-03 `816b3fb`). Not touched by Plan 25-04's tasks
  (TableSection.svelte / ShowcasePage.svelte). Out of scope per executor scope-boundary rule.

  Orchestrator correction: a bisect against `9a411b1` and `816b3fb` shows the drift actually
  originated in **Plan 25-02** (the commit that added the portal-wired panel), not 25-03. It
  was "pre-existing" only relative to 25-04's own commits — the file was created inside Phase
  25, so this was phase-local drift, not inherited debt. Fixed with `prettier --write` at the
  wave-2 gate rather than carried forward; `pnpm lint` is now clean.
