---
quick_id: 260805-gdz
slug: lan-print-surface-swallowed-error-and-stop-hiding-pagination-container
phase: 260805-gdz
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/acts/PdfPreviewModal.svelte
autonomous: true
requirements: [GDZ-01, GDZ-02]
must_haves:
  truths:
    - "handlePrint's catch block binds the thrown error (no more bare `catch {}`) and logs it via console.error with enough context to identify which print path failed (printViaSystemBrowser/desktop vs printViaTopLevel/LAN) before showing the existing toast — the real exception is never silently discarded again, on either path"
    - "printViaTopLevel's injected printStyle no longer applies `display: none` to #act-print-root while Paged.js's previewer.preview() renders/measures into it — the container remains a real, laid-out box (non-zero getBoundingClientRect) throughout pagination"
    - "#act-print-root is still invisible to a user browsing the app on screen after the fix — achieved via off-screen positioning (position: absolute; left: -100000px), not display:none"
    - "The off-screen positioning is explicitly reset (position: static; left: auto) inside the existing @media print block, so printed/saved-as-PDF output is not pushed off the page"
    - "The @media print rule hiding the rest of the app (body > :not(#act-print-root) { display: none }) is unchanged"
    - "printViaSystemBrowser is byte-for-byte unchanged"
  artifacts:
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "handlePrint logs the caught print error with branch context via console.error"
      contains: "console.error"
    - path: "ui/src/features/acts/PdfPreviewModal.svelte"
      provides: "printViaTopLevel hides #act-print-root off-screen instead of display:none, with an explicit print-time reset"
      contains: "left: -100000px"
  key_links:
    - from: "handlePrint's catch block"
      to: "console.error"
      via: "bound error parameter, logged before the existing pushToast('error', ...) call"
      pattern: "catch \\(.*\\)\\s*\\{[\\s\\S]*console\\.error"
    - from: "printStyle.textContent's base #act-print-root rule"
      to: "the @media print block"
      via: "position reset (position: static; left: auto) that overrides the off-screen base rule only while printing"
      pattern: "@media print[\\s\\S]*position:\\s*static"
---

<objective>
Two related fixes to the LAN-browser print path in `PdfPreviewModal.svelte`, following live Windows
UAT against the LAN server (build 1.3.2, which already contains the earlier
Paged.js-stylesheets-as-object fix — confirmed by a clean Network tab). The print dialog still never
opens; the existing error toast fires but the browser console is completely empty, because
`handlePrint`'s bare `catch { ... }` discards the real exception without binding or logging it.

**CHANGE 1 (primary deliverable — diagnosability):** `handlePrint`'s catch must bind the error and
`console.error` it with enough context to tell which print path failed (`printViaSystemBrowser`
desktop vs `printViaTopLevel` LAN), while still showing the existing toast. This is a permanent
improvement, not a temporary debug probe — an error that only manifests on a remote LAN machine must
never again be invisible.

**CHANGE 2 (a real defect, NOT proven to be the cause of the above — see honesty note below):** In
`printViaTopLevel`, the injected `printStyle` currently sets
`@media screen { #act-print-root { display: none !important; } }` BEFORE
`await previewer.preview(bodyHtml, [...], printRoot)` renders into that very container. Paged.js
measures real DOM geometry (`getBoundingClientRect`) to decide page breaks; inside a `display: none`
subtree every box is 0x0, so pagination cannot work correctly there regardless of platform. This is a
defect on first principles, found by reading the code — it has NOT been confirmed as the root cause
of the UAT failure. An earlier attempt to isolate it in a standalone harness was INCONCLUSIVE (the
control case — a visible container — also hung, so the experiment did not isolate the variable). Fix
it anyway because it is wrong regardless, but do not claim it resolves the UAT report.

Purpose: make the next failure (if this one persists) diagnosable from a single browser console
screenshot instead of requiring another live UAT round-trip, and remove a genuine pagination-geometry
bug that can only ever produce wrong or empty paginated output.

