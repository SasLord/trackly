---
phase: 37-data-privacy
reviewed: 2026-08-18T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - scripts/check-privacy.mjs
  - scripts/check-privacy.selftest.mjs
  - .githooks/pre-commit
  - scripts/setup-hooks.sh
  - .github/workflows/ci-fast.yml
  - crates/trackly-app/src/pdf/renderer.rs
  - .gitignore
  - scripts/privacy-tokens.sha256
  - scripts/fixtures/privacy/README.md
  - scripts/fixtures/privacy/allowlist-regression.rs.txt
  - scripts/fixtures/privacy/binary-regression.docx
  - scripts/fixtures/privacy/empty.sha256
  - scripts/fixtures/privacy/tokens.fixture.sha256
  - scripts/fixtures/privacy/with-marker.md
  - scripts/fixtures/privacy/without-marker.md
  - scripts/check-privacy-requisites.sh (deleted — confirmed clean removal, no dangling references)
findings:
  critical: 2
  warning: 5
  info: 3
  total: 10
status: issues_found
---

# Phase 37: Code Review Report

**Reviewed:** 2026-08-18
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

The unified privacy gate (`scripts/check-privacy.mjs`) is well-structured, honors the "never
print a matched value" contract (D-16), uses `execFileSync` with array arguments everywhere
(no shell injection surface), and the bash entry points (`.githooks/pre-commit`,
`scripts/setup-hooks.sh`) are correctly quoted with `set -euo pipefail`. Mode-1's `ALLOWED`
list and pattern were verified byte-for-byte equivalent to the deleted
`scripts/check-privacy-requisites.sh` (the "ported verbatim" claim in 37-03-SUMMARY.md holds).
The old script printed the offending value on failure (`echo "  value: \"${value}\""`) — the
new gate's D-16 discipline is a genuine, verified improvement. `.gitignore`'s negation pair
for `.planning/reference/design-system-v2/` was verified to work correctly with
`git check-ignore`.

However, two **live-reproduced, Critical** detection/fail-open bugs undermine the gate's core
purpose, and five further **Warning**-level gaps weaken its coverage. All reproductions below
used exclusively fictional, throwaway data in the session scratchpad (never committed, never
printed as plaintext beyond the illustrative snippets already used by this phase's own
fixtures) and are directly reproducible against the code as currently written.

Everything below was verified by running the actual `scripts/check-privacy.mjs` binary (not
just static reading) against synthetic inputs, using `--hashes` pointed at either a scratch
hash file or the real `scripts/privacy-tokens.sha256` as noted per finding.

## Critical Issues

### CR-01: `--add` and scan-time tokenization disagree — many real multi-word/compound tokens become permanently unmatchable

**File:** `scripts/check-privacy.mjs:472-490` (hash computed for `--add`) vs. `scripts/check-privacy.mjs:114-124` + `310-328` (hash computed during scan)

**Issue:** `runAdd()` hashes the operator's raw typed string directly:
```js
const normalized = normalize(value);   // line 478 — normalize() only lowercases + ё→е + NFC
const hash = sha256Hex(normalized);
```
But `scanHashes()` never hashes a raw line — it always decomposes the line into `\p{L}\p{N}`-only
word runs via `WORD_RE`, then rebuilds 1–3-word n-grams by **joining tokens with a single ASCII
space** (`extractNgrams`, line 120: `words.slice(i, i + n).join(' ')`), and only *that*
reconstructed string is normalized and hashed.

These two code paths only produce the same hash when the added value (a) contains **exactly**
single-ASCII-space-separated word tokens, with no other punctuation between them, and (b)
tokenizes to **3 words or fewer** (the scan-time n-gram window never exceeds `n=3`, line 118:
`for (let n = 1; n <= 3; n++)`). Any value that has a hyphen, a period, a non-breaking space, or
more than 3 word-tokens produces a hash at `--add` time that can **never** be produced again by
the scanner — the entry becomes a permanently dead, silently-unmatchable line in the trusted
hash file. The operator who ran `--add` and got "PASS — токен ... добавлен" has no way to know
the entry will never fire.

