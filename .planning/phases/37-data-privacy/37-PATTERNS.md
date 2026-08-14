# Phase 37: Приватность данных — Pattern Map

**Mapped:** 2026-08-14
**Files analyzed:** 10 (new/modified/deleted)
**Analogs found:** 7 exact-or-role-match / 10 (3 have no direct analog — noted below with the
closest structural relative)

> Privacy note: this file quotes only file paths, code shapes, and CLAUDE.md-approved
> placeholders (`org.name`, «Иванов И.И.», `example.local`). No real requisite, ФИО, or
> infrastructure identifier appears below.

## Two concrete placement questions (answered first, they gate everything else)

### Q1: Do `scripts/` (root) and `ui/scripts/` differ in convention?

**Yes, in one load-bearing way: the executable bit, tied to how each is invoked.**

| Property | `scripts/check-privacy-requisites.sh` (root) | `ui/scripts/*.mjs` |
|---|---|---|
| Shebang | `#!/usr/bin/env bash` | `#!/usr/bin/env node` |
| File mode | `-rwxr-xr-x` (executable) | `-rw-r--r--` (NOT executable) |
| Invocation | `./scripts/check-privacy-requisites.sh` (direct exec, shebang used) | `node scripts/check-contrast.mjs` (invoked via `node`, shebang is decorative) |
| Module system | n/a (bash) | ESM (`.mjs`, `import`/no `require`) |
| Where called from | `ci-fast.yml` step `run:` | `ui/package.json` `scripts.lint` chain |

**Implication for the new gate (D-09: root `scripts/`, `.mjs`):** `scripts/check-privacy.mjs`
should follow the **content convention of `ui/scripts/*.mjs`** (ESM, zero-dependency,
`node:fs`/`node:path`/`node:url`, header-comment style, `argv[2]`/flag-based self-test hook)
but the **invocation convention of root `scripts/`**: it will be called directly from two
places that are NOT `node scripts/...` — `.githooks/pre-commit` (a shell entry point that
itself needs the executable bit, since git invokes hooks directly) and `ci-fast.yml`'s
`run: node scripts/check-privacy.mjs` (explicit `node`, no exec bit needed on the `.mjs`
itself, matching the `ui/scripts/*.mjs` mode-644 pattern — do not `chmod +x` it). Give the
executable bit only to the two shell entry points that are exec'd directly:
`scripts/setup-hooks.sh` and `.githooks/pre-commit`. The `.mjs` files stay `644`, invoked
via `node ...` everywhere, consistent with every existing `ui/scripts/*.mjs`.

### Q2: Where do fixtures live today — is root `tests/fixtures/privacy/` consistent?

