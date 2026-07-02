---
slug: dashboard-consumption-chart-422
status: resolved
trigger: |
  dashboard consumption chart 422
  В Дашборд не отображаются графики "Динамика расхода картриджей"
created: 2026-07-02
updated: 2026-07-02
---

# Debug Session: dashboard-consumption-chart-422

<!-- CYCLE 1 (HTTP 422): RESOLVED + user live-verified. See "Resolution (cycle 1)" below. -->
<!-- CYCLE 2 (empty render / no plotted data): RESOLVED + user live-verified. See "Resolution (cycle 2)" below. -->

## Current Focus (cycle 2)

- hypothesis: The 422 is fixed; the response now returns data (legend shows 3 model series, one x-tick «Июн.»). But the dev DB has all cartridge installs collapsed into a single month bucket, so `uniqueMonths.length === 1`. In ChartWidget.svelte, `toPoints()` has `if (series.length < 2) return '';` → every single-point series produces an empty `points` string → all `<polyline>` elements draw nothing. A line-only renderer cannot show a single-month series; data exists but is invisible.
- test: Confirm via code inspection that (a) DTO JSON keys match the Svelte interface (data maps correctly), (b) toPoints() bails on <2 points, (c) backend GROUP BY strftime collapses current-month installs into one month_key.
- expecting: All three confirmed → root cause = single-bucket data + line-only renderer. Fix = render a single-month series as visible markers (dots) instead of an invisible zero-length polyline.
- next_action: none — RESOLVED. User live-verify: «Всё хорошо, увидел график с добавлением нового месяца.»

## Symptoms (cycle 2)

- expected: На Дашборде секция «Динамика расхода картриджей» должна показывать построенные данные (линии/точки) по моделям картриджей.
- actual: Ошибки нет, но график пуст. По оси X одна подпись «Июн.» (примерно по центру), ниже — легенда с цветами трёх моделей. Ни линий, ни точек — данные не отображаются.
- error: none (progression from the resolved HTTP 422).
- environment: Desktop (Tauri webview) + LAN browser (server mode).
- timeline: Появилось сразу после устранения 422 — запрос теперь успешен, но отрисовка пуста.
- reproduction: Открыть Дашборд → «Динамика расхода картриджей» (dev DB, где все установки картриджей в текущем месяце).

## Symptoms

- expected: На Дашборде должны отображаться графики «Динамика расхода картриджей».
- actual: Графики не отображаются.
- error: HTTP 422 (Unprocessable Entity) при запросе данных для графика расхода картриджей.
- environment: Воспроизводится и в десктопе (Tauri webview), и в браузере по LAN (server mode).
- timeline: Никогда не работало (новая функция, ни разу не отображалась корректно).
- reproduction: Открыть Дашборд → секция «Динамика расхода картриджей».

## Current Focus (cycle 1 — resolved, kept for history)