Live reproduction (fictional data only, run against the real gate binary):
```
$ node -e '...sha256Hex(normalize("Образцов-Показательный"))...' > repro-hashes.sha256   # simulates --add exactly
$ echo "Акт подписан представителем Образцов-Показательный по доверенности." > repro-doc.md
$ node scripts/check-privacy.mjs --hashes repro-hashes.sha256 repro-doc.md
[check-privacy] PASS — 0 нарушений        # should have been a violation — the literal string is present verbatim
```
Second reproduction, a 5-word fictional address-shaped value (illustrating the >3-word-window
gap):
```
$ node -e '...sha256Hex(normalize("проспект Образцовый дом двенадцать строение три"))...' > repro-hashes2.sha256
$ echo "Адрес доставки: проспект Образцовый дом двенадцать строение три, офис 5." > repro-doc2.md
$ node scripts/check-privacy.mjs --hashes repro-hashes2.sha256 repro-doc2.md
[check-privacy] PASS — 0 нарушений        # again should have failed
```
This is directly relevant to the project's actual threat model: Russian ФИО frequently use
hyphenated double surnames (Иванова-Петровская-style), and addresses/organization strings
routinely exceed 3 words. The self-test suite does not catch this because its only fixture
(`Пчёлкин Артём`) is a clean 2-word, single-space-separated case that happens to satisfy both
constraints — see the fixture's own text in `scripts/fixtures/privacy/with-marker.md`.

**Fix:** Make `runAdd()` derive its hash through the *same* tokenization function the scanner
uses, and reject inputs that cannot round-trip:
```js
const words = [...value.matchAll(WORD_RE)].map((m) => m[0]);
if (words.length === 0 || words.length > 3) {
  console.error(
    `${TAG} FAIL — значение должно токенизироваться в 1–3 слова (получено: ${words.length}); ` +
    `значения с пунктуацией/более 3 слов никогда не совпадут при сканировании (n-грамм окно ≤3).`,
  );
  process.exit(1);
}
const canonical = words.join(' ');       // exact same join scanHashes() will produce
const normalized = normalize(canonical);
const hash = sha256Hex(normalized);
```
For values that genuinely need >3 words or internal punctuation to be meaningfully unique,
either raise the n-gram window and re-derive existing hashes, or explicitly document that
`--add` only protects the largest ≤3-word substring and require the operator to add each
qualifying 2-/3-word sub-phrase separately (mirroring exactly what the scanner will generate).

---

### CR-02: Unreadable or oversized scan targets are silently skipped — zero diagnostic, exit 0

**File:** `scripts/check-privacy.mjs:230-239` (`collectStagedTargets`) and `scripts/check-privacy.mjs:257-262` (`collectHeadTargets`)

**Issue:** Both target-collection functions wrap the content read in a bare `try { ... } catch { continue; }`:
```js
try {
  content = execFileSync('git', ['show', `:${relPath}`], {
    cwd: REPO_ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024,
  });
} catch {
  continue;                      // <-- no console.error, no counter, no exit-code effect
}
```
```js
try {
  content = fs.readFileSync(abs, 'utf8');
} catch {
  continue;                      // <-- same: silent
}
```
*Any* error — a permission-denied file, a staged blob exceeding the hardcoded 64 MB
`maxBuffer`, a transient git failure, a broken symlink — causes that file to be dropped from
the scan set with **no message anywhere**, and the run still ends with `PASS — 0 нарушений`
and exit 0. This directly contradicts the gate's own stated fail-closed philosophy (R7's
comment block explicitly says "Пустой/отсутствующий файл хэшей намеренно приводит к отказу...
а не к молчаливому пропуску проверки" — but that discipline is not applied to the *content*
read path at all) and it contradicts `ci-fast.yml`'s own comment claiming this step "is the
check that cannot be skipped" (line 28 — a single unreadable file silently *is* skipped, and
nothing about the run indicates this happened).

Live reproduction, using a fictional-but-unlisted `inn` literal in an unreadable file, scanned
against the real production hash list:
```
$ echo 'inn: "9998887771",' > blocked.rs && chmod 000 blocked.rs
$ node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 blocked.rs
[check-privacy] PASS — 0 нарушений        # file was never scanned, no warning printed
```
The `maxBuffer: 64 * 1024 * 1024` on the `git show` call is a second, distinct trigger for the
same silent-skip path: a staged file exceeding 64 MB — precisely the kind of accidental large
export/dump this gate exists to catch — throws `ERR_CHILD_PROCESS_STDOUT_MAXBUFFER`, which is
swallowed the same way.

**Fix:** Never silently drop a target. At minimum, log loudly on every content-read failure and
propagate that as a scan-level failure:
```js
} catch (err) {
  console.error(`${TAG} FAIL — не удалось прочитать содержимое файла для сканирования (пропуск был бы небезопасен): ${relPath} (${err.code || err.message})`);
  unreadableTargets.push(relPath);
  continue;
}
// ... after collecting all targets:
if (unreadableTargets.length > 0) {
  console.error(`${TAG} FAIL — ${unreadableTargets.length} файлов не удалось прочитать для сканирования — гейт не может гарантировать отсутствие нарушений (fail-closed)`);
  process.exit(1);
}
```
This preserves fail-closed semantics end-to-end instead of only at the `--hashes`-loading step.

## Warnings

### WR-01: Mode 1 (allowlist) only scans `.rs`/`.html`-shaped files — new real requisite literals in any other file type are invisible to it