**No existing root-level `tests/` convention for Node exists in this repo — inventing one is
a real deviation, not a copy.** Confirmed: `git ls-files | grep '^tests/'` is empty; the only
fixture convention in the repository is **colocated with the Rust test crate that consumes
them**: `crates/trackly-app/tests/fixtures/` (flat files: `act_42.json`, `act_42.sha256`,
`logo_test.png`, `logo_test_with_script.svg`, plus a `devices/` subdirectory with its own
CSV fixtures and a `devices/README.md` explaining each file's purpose). Rust tests reference
them with `include_bytes!("fixtures/...")` / `include_str!("fixtures/...")` — relative to the
test file, not an absolute repo-root path.

There is a same-purpose analog already in that directory worth citing directly:
`crates/trackly-app/tests/fixtures/act_42.sha256` is a bare SHA-256 hash file (no comments, no
class label, single line, no trailing newline) — proof the repo already commits raw `.sha256`
fixture files, but in a different **format** than D-07's `<sha256> <class>`-per-line format
with `#`-comments. Do not copy that bare format for `privacy-tokens.sha256` or its test
fixture — D-07 is locked and richer (needs the class label and a recovery header).

**Recommendation for the planner:** `tests/fixtures/privacy/` at repo root is **not** what the
existing convention would produce by extension — the existing pattern would colocate fixtures
next to the consumer, i.e. `scripts/fixtures/privacy/` (sibling to `scripts/check-privacy.mjs`
and `scripts/check-privacy.selftest.mjs`), the same way `crates/trackly-app/tests/fixtures/`
sits next to `crates/trackly-app/tests/*.rs`. This is **not locked** by CONTEXT.md — RESEARCH.md
proposes `tests/fixtures/...` paths only as illustrative Wave-0-gap examples, not a decision.
Flagging both options for the planner to pick explicitly rather than silently importing an
unprecedented root `tests/` directory: (a) `scripts/fixtures/privacy/` — consistent with the
colocation precedent; (b) `tests/fixtures/privacy/` — matches RESEARCH.md's illustrative naming
verbatim but introduces a new root-level convention with no other consumer. Either way, mirror
`devices/README.md`'s convention of a short README inside the fixtures directory explaining
what each fixture is for (doubly valuable here since fixture content must stay fictional).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `scripts/check-privacy.mjs` | utility (CLI gate) | batch/transform (scans file set, tokenizes, compares hashes) | `ui/scripts/check-print-isolation.mjs` | exact (form: zero-dep ESM durable structural gate, self-test-by-argument, explanatory header) |
| `scripts/check-privacy.selftest.mjs` | test (self-test runner) | batch | *(no direct file analog — see below)* | partial — closest relative is the `argv[2]`-driven self-test convention embedded in `check-print-isolation.mjs`'s own `main()`, not a separate file |
| `scripts/privacy-tokens.sha256` | config/data (committed constant) | n/a (static data, read by gate) | `scripts/check-privacy-requisites.sh`'s `ALLOWED` array (closest *content* analog: explained, versioned allowlist) + `crates/trackly-app/tests/fixtures/act_42.sha256` (closest *format-family* analog: repo already commits raw `.sha256` files, different shape) | role-match |
| `scripts/setup-hooks.sh` | utility (one-shot installer script) | n/a | `scripts/check-privacy-requisites.sh` | role-match (shebang, header-comment-explaining-why, `set -euo pipefail`, executable bit) |
| `.githooks/pre-commit` | hook entry point | request-response (git invokes synchronously, blocks commit on nonzero exit) | `scripts/check-privacy-requisites.sh` (as the only existing directly-exec'd, executable-bit script in the repo) | partial — no existing `.githooks/` precedent, but exec-bit + shebang + fail-closed convention transfers directly |
| `CONTRIBUTING.md` | doc | n/a | `README.md` | partial — deliberate audience split, not a content template (see below) |
| `tests/fixtures/privacy/*` (or `scripts/fixtures/privacy/*`, see Q2) | test fixture | batch | `crates/trackly-app/tests/fixtures/` | role-match (naming/placement conventions; see Q2 for path caveat) |
| `.github/workflows/ci-fast.yml` (modified) | CI config | n/a | itself (existing `Privacy gate` step) | exact — same file, step relocated + command swapped |
| `.gitignore` (modified) | config | n/a | itself (existing commented blocks) | exact — same file, new commented block + negation |
| `scripts/check-privacy-requisites.sh` (deleted) | *(source, not target)* | — | — | — (logic absorbed into mode 1 of the new gate, see Pattern Assignments) |

## Pattern Assignments

### `scripts/check-privacy.mjs` (utility, batch/transform)

**Primary analog:** `ui/scripts/check-print-isolation.mjs`
**Secondary analogs:** `ui/scripts/check-contrast.mjs`, `ui/scripts/check-pagedjs-csp-hash.mjs`
**Absorbed-logic source:** `scripts/check-privacy-requisites.sh`

**Header-comment convention** (copy the shape, not the content) — from
`ui/scripts/check-print-isolation.mjs` lines 1-39:
```javascript
#!/usr/bin/env node
// [check-print-isolation] Постоянный гейт против регрессий LAN-печати в
// `PdfPreviewModal.svelte` → `printViaTopLevel()`.
//
// Почему он существует: ...
//
// Гейт СТРУКТУРНЫЙ: он читает исходник и проверяет, что инварианты (а не
// конкретные байты) на месте. ...
//
// Проверяемые инварианты:
//   INV-1a (260805-ifj + 260805-jwf) — ...
//   ...
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-print-isolation.mjs                 # проверить репозиторий
//   node scripts/check-print-isolation.mjs <path.svelte>   # проверить копию (самотест гейта)
```
Every gate in this project opens with: `[gate-name]` tag, "почему он существует" prose tying
the check to the defect(s) it prevents, a bulleted list of the named invariants it checks, an
explicit "Zero-dependency: только node:X/Y/Z" line, and a `Usage:` block showing every CLI
form. `RESEARCH.md`'s own drafted header for `check-privacy.mjs` already follows this shape —
copy it verbatim as the starting header, it is not decorative, it is the project's convention.

**Imports pattern** (from `check-print-isolation.mjs` lines 40-47, `check-pagedjs-csp-hash.mjs`
lines 21-24):
```javascript
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
// add for check-privacy.mjs specifically:
import crypto from 'node:crypto';          // per check-pagedjs-csp-hash.mjs's usage
import { execFileSync } from 'node:child_process';  // new — no existing gate shells out yet

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');   // note: repo ROOT, not UI_ROOT —
                                                     // check-print-isolation.mjs resolves to
                                                     // ui/, this gate must resolve to the repo
                                                     // root since it scans *.rs/*.md/*.html too
```
`execFileSync` for `git show`/`git diff` plumbing has no precedent in `ui/scripts/` — every
existing gate there only reads static files off disk. This is new territory for the project's
gate convention; RESEARCH.md's Anti-Patterns section already flags the correct/incorrect forms
(array-args `execFileSync`, never string-interpolated `exec()`).

**Self-test-by-argument convention** (from `check-print-isolation.mjs` lines 486-499):
```javascript
function main() {
  const arg = process.argv[2];
  const target = arg ? path.resolve(process.cwd(), arg) : DEFAULT_TARGET;

  let source;
  try {
    source = fs.readFileSync(target, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${target}`);
    process.exit(1);
  }
  ...
}
```
`check-privacy.mjs` needs the analogous shape but with `--hashes <path>` (per C-01, so the
selftest never touches the production hash file) instead of a positional target — the target
*set* is computed by mode (`--staged` vs default HEAD scan), only the hash-file path is
swappable.

**Read-fail → exit 1 pattern** (`check-print-isolation.mjs` lines 490-496,
`check-contrast.mjs` lines 126-129) — this is the direct precedent for R7's fail-closed
requirement:
```javascript
const content = readFileSafe(tokensScssPath);
if (content == null) {
  console.error(`[check-contrast] cannot read ${relPath(tokensScssPath)}`);
  process.exit(1);
}
```
Both existing gates already treat "file I expected to read is missing/unreadable" as a hard
`exit 1`, not a soft skip — this is the same posture D-11/R7 ask for, just applied to
`--hashes` instead of a fixed source file. Copy this reflex directly; do not invent a new one.

**Flag-parsing convention** (`check-pagedjs-csp-hash.mjs` line 87, `check-tokens.mjs` uses a
slightly richer `parseArgs`):
```javascript
const printOnly = process.argv.includes('--print');
```
For a single boolean flag, `process.argv.includes('--flag')` is the established idiom in this
project — no `yargs`/`commander`, hand-rolled is intentional (D-09: zero-dependency). For
`--hashes <path>` (needs a value, not just a boolean), `check-tokens.mjs` has the closest
value-flag precedent — read it if the planner needs `--src=<dir>`-style `key=value` parsing;
not excerpted here since `--hashes <path>`-with-space is simpler than that file's `=`-form.

**Exit-code / message discipline** (`check-print-isolation.mjs` lines 501-516):
```javascript
for (const v of violations) {
  console.error(`${TAG} ${label} — ${v.inv} (регресс быстрофикса ${v.fixId}): ${v.message}`);
  console.error(`${TAG}   ${v.hint}`);
}