- hypothesis: DashboardPage.svelte calls apiCall('dashboard_get_consumption_chart', { window_months }) with a snake_case arg name, but BOTH transports expect camelCase `windowMonths` → deserialization fails → axum returns 422, Tauri rejects the arg.
- test: Compare the arg name the frontend sends vs. what each transport expects (generated bindings.ts, axum payload struct rename_all, backend integration test).
- expecting: Frontend sends `window_months`; both backends want `windowMonths`. Confirmed.
- next_action: awaiting user live-verify of the chart in desktop + browser.
- reasoning_checkpoint:
    hypothesis: "Frontend sends snake_case `window_months`; both transports expect camelCase `windowMonths`, so deserialization fails. Axum surfaces this as 422; Tauri rejects the unknown/missing arg. Same shared-contract mismatch → reproduces in both transports."
    confirming_evidence:
      - "ui/src/bindings.ts:884 (tauri-specta generated) invokes with `{ windowMonths }` — camelCase is the canonical Tauri contract."
      - "http/dashboard.rs:29-31 GetConsumptionChartPayload has #[serde(rename_all = camelCase)] + required (non-Option) window_months:u8 → JSON must be `windowMonths`."
      - "tests/role_endpoint_matrix.rs:1085 asserts axum contract with json!({ \"windowMonths\": 6 }) and passes."
      - "DashboardPage.svelte:99 sent `window_months: windowMonths` (snake_case key), bypassing the generated binding."
      - "dashboard_get_all_widgets works because its only field `period` is single-word (camel==snake) AND Option, so no casing conflict — explains why only the chart 422s."
    falsification_test: "If the key is `windowMonths` and the chart still 422s, the hypothesis is wrong."
    fix_rationale: "The mismatch is purely the arg key casing on the frontend call. Sending `windowMonths` matches both the generated Tauri binding and the axum camelCase payload — fixes root cause on both transports with one change."
    blind_spots: "No live end-to-end request yet; relying on contract inspection + the existing passing integration test + rebuilt-bundle grep. windowMonths value type is number (3|6|12) which matches u8 — fine."

## Evidence

- checked: ui/src/features/dashboard/DashboardPage.svelte:98-100 (loadChart)
  found: Calls apiCall('dashboard_get_consumption_chart', { window_months: windowMonths }) — snake_case arg key (pre-fix).
  implication: Payload key was `window_months`.
- checked: ui/src/bindings.ts:882-884 (tauri-specta generated dashboardGetConsumptionChart)
  found: TAURI_INVOKE("dashboard_get_consumption_chart", { windowMonths }) — camelCase.
  implication: The canonical Tauri contract expects `windowMonths`; the hand-written apiCall call diverged.
- checked: crates/trackly-app/src/http/dashboard.rs:27-31 (GetConsumptionChartPayload)
  found: #[serde(rename_all = "camelCase")] over a required `window_months: u8` → axum expects JSON key `windowMonths`; missing required field → serde error → 422 Unprocessable Entity.
  implication: This is the direct source of the browser 422.
- checked: crates/trackly-app/tests/role_endpoint_matrix.rs:1083-1085
  found: Integration test posts json!({ "windowMonths": 6 }) and passes.
  implication: Backend camelCase contract is correct and tested; frontend is the defect.
- checked: dashboard_get_all_widgets (http/dashboard.rs:21-25 + DashboardPage.svelte:80-82)
  found: Only field `period` — single word (camel==snake) and Option, so absence deserializes to None.
  implication: Explains why widgets load but only the consumption chart 422s.
- checked: ui/dist compiled bundle after fix + rebuild (session-manager independent verification)
  found: New bundle index-D7klIfUd.js emits ge("dashboard_get_consumption_chart",{windowMonths:t(w)}); grep for `window_months` across ui/dist → NONE; stale bundle index-CVXnPDXN.js removed.
  implication: The deployed frontend (server mode / desktop bundle) now sends the correct camelCase key; the stale-dist gap (project memory: dev_browser_testing_needs_ui_build) is closed.

## Eliminated

- Transport-specific bug — ruled out: fails on both Tauri invoke and axum HTTP, sharing the same apiCall args object and the same serde DTO.
- Backend query / service logic — ruled out: service get_consumption_chart passes on empty + populated DB (tests/dashboard_widgets.rs); 422 occurs at request deserialization, before the service runs.
- Auth (401/403) — ruled out: reported status is 422, not 401/403.

## Reasoning Checkpoint (cycle 2)