Output: `handlePrint` logs caught print errors with path context; `printViaTopLevel` hides its
pagination container off-screen (preserving real layout) instead of via `display:none`, with an
explicit print-time reset so printed output is not pushed off the page.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- ui/src/features/acts/PdfPreviewModal.svelte — CURRENT (already contains the earlier
     stylesheets-as-object fix from plan 260805-edd). Full file is in scope for the Edit tool;
     this excerpt pins the exact before-text for both changes. -->

Current `handlePrint` (to be changed by Task 1):

    async function handlePrint() {
      if (!ready || htmlContent === null) return;
      try {
        if (isTauri) {
          await printViaSystemBrowser(htmlContent);
        } else {
          await printViaTopLevel(htmlContent);
        }
      } catch {
        pushToast('error', 'Не удалось открыть документ для печати');
      }
    }

Current `printViaTopLevel`'s `printStyle.textContent` assignment and the call immediately after it
(to be changed by Task 2 — only the `printStyle.textContent` template literal changes; the
`previewer.preview(...)` call line, the `printRoot`/`printStyle` DOM setup above it, and the
`afterprint` cleanup are UNCHANGED):

    printStyle.textContent = `
      ${cssText}
      @media print {
        body > :not(#${PRINT_ROOT_ID}) {
          display: none !important;
        }
        #${PRINT_ROOT_ID} {
          display: block !important;
        }
      }
      @media screen {
        #${PRINT_ROOT_ID} {
          display: none !important;
        }
      }
    `;

    // ... cleanup / afterprint listener (UNCHANGED) ...

    const { Previewer } = await import('pagedjs');
    const previewer = new Previewer();
    await previewer.preview(bodyHtml, [{ 'act-preview.css': cssText }], printRoot);

    window.focus();
    window.print();

`PRINT_ROOT_ID` and `PRINT_STYLE_ID` are module-level consts already defined above this function
(`'act-print-root'` / `'act-print-style'`) — do not redefine them.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Bind and log the swallowed print error with path context</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
In `handlePrint` (shown above under "Current handlePrint"):

1. Add a `const printPath = isTauri ? 'printViaSystemBrowser' : 'printViaTopLevel';` line right
   after the existing `if (!ready || htmlContent === null) return;` guard, before the `try` block.
   This captures which branch is about to run — both `printViaSystemBrowser` (desktop/Tauri) and
   `printViaTopLevel` (LAN browser) failures flow through the SAME catch below, so the log line must
   distinguish them.

2. Change `catch {` to `catch (err) {` and add a `console.error(...)` call as the FIRST statement
   inside the catch, before the existing `pushToast(...)` call. The log message must include: a
   `[PdfPreviewModal]` prefix (matching the existing `console.warn` idiom already used by
   `enterDegraded` elsewhere in this file), the literal text `handlePrint failed`, the `printPath`
   value, and the bound `err` itself as a second argument to `console.error` so the browser devtools
   console renders the full stack trace/object (do not stringify `err` — pass it as-is).

3. Do not change the `pushToast('error', 'Не удалось открыть документ для печати')` call itself, its
   message text, or anything inside the `try` block. Do not touch `printViaSystemBrowser` or
   `printViaTopLevel`'s bodies in this task — that is Task 2's job for `printViaTopLevel` only, and
   `printViaSystemBrowser` must remain byte-for-byte unchanged by this entire plan.

Net effect: any future print failure — on either the desktop or LAN-browser path — is visible in the
browser console with a clear branch label, in addition to the existing user-facing toast.
  </action>
  <verify>
    <automated>grep -F "catch (err)" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && grep -F "console.error" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte | grep -F "handlePrint" >/dev/null && echo GREP_OK</automated>
  </verify>
  <done>handlePrint's catch binds `err`, logs it via `console.error('[PdfPreviewModal] handlePrint failed ...', printPath, err)` (or equivalent single-call form carrying the same three pieces of information) before the unchanged pushToast call; printViaSystemBrowser and printViaTopLevel bodies are untouched by this task.</done>
