# Deferred Items — Phase 25 (dropdown)

## Plan 25-04

- **`ui/src/lib/components/Dropdown.svelte` fails `prettier --check`** — pre-existing
  formatting drift, confirmed present at `HEAD~2` (before this plan's commits, introduced in
  Plan 25-03 `816b3fb`). Not touched by Plan 25-04's tasks (TableSection.svelte /
  ShowcasePage.svelte). Out of scope per executor scope-boundary rule — only auto-fix issues
  directly caused by the current task's changes. `pnpm lint` will report this file until a
  future plan runs `prettier --write` on it.