if (violations.length > 0) {
  console.error(`${TAG} FAIL — ${violations.length} нарушений ...`);
  process.exit(1);
}

console.error(`${TAG} PASS — 0 нарушений`);
process.exit(0);
```
Every gate: violations printed to **stderr** (not stdout — stdout is reserved for
machine-readable output like `check-pagedjs-csp-hash.mjs --print`'s hash), one line of
location+what, one indented line of why-it-matters/what-to-do, a final `FAIL — N нарушений`
summary line, `process.exit(1)`; on success a `PASS — 0 нарушений` line and `process.exit(0)`.
**D-16 modifies this for `check-privacy.mjs` specifically: the "what" line must say
`путь:строка — маркер класса X` and must NOT include the matched value or the raw line
content** — see Anti-Patterns below, this is the one place the new gate must diverge from
every analog it's copying form from.

**Two-mode-in-one-file separation** — RESEARCH.md's own Anti-Patterns section already
prescribes keeping mode 1 (allowlist, absorbed from the bash script) and mode 2 (n-gram hash)
as two independent functions with independent target-file lists, not a merged per-file loop.
No repo analog demonstrates two-mode gates today (every existing `ui/scripts/*.mjs` gate is
single-purpose) — this is new structure, but the file-header convention above (bulleted
"Режимы:" list) already gives it a documented seam.

---

### `scripts/check-privacy.selftest.mjs` (test, batch)

**No direct file analog exists in this repo.** Every existing gate is *itself* the test (run
it, check the exit code) — there is no separate `*.selftest.mjs` or `*.test.mjs` file anywhere
under `ui/scripts/`. The closest structural relative is the `argv[2]`-driven self-test
mechanism embedded inside `check-print-isolation.mjs`'s own `main()` (see excerpt above) —
that file *can* be pointed at an arbitrary `.svelte` copy for testing, but the test-calling
code lives outside the repo (documented only as "самотест гейта" in the header comment, not
committed as a script).

Since CONTEXT (C-01) explicitly requires the selftest to never touch the production hash
file, and RESEARCH.md's Validation Architecture section already sketches the shape (a set of
`node scripts/check-privacy.mjs --hashes tests/fixtures/... <fixture-target>` invocations with
asserted exit codes), the closest transferable convention is: **write
`check-privacy.selftest.mjs` as a thin Node script that shells out to
`check-privacy.mjs` via `execFileSync` (or imports it as a module and calls its exported
function directly — CONTEXT does not lock which), asserts exit codes per RESEARCH.md's fixture
table, and itself follows the same header-comment + `[gate-name]` + `PASS`/`FAIL` + exit-code
convention as every other gate** — i.e., the selftest file is *also* a gate in this project's
existing structural-gate idiom, just one whose subject is another gate's behavior rather than
source code.

**Zero-value-in-output requirement is stricter here than anywhere else in the codebase:** the
prompt's own scope note ("asserts... that no token appears in output") has no repo precedent —
every existing gate is free to print whatever it wants because nothing it prints is sensitive.
This selftest is the one script in the entire project where the assertion surface itself
(stdout/stderr of the child process) must be scanned for leakage, not just the exit code. No
analog for this pattern exists; treat it as new and document the reasoning inline in the file
header per this project's convention of never adding an invariant without explaining why.

---

### `scripts/privacy-tokens.sha256` (config/data)

**No code analog — this is a data file.** Closest content analog for the "explained, versioned
list" shape is `scripts/check-privacy-requisites.sh` lines 27-59 (`ALLOWED` array with a header
comment directly above it):
```bash
# Fictional placeholder values approved for fixtures, tests and demo contexts.
# NEVER add a real organization's requisites here.
ALLOWED=(
  # Preview demo context (template_service.rs)
  "..."
  ...
)
```
Copy the *spirit* (a leading comment block explaining what this list is, why entries are safe,
and how to add one) into D-07's mandated header for `privacy-tokens.sha256` — the fail-closed
recovery instructions R7 requires belong in that same header block, textual, above the
`<sha256> <class>` lines.

Closest raw-format analog: `crates/trackly-app/tests/fixtures/act_42.sha256` — a single bare
SHA-256 line, no trailing newline, no comments. **Do not copy this format** — it proves the
repo is comfortable committing `.sha256` files, nothing more; D-07's locked format
(`<sha256> <class>`, `#`-comments, sorted, header with recovery instructions) is richer by
design and this file's minimalism would fail R7's "explains how to recover" requirement.

---

### `scripts/setup-hooks.sh` (utility, one-shot installer)

**Analog:** `scripts/check-privacy-requisites.sh` (only existing root-`scripts/` bash file)

**Shebang + strict-mode + header convention** (lines 1-25):
```bash
#!/usr/bin/env bash
#
# Privacy gate (WR-11 / PRIV-01): organization requisites must never be
# hardcoded into the repository as real values.
#
# ...
#
# Run locally: ./scripts/check-privacy-requisites.sh

set -euo pipefail

cd "$(dirname "$0")/.."
```
`setup-hooks.sh` should open the same way: shebang, a short header explaining what one command
does and why it exists (tie it to D-10's rationale — git does not enable hooks itself), `set
-euo pipefail`, and `cd "$(dirname "$0")/.."` if it needs repo-root-relative behavior (it may
not, since `git config core.hooksPath .githooks` is location-independent — verify before
copying the `cd` line, it may be unnecessary ceremony for a true one-liner). Executable bit
required (`chmod +x`, matches `check-privacy-requisites.sh`'s `-rwxr-xr-x`).

---

### `.githooks/pre-commit` (hook entry point)

**No `.githooks/` precedent exists in the repo** (confirmed: directory absent, `.git/hooks/`
contains only `*.sample` files, `git config core.hooksPath` is unset). Closest transferable
convention is still `scripts/check-privacy-requisites.sh`'s executable-bit + shebang pattern,
since it is the only script in the repo today that is invoked by direct execution rather than
via `node`/`bash` prefix — exactly the invocation model a git hook needs (git executes the
hook file directly, checking only the executable bit and, on the shebang line, the
interpreter).

**Required behavior per D-11 (fail-closed, staged-blob source):**
```javascript
// Pattern sketched in RESEARCH.md's Pattern 1 — hook should call this exact
// shape via execFileSync from check-privacy.mjs, or the hook script itself
// does the staged-diff plumbing and delegates scanning to check-privacy.mjs
// with --staged. Either split is Claude's Discretion; RESEARCH.md's diagram
// shows check-privacy.mjs itself owning both --staged and default (HEAD)
// modes, with .githooks/pre-commit as a one-line wrapper:
```
```bash
#!/usr/bin/env bash
set -euo pipefail
if ! command -v node >/dev/null 2>&1; then
  echo "pre-commit: node not found — cannot run the privacy gate (fail-closed, D-11)." >&2
  echo "Install Node or bypass with 'git commit --no-verify' (NOT recommended)." >&2
  exit 1
fi
exec node "$(git rev-parse --show-toplevel)/scripts/check-privacy.mjs" --staged
```
No existing file in the repo demonstrates the "check for a binary, fail loudly if absent"
idiom — this is new, but the *tone* (explain what to do next, per the project's established
error-message convention) transfers directly from `check-privacy-requisites.sh`'s closing
`cat <<'EOF' ... EOF` block (lines 92-104, see Anti-Patterns for what NOT to copy from it).

---

### `CONTRIBUTING.md` (doc)

**Analog:** `README.md` — but a **deliberate split**, not a shared template. Confirmed by
reading `README.md`'s full section list: `Установка и запуск`, `Portable-режим`, `Требования`,
`Серверный режим`, `Предупреждения безопасности`, `Проверка контрольных сумм`, `Лицензия` —
every section is end-user-facing (install, run, verify checksums). There is **no existing
developer-setup section anywhere in the repo** to absorb into `CONTRIBUTING.md` — confirmed by
grep across `README.md`'s headings; no other root-level doc exists (`find` for
`CONTRIBUTING*` at repo root returns nothing).

**What to copy from README.md:** only the *prose register* (plain, imperative Russian
instructions, numbered steps for install-like sequences, fenced code blocks for exact
commands) — not any content. `CONTRIBUTING.md`'s hook-enabling section should read like
README's `## Portable-режим (Windows)` section in tone: short paragraph of "why", then a
numbered or code-fenced "how":
```markdown
## Portable-режим (Windows)

Portable ZIP содержит `trackly.exe` вместе с пустым файлом `portable.txt`.
Наличие `portable.txt` рядом с исполняемым файлом активирует portable-режим:
...
```
Mirror this for the hook-enabling section: one paragraph explaining `core.hooksPath` is not
automatic (ties to D-10's rationale), then the exact one-line command
(`./scripts/setup-hooks.sh` or the raw `git config core.hooksPath .githooks` it wraps).

---

### `.github/workflows/ci-fast.yml` (modified)

**Analog:** itself. Current step (lines 80-86):
```yaml
      # PRIV-01 (WR-11): the repository is PUBLIC and anything committed stays
      # in git history even after deletion from HEAD, so the durable control is
      # keeping real organization requisites from entering at all. Runs early —
      # it is instant and a privacy violation should not wait on a 20-minute
      # build to be reported.
      - name: Privacy gate (organization requisites)
        run: ./scripts/check-privacy-requisites.sh
```
Per D-12, this step block moves to immediately after `Checkout` (currently lines 19-20) —
**before** `Install Rust toolchain`, `Install Tauri 2 Linux system dependencies`, `pnpm
install`, and the SPA build. Update `run:` to call the new gate (no `setup-node`/`pnpm
install` prerequisite needed — Node is preinstalled on `ubuntu-latest`, confirmed by
Environment Availability in RESEARCH.md). Keep the explanatory comment block — it already
states the exact rationale D-12 restates (instant check, should not wait on a 20-minute
build) — just update the step name/command:
```yaml
      - name: Checkout
        uses: actions/checkout@v4

      # PRIV-01/PRIV-02: repository is PUBLIC ... (keep/extend existing comment)
      - name: Privacy gate
        run: node scripts/check-privacy.mjs
```
`ci-full.yml` — confirmed via `grep -n "check-privacy\|Privacy gate" ci-full.yml` (zero
matches) that no privacy step exists there today; D-12 explicitly says do not add one.

---

### `.gitignore` (modified)

**Analog:** itself. Existing convention is a short comment line, then one or more bare paths
per concern, no blank-line-per-entry (see e.g. lines 8-17, the Node/pnpm block):
```gitignore
# Node / pnpm
node_modules/
ui/node_modules/
ui/dist/
.vite/
.svelte-kit/
pnpm-debug.log*
npm-debug.log*
yarn-debug.log*
yarn-error.log*
```
For D-04 (ignore all of `.planning/reference/`, keep `design-system-v2/` tracked — the
CONTEXT amendment noted in the phase scope), the negation form has **no existing precedent in
this file** — every current entry is a plain ignore, none use `!`. Standard gitignore negation
syntax applies (`!` must appear in a **later** line than the broader ignore, and the parent
directory of the negated path must not itself be ignored by a directory-pattern — trailing
`/`-only ignores block negation of nested paths; verify the exact form works with
`git check-ignore -v` before committing, this is the one place `.gitignore` semantics are
easy to get subtly wrong):
```gitignore
# .planning/reference/ holds local-only source samples (D-04); everything
# under it is ignored except design-system-v2/, which predates this rule
# and is intentionally tracked.
.planning/reference/*
!.planning/reference/design-system-v2/
```
(Wildcard-then-negate `reference/*` rather than bare `reference/` is required for the negation
to have any effect — a bare directory ignore blocks git from ever descending into it to find
the negated exception. Confirm with `git check-ignore -v` per Pitfall 3 in RESEARCH.md before
treating this as done.)

---

## Shared Patterns

### Structural-gate header convention
**Source:** `ui/scripts/check-print-isolation.mjs` lines 1-39 (also present in
`check-contrast.mjs` lines 1-22, `check-pagedjs-csp-hash.mjs` lines 1-20, `check-tokens.mjs`
lines 1-19)
**Apply to:** `scripts/check-privacy.mjs`, `scripts/check-privacy.selftest.mjs`
```javascript
#!/usr/bin/env node
// [gate-name] <one-line what>.
//
// Почему он существует: <tie to the defect/risk it prevents>.
//
// Zero-dependency: только node:X/Y/Z.
//
// Usage:
//   node scripts/gate-name.mjs [flags]
```

### Fail-on-unreadable-input reflex
**Source:** `ui/scripts/check-contrast.mjs` lines 126-129, `check-print-isolation.mjs` lines
490-496
**Apply to:** `scripts/check-privacy.mjs`'s `--hashes` handling (R7)
```javascript
const content = readFileSafe(path);
if (content == null) {
  console.error(`[gate-name] cannot read ${relPath}`);
  process.exit(1);
}
```

### stderr-only violation reporting + PASS/FAIL summary line
**Source:** `ui/scripts/check-print-isolation.mjs` lines 501-516,
`ui/scripts/check-contrast.mjs` lines 171-177
**Apply to:** all gate output in `check-privacy.mjs` and `check-privacy.selftest.mjs`
```javascript
console.error(`${TAG} FAIL — ${violations.length} нарушений ...`);
process.exit(1);
// or
console.error(`${TAG} PASS — 0 нарушений`);
process.exit(0);
```

### `execFileSync` with array args, never string-interpolated `exec()`
**Source:** RESEARCH.md Anti-Patterns section (no direct in-repo precedent — this is a
project-wide security posture stated in RESEARCH, not yet demonstrated in an existing file)
**Apply to:** every git-plumbing call in `check-privacy.mjs`
```javascript
execFileSync('git', ['show', `:${relPath}`], { encoding: 'utf8' });  // correct
// NEVER: execSync(`git show :${relPath}`)                            // shell-injectable
```

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `scripts/check-privacy.selftest.mjs` | test | batch | No existing `*.selftest.mjs`/`*.test.mjs` file in the repo — every other gate is tested by manually invoking itself with an argument, not by a committed test-runner script. Build it as a gate-testing-a-gate, following the same header/PASS-FAIL convention as every other `ui/scripts/*.mjs` file (see Shared Patterns) since no closer precedent exists. |
| `.githooks/pre-commit` | hook entry point | request-response | No `.githooks/` directory exists anywhere in the repo's history for this to extend; only the executable-bit + shebang convention from `scripts/check-privacy-requisites.sh` transfers. |
| `scripts/privacy-tokens.sha256` (format) | config/data | n/a | D-07's `<sha256> <class>` + `#`-comment format has no exact precedent — `act_42.sha256` is the closest *file-type* analog but a structurally simpler bare-hash format that must NOT be copied (see Pattern Assignments). |

## Anti-Patterns to Flag (do NOT copy these from the analogs)

1. **Printing the matched value in the failure message.** `scripts/check-privacy-requisites.sh`
   line 87 does exactly this:
   ```bash
   echo "  value: \"${value}\""
   ```
   D-16 explicitly states the new gate does **not** inherit this — CI logs in a public repo are
   world-readable, and printing the matched value would turn the gate itself into a leak
   vector. The new gate's messages must be `путь:строка — маркер класса X` only, for **both**
   mode 1 (absorbed allowlist) and mode 2 (n-gram hash) — the old script's line-91 `value:`
   print is a case that must be dropped during absorption, not preserved for "regression
   parity." C-02's regression requirement is about *detection* (a real `inn`/`ogrn` literal
   still fails the gate), not about *preserving the old message format*.

2. **`.planning/reference/*` bare-directory-ignore without wildcard.** A plain
   `.planning/reference/` line in `.gitignore` (matching the *style* of most existing entries
   in this file, e.g. `node_modules/`) will silently defeat the `!design-system-v2/` negation
   D-04 requires — git will not descend into a directory-ignored path to evaluate exceptions.
   Do not pattern-match the terse directory-only style used elsewhere in this `.gitignore` for
   this one entry; it needs the `reference/*` + `!reference/design-system-v2/` form instead.

3. **Any "disable this one token/line" escape hatch in `check-privacy.mjs`.** D-14/D-13/R9 are
   explicit and locked: exclusions are **path-only** (constants in code, reviewable in diff).
   No analog in the codebase has an inline-suppression comment convention (e.g. no
   `// eslint-disable-next-line`-style opt-out exists in any of the `ui/scripts/*.mjs` gates
   either) — this is consistent with the rest of the project's gates, not a new restriction,
   but worth stating explicitly since it is the single most security-relevant invariant of this
   phase's deliverable.

## Metadata

**Analog search scope:** `ui/scripts/*.mjs` (6 files, 4 read in full), `scripts/` (root, 1 file
read in full), `.github/workflows/ci-fast.yml` + `ci-full.yml` (read/grepped),
`crates/trackly-app/tests/fixtures/` (directory listing + one file content + three test-file
references), `README.md` (full heading scan), `.gitignore` (read in full), `ui/package.json`
`scripts.lint` chain (read).
**Files scanned:** ~14
**Pattern extraction date:** 2026-08-14