reasoning_checkpoint:
  hypothesis: "The 422 is resolved and the response now carries data (3-series legend + one «Июн.» x-tick prove data.length>0 and uniqueMonths.length===1). ChartWidget.svelte renders each series only as an SVG <polyline>, and toPoints() returns '' when series.length<2. With a single month bucket, every series is a single point → empty polyline → nothing drawn. Cause = single-time-bucket data rendered by a line-only renderer that requires >=2 points."
  confirming_evidence:
    - "ChartWidget.svelte:44-45 toPoints(): `if (series.length < 2) return '';` — single-point series yields empty points string, polyline draws nothing."
    - "ChartWidget.svelte only draws <polyline> per model (lines 131-140); there is no marker/dot/bar fallback for a 1-length series."
    - "User report: no error, legend with 3 model colors, exactly one x-axis tick «Июн.» → response non-empty AND uniqueMonths.length===1."
    - "dashboard_service.rs:411,420 GROUP BY strftime('%Y-%m', ...) month_key collapses all current-month installs into one bucket; dev DB installs are all in June → single month_key."
    - "ConsumptionPoint DTO (dto/reports.rs:107-116) has no rename_all → JSON keys month_key/model_label/installs match the Svelte interface exactly, so data maps correctly (not a mapping bug)."
  falsification_test: "If uniqueMonths.length were >=2 and lines still didn't draw, the <2-point guard would be irrelevant and the hypothesis wrong. The single centered «Июн.» tick confirms exactly one bucket, so the guard IS the cause."
  fix_rationale: "Add SVG circle markers for every data point so a series is visible regardless of point count. A single-month series then shows a dot (or dots per model) instead of an invisible zero-length polyline. Polylines remain for >=2 buckets. Addresses root cause (invisible single-point series) not a symptom."
  blind_spots: "Whether the user's REAL (non-dev) DB will have multi-month data is unknown — if it does, lines will render and markers are still a strict improvement. Need to confirm with user whether single-month is expected in their data, but the fix is safe either way."

## Resolution (cycle 1 — HTTP 422)

- root_cause: DashboardPage.svelte's loadChart() called apiCall with a snake_case argument key `window_months`, but the shared request contract (tauri-specta generated binding AND the axum GetConsumptionChartPayload with rename_all="camelCase" over a required field) expects `windowMonths`. The key mismatch failed deserialization on both transports — axum returned 422, Tauri rejected the arg — so the consumption chart never loaded. dashboard_get_all_widgets is unaffected because its single field `period` is one word and Option. The deployed ui/dist also still carried the old key, so the fix required a rebuild.
- fix: In ui/src/features/dashboard/DashboardPage.svelte, changed the apiCall payload key from `window_months: windowMonths` to `windowMonths`, then rebuilt ui/dist for server mode.
- verification:
    - svelte-check → 0 errors (only pre-existing warnings).
    - pnpm --dir ui build → ui/dist rebuilt; new bundle index-D7klIfUd.js sends {windowMonths}; grep `window_months` across ui/dist → NONE; stale bundle removed.
    - cargo test -p trackly-app --test role_endpoint_matrix (TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1) → ok, 1 passed (camelCase contract incl. Cases 27/28 dashboard_get_consumption_chart).
    - Live UAT in user's real desktop + LAN-browser environment → CONFIRMED (user live-verify: request succeeds, no error). Cycle 1 CLOSED.
- files_changed: [ui/src/features/dashboard/DashboardPage.svelte, ui/dist (rebuilt, gitignored)]

## Evidence (cycle 2)

- checked: ChartWidget.svelte:44-53 toPoints()
  found: `if (series.length < 2) return '';` then builds coordinates dividing by `(series.length - 1)`. A single-point series returns '' (avoids /0), so the <polyline points=""> draws nothing.
  implication: Any series with exactly one month bucket is invisible. Line-only renderer cannot show single-bucket data.
- checked: ChartWidget.svelte:131-153 SVG body
  found: Only <polyline> per model is drawn (plus x-axis <text>). No <circle>/marker/bar fallback for single points.
  implication: There is no visual element for a 1-length series → confirms empty plot despite present data.
- checked: ChartWidget.svelte:65-71 uniqueMonths + user report
  found: uniqueMonths is a sorted Set of month_key; user sees exactly one x-tick «Июн.» → uniqueMonths.length === 1.
  implication: Data present but single bucket. Matches the toPoints <2 guard exactly.