**File:** `scripts/check-privacy.mjs:99` (`REQUISITE_FILE_RE`), `scripts/check-privacy.mjs:290-308` (`scanAllowlist`)

**Issue:** `scanAllowlist` early-returns for any path that doesn't match
`/\.(rs|html)(\.|$)/`. A JSON/TOML/YAML/Svelte/TS config or fixture file carrying an unlisted
`inn`/`kpp`/`okpo`/`ogrn`/`phone`/`fax` literal is entirely untested by mode 1, and mode 2
(hash-based) can only ever catch it if the *exact* value was already proactively hashed via
`--add` — a chicken-and-egg gap for genuinely new real data. Live reproduction, fictional
literal in a `.json` file, scanned against the real production hash list:
```
$ echo '{ "inn": "9998887774", "kpp": "999888777" }' > org.json
$ node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 org.json
[check-privacy] PASS — 0 нарушений
```
**Fix:** Either broaden `REQUISITE_FILE_RE` to cover the other structured-data file types the
project actually uses for config/fixtures (`.json`, `.toml`, `.yaml`, `.svelte`, `.ts`), or
document explicitly (in the header comment and in `CONTRIBUTING.md`) that mode 1's coverage is
intentionally `.rs`/`.html`-only and that other file types rely solely on mode 2's
already-known-value coverage.

### WR-02: `REQUISITE_PATTERN`'s key match is case-sensitive

**File:** `scripts/check-privacy.mjs:90-91`

**Issue:** `(inn|kpp|okpo|ogrn|phone|fax)` has no `i` flag. `INN:`/`Inn:`-cased keys (plausible
in generated/templated JSON, or copy-pasted from external sources) bypass mode 1 entirely.
Live reproduction, combined with WR-03 in one file, scanned against the real production hash
list:
```
$ cat > unquoted.rs <<'EOF'
struct FakeOrg {
    inn: 9998887772,
    INN: "9998887773",
}
EOF
$ node scripts/check-privacy.mjs --hashes scripts/privacy-tokens.sha256 unquoted.rs
[check-privacy] PASS — 0 нарушений
```
**Fix:** Add the `i` flag to `REQUISITE_PATTERN` (and re-verify the 23-literal `ALLOWED`
round-trip still holds — case-insensitive matching only widens detection, it cannot introduce
false negatives against the existing allowlist).

### WR-03: `REQUISITE_PATTERN` requires the value to be double-quoted — unquoted numeric literals bypass mode 1

**File:** `scripts/check-privacy.mjs:90-91`

**Issue:** The value group is `"([^"]*)"` — a bare/unquoted requisite value (e.g. a Rust field
typed as `i64`/`u64` rather than `String`, written as `inn: 9998887772,`) never matches the
pattern at all, so it is never checked against `ALLOWED`. See the same live reproduction as
WR-02 above (the unquoted `inn: 9998887772,` line). This gap is inherited verbatim from the
deleted bash script, but is now exposed to more of the codebase now that this is the sole
durable enforcement point.

**Fix:** Extend `REQUISITE_PATTERN` (or add a second alternative branch) to also match a bare
numeric/bareword value, e.g. `"?([^",}\s][^,}\n]*?)"?\s*[,}\n]` after the colon, and thread the
captured value through the same `ALLOWED_SET` check.

### WR-04: `AUTO_SCAN_EXCLUDED_PREFIXES` permanently exempts `scripts/fixtures/privacy/` from both enforcement points

**File:** `scripts/check-privacy.mjs:186-190`, `547-555`

**Issue:** Confirmed the mode-awareness is implemented correctly (the filter is skipped when
`args.files.length > 0`, so `check-privacy.selftest.mjs`'s explicit positional invocations still
see the fixtures' deliberate violations — verified by re-running the self-test, all 6
assertions still pass). But for the two modes that actually run in production
(`--staged` in the pre-commit hook, full-HEAD in `ci-fast.yml`), this directory is *permanently*
invisible to both the local hook and the CI backstop — the two enforcement points this whole
phase exists to build. This is a smaller, narrowly-scoped, already-self-documented residual
risk (37-04-SUMMARY.md's deviation #1 flags it explicitly), but it is worth recording formally:
if real data is ever placed under `scripts/fixtures/privacy/` (accidental copy-paste, a future
contributor unaware of the convention), neither backstop will ever fire on it, forever.

**Fix (optional hardening, not required to ship):** Add a narrow, separate CI/self-test
assertion that diffs the *set* of files under `scripts/fixtures/privacy/` against an expected
fixed list (or an expected file count + `git log` review requirement), so any addition to that
directory is forced through explicit review even though the content itself is exempt from the
privacy scan proper.

### WR-05: `promptHidden`'s control-character detection only inspects the first byte of a stdin `data` chunk, but the printable branch appends the whole chunk

**File:** `scripts/check-privacy.mjs:398-417`

**Issue:**
```js
function onData(chunk) {
  const char = chunk.toString();       // may be MULTIPLE characters in one event (paste, fast typing)
  const code = char.charCodeAt(0);     // only inspects the FIRST character
  if (code === 13 || code === 10 || code === 4) { /* submit */ }
  else if (code === 3) { /* Ctrl+C, exit */ }
  else if (code === 127 || code === 8) { /* backspace ONE char */ }
  else { input += char; }              // appends the ENTIRE multi-char chunk
}
```
If a terminal delivers more than one character per `data` event (e.g. bracketed paste, or a
burst of buffered keystrokes) and the *first* character of that chunk happens to be Enter,
Ctrl+D, Ctrl+C, or Backspace, the rest of the chunk's content is silently dropped (submit/exit
branches) or misapplied as a single-character backspace, instead of being processed
character-by-character. Conversely, in the normal (non-control) branch, a multi-character
pasted chunk is appended as one atomic string without control-character stripping — if the
pasted content itself embeds a stray control byte after the first character, it becomes part of
the hashed value verbatim. This directly affects the integrity of what gets written into the
trusted `scripts/privacy-tokens.sha256` list, since `--add` is the only production write path
for that file.