</task>

<task type="auto">
  <name>Task 2: Replace display:none hiding with off-screen positioning in printViaTopLevel's pagination container</name>
  <files>ui/src/features/acts/PdfPreviewModal.svelte</files>
  <action>
In `printViaTopLevel` (shown above under "Current printViaTopLevel's printStyle.textContent"), edit
ONLY the `printStyle.textContent` template literal — no other line in this function changes, and the
`previewer.preview(...)` call, `printRoot`/`printStyle` setup, and `afterprint` cleanup stay exactly
as they are:

1. Remove the trailing `@media screen { #${PRINT_ROOT_ID} { display: none !important; } }` block
   entirely.

2. Add an unconditional (not media-gated) base rule for `#${PRINT_ROOT_ID}` that positions it
   off-screen while preserving real layout/geometry: `position: absolute; left: -100000px; top: 0;`.
   Place this rule BEFORE the `@media print { ... }` block in the template literal (base rules first,
   media overrides after — this ordering is load-bearing: it lets the `@media print` block's reset in
   step 3 win the cascade during printing, since both rules target the same `#${PRINT_ROOT_ID}`
   selector at equal specificity and CSS resolves ties by source order).

3. Inside the existing `@media print { #${PRINT_ROOT_ID} { display: block !important; } }` rule, add
   two more declarations to the same block: `position: static; left: auto;`. This explicitly resets
   the off-screen positioning from step 2 only while printing, so the printed/saved-as-PDF output
   renders in its normal position instead of being pushed off the page. Do not touch the
   `body > :not(#${PRINT_ROOT_ID}) { display: none !important; }` rule inside `@media print` — leave
   it exactly as-is.