- checked: dashboard_service.rs:410-421 get_consumption_chart SQL
  found: GROUP BY model_label, month_key where month_key = strftime('%Y-%m', datetime(..., '+3 hours')). Dev DB installs all in current month → one month_key across all rows.
  implication: Backend correctly returns per-model counts, all under a single June bucket. Not a backend defect.
- checked: dto/reports.rs:107-116 ConsumptionPoint + http/dashboard.rs response
  found: ConsumptionPoint has NO #[serde(rename_all)]; fields serialize as month_key/model_label/installs — identical to the Svelte ConsumptionPoint interface (ChartWidget.svelte:8-12).
  implication: Data mapping is correct; ruled out a key-casing/mapping bug on the response side.

## Eliminated (cycle 2)

- Response mapping / key-casing bug — ruled out: ConsumptionPoint DTO has no rename_all; JSON keys (month_key, model_label, installs) exactly match the Svelte interface. Legend rendering (which reads model_label) proves the data deserialized fine.
- Empty-state branch (data.length===0) — ruled out: the {#if data.length === 0} branch would show «Нет данных…» text and NO legend; user sees the legend + one x-tick, so data.length>0.
- Backend query defect — ruled out: SQL correctly groups per model+month; single bucket is expected for a dev DB with all installs in the current month, not a query error.

## Resolution (cycle 2 — empty render)

- root_cause: After the 422 was fixed, the consumption chart request succeeds and returns data, but the dev DB has all cartridge installs in the current month, so the backend GROUP BY strftime('%Y-%m', ...) collapses everything into a single month_key → uniqueMonths.length === 1. ChartWidget.svelte rendered each model series only as an SVG <polyline>, and its toPoints() returned '' whenever series.length < 2. A single-point series therefore produced an empty polyline that drew nothing — the plot was invisible even though data (3 model series, one June bucket) was present. The legend + single «Июн.» x-tick were the only visible artifacts.
- fix: In ui/src/features/dashboard/ChartWidget.svelte:
    - Split the old toPoints() into toCoords() (computes {x,y} per point; a single point is centered at x=200) and toPolyline() (builds the points string, still needs >=2 points).
    - Added a <circle r="3"> marker per data point for every series, so a single-month series is always visible as a dot (and multi-month series show dots on the line). Polylines still render for >=2 buckets.
    - Centered the single-month x-axis tick at x=200 to match the centered marker (was left-aligned at x=10).
- verification:
    - pnpm run svelte-check → 0 errors (37 pre-existing warnings, none in ChartWidget.svelte).
    - TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 pnpm run build → ui/dist rebuilt (export_bindings prebuild passed); new bundle index-CRpQSzbH.js contains the ChartWidget (`Динамика расхода картриджей`) + `circle` markers.
    - Live UAT in user's real desktop + LAN-browser environment → CONFIRMED. User live-verify: «Всё хорошо, увидел график с добавлением нового месяца.» — chart renders (circle markers make the single-month series visible; adding a new month shows the line). Cycle 2 CLOSED.
- files_changed: [ui/src/features/dashboard/ChartWidget.svelte, ui/dist (rebuilt, gitignored)]

## Session Outcome

Both cycles RESOLVED and user live-verified:
- Cycle 1 (HTTP 422): frontend snake_case `window_months` → backend camelCase `windowMonths` serde mismatch; 422 on both transports. Fixed in DashboardPage.svelte.
- Cycle 2 (empty render): single month bucket + line-only renderer (`toPoints()` bailed on <2 points) drew nothing; added SVG circle markers per point so single-bucket series are visible. Fixed in ChartWidget.svelte.
ui/dist is gitignored — rebuild via `pnpm --dir ui build` after checkout.
Note: user separately requested a chart visual-polish task — tracked OUTSIDE this debug session.