**Fix:** Iterate over the chunk's individual characters instead of only inspecting index 0:
```js
function onData(chunk) {
  for (const char of chunk.toString()) {
    const code = char.charCodeAt(0);
    if (code === 13 || code === 10 || code === 4) { cleanup(); process.stdout.write('\n'); resolve(input); return; }
    if (code === 3) { cleanup(); process.stdout.write('\n'); process.exit(1); }
    else if (code === 127 || code === 8) { input = input.slice(0, -1); }
    else { input += char; }
  }
}
```

## Info

### IN-01: Stale comment in `allowlist-regression.rs.txt` describes pre-widening `REQUISITE_FILE_RE` behavior

**File:** `scripts/fixtures/privacy/allowlist-regression.rs.txt:1-8`

**Issue:** The fixture's header comment states: "this file's own name ends in `.rs.txt`, so the
`\.(rs|html)$` filter in check-privacy.mjs deliberately does NOT match this fixture by design."
That was true of the *original* filter before 37-03's own documented widening
(`\.(rs|html)$` → `\.(rs|html)(\.|$)`, per 37-03-SUMMARY.md deviation #1). The current
`REQUISITE_FILE_RE` **does** match `.rs.txt`-shaped paths (verified: `/\.(rs|html)(\.|$)/.test('allowlist-regression.rs.txt')` → `true`). The comment was not updated after the same task
widened the regex, and now misleads a future reader about why the fixture needs
positional-argument invocation (the real reason is `AUTO_SCAN_EXCLUDED_PREFIXES`, not the file
extension filter).

**Fix:** Update the comment to reflect current behavior — the file *does* match
`REQUISITE_FILE_RE`; what actually requires positional-argument invocation in the self-test is
`AUTO_SCAN_EXCLUDED_PREFIXES` excluding `scripts/fixtures/privacy/` from auto-discovery once
committed.

### IN-02: Conflicting-class duplicate hash entries silently resolved by Map insertion order

**File:** `scripts/check-privacy.mjs:358-374` (`loadHashes`), `scripts/check-privacy.mjs:422-441` (`readExistingHashLines`)

**Issue:** `readExistingHashLines`/`runAdd` store hash lines in a `Set` keyed by the full
`"<hash> <class>"` string, so adding the same value twice under two different classes (e.g. `B`
then later `C`) keeps *both* lines in the file rather than replacing the first. `loadHashes`
then builds a `Map<hash, class>` by iterating the file top-to-bottom — since the file is kept
sorted, whichever class letter sorts later for that hash silently wins, with no warning to the
operator that a duplicate/conflicting classification exists for the same underlying value.

**Fix:** In `runAdd`, check whether the computed hash already has an entry under a *different*
class before appending, and warn/prompt for confirmation rather than silently allowing both
lines to coexist.

### IN-03: No pinned Node version for the CI privacy step

**File:** `.github/workflows/ci-fast.yml:19-31`

**Issue:** The privacy gate step runs immediately after `actions/checkout@v4`, before any
`actions/setup-node` or explicit toolchain pin — it relies on whatever Node version
`ubuntu-latest`'s hosted image ships by default. This works today (the script only needs modern
`\p{L}`/`\p{N}` Unicode property escape support, long available), but is an unpinned dependency
on runner-image defaults that could silently drift.

**Fix:** Optional hardening — add an explicit `actions/setup-node@v4` step with a pinned major
version before the privacy gate step, for the same reproducibility reason the Rust toolchain
step two lines below is pinned to `1.88`.

---

_Reviewed: 2026-08-18_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