Resulting shape of the template literal (illustrative — use the actual `PRINT_ROOT_ID` interpolation,
matching the existing code's style):

    printStyle.textContent = `
      ${cssText}
      #${PRINT_ROOT_ID} {
        position: absolute;
        left: -100000px;
        top: 0;
      }
      @media print {
        body > :not(#${PRINT_ROOT_ID}) {
          display: none !important;
        }
        #${PRINT_ROOT_ID} {
          display: block !important;
          position: static;
          left: auto;
        }
      }
    `;

Also update the block comment directly above this `printStyle.textContent` assignment (the one
starting "Document's own inline styles (incl. @page) + visibility scoping...") to note WHY this
changed: `display: none` zeroes out `getBoundingClientRect` for every box in the hidden subtree, and
`await previewer.preview(...)` (which runs immediately after this assignment) needs real geometry to
paginate `#${PRINT_ROOT_ID}`'s content — off-screen positioning keeps the container out of the visual
viewport without collapsing its layout box. Note explicitly in the comment that this is a defect fix
on first principles, not a confirmed fix for the specific LAN print-dialog failure reported in UAT
(an isolated-harness attempt to prove the causal link was inconclusive).
  </action>
  <verify>
    <automated>grep -F "position: absolute" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && grep -F "left: -100000px" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && grep -F "position: static" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && ! grep -F "@media screen" /Users/madsas/Projects/trackly/ui/src/features/acts/PdfPreviewModal.svelte >/dev/null && echo GREP_OK && pnpm --dir /Users/madsas/Projects/trackly/ui svelte-check && pnpm --dir /Users/madsas/Projects/trackly/ui lint && pnpm --dir /Users/madsas/Projects/trackly/ui build</automated>
  </verify>
  <done>printStyle.textContent no longer contains an `@media screen` block hiding #act-print-root via display:none; #act-print-root is hidden off-screen via `position: absolute; left: -100000px; top: 0;` applied unconditionally, reset to `position: static; left: auto;` inside @media print alongside the existing `display: block !important;`; svelte-check, lint (incl. the CSP hash-drift gate, unaffected since bootstrapScript.js is untouched), and build all pass; printViaSystemBrowser is byte-for-byte unchanged.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Backend-rendered document HTML → top-level document via `printViaTopLevel` | `html` passed into `printViaTopLevel` originates from the backend's own act/report render endpoints (`acts.renderPdf`, `renderAcceptancePdf`, `reports_export_pdf`) — already-trusted, server-generated markup. This plan changes only (a) how a caught print error is logged, and (b) how the pagination container is hidden on-screen. It does not change what HTML is parsed, sourced, or injected, so no new trust boundary is crossed. |
| Caught exception → browser devtools console | `console.error` now surfaces the previously-swallowed `err` object. The error can only originate from code already running in this same trusted first-party module (Paged.js import, DOM injection, `window.print()`), so its content is developer-facing diagnostic text/stack, not attacker-controlled input. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-gdz-01 | Information Disclosure | `handlePrint`'s new `console.error(...)` call | accept | The logged error/stack is visible only in the local user's own browser devtools console (never transmitted anywhere by this change); it originates from this file's own trusted print-path code, not from attacker-controlled input. No PII beyond what an unhandled JS exception already exposes to a user inspecting their own browser. |
| T-gdz-02 | Tampering | `printStyle.textContent`'s off-screen positioning rule for `#act-print-root` | accept | Purely presentational CSS change (display:none to position:absolute+offset, with an explicit print-time reset). The backend-rendered document content injected into the container is unchanged by this plan; only how the container is hidden on-screen changes. |
| T-gdz-03 | Denial of Service | Off-screen container remaining in the DOM between prints | accept | Unchanged from the current design: the existing `afterprint` listener already calls `printRoot.innerHTML = ''` and removes itself — this plan does not touch that cleanup path, only the CSS applied while the container is populated. |
</threat_model>

<verification>
1. `pnpm --dir ui svelte-check` — no new type errors.
2. `pnpm --dir ui lint` — passes, including the CSP hash-drift gate (bootstrapScript.js untouched).
3. `pnpm --dir ui build` — production build succeeds.
4. Manual/human-check (NOT automatable — requires a real LAN browser hitting the axum server, per
   `synthetic_harness_not_verification`): from a real browser at `https://web.cmy.local:8443` (or
   equivalent LAN URL), open a document preview, press «Печать».
   - If it STILL fails: open devtools console BEFORE pressing «Печать» this time — the new
     `console.error` must now show the real exception with a `printViaTopLevel` (or
     `printViaSystemBrowser`) label. Report that error text — it is the actual next lead, since
     Change 2 was never proven to be the cause.
   - If it now succeeds: confirm the native print dialog opens and the paginated output looks
     correct (no content pushed off the page, which would indicate the `@media print` reset in
     Change 2 did not take effect).
   Record the actual observed result in the SUMMARY — do not assume success or failure.
</verification>

<success_criteria>
- `handlePrint`'s catch binds and logs the real error with print-path context via `console.error`,
  in addition to the unchanged user-facing toast — on both the desktop and LAN-browser branches.
- `printViaTopLevel` no longer hides `#act-print-root` via `display: none` at any point during
  `previewer.preview()`'s render/measure pass; it is hidden off-screen instead, with an explicit
  `position: static; left: auto;` reset inside `@media print`.
- `printViaSystemBrowser` and all CSP-hash-locked files are unchanged.
- `svelte-check`, `lint`, and `build` all pass.
- The plan does NOT claim Change 2 fixes the reported LAN print-dialog failure — that remains
  unconfirmed pending the next real LAN-browser test with console logging now available (see
  verification step 4).
</success_criteria>

<output>
Create `.planning/quick/260805-gdz-lan-print-surface-swallowed-error-and-st/260805-gdz-SUMMARY.md` when done
</output>
