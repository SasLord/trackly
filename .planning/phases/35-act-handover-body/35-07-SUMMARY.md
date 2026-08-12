---
phase: 35-act-handover-body
plan: 07
subsystem: printing
tags: [css, html-templates, regression-tests, act-handover, legacy-defaults, privacy]

# Dependency graph
requires:
  - phase: 35-act-handover-body (планы 01-06)
    provides: переработанное тело акта приёма-передачи (D-01..D-12), горизонтальный блок подписей, срез _legacy_defaults/v22/
provides:
  - "`.signature-row .signature-name` в act_handover.html и act_acceptance.html разрешает перенос длинного ФИО (min-width:0/white-space:normal/overflow-wrap:break-word), без изменений разметки/текста тела"
  - "Структурный CSS-гейт signature_name_css_permits_wrap_for_long_names (по селектору, не html.contains)"
  - "Два теста полного рендер-пайплайна с вымышленной длинной ФИО-фикстурой, подтверждающих целостность имени в <span class=\"signature-name\">"
  - "Срез _legacy_defaults/v23/ (пре-фикс тело обоих шаблонов) + регрессионный тест upgrade_replaces_v23_legacy_default_with_current_bundled_body"
  - "Human-UAT подтверждение переноса длинного вымышленного ФИО на обоих транспортах для обоих актов"
affects: [35-верификация-повтор]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "structural CSS-wrap gate: extract_rule_body(selector) + assert min-width/white-space/overflow-wrap present, bare nowrap absent — reused extract_style_block/extract_rule_body helpers as-is"
    - "legacy-defaults index-N sibling test pattern (bodies.get(N)) — extended to v23"
    - "full-pipeline span-content assertion: find marker before matched fixture text, slice to next </span>, assert_eq exact span content"

key-files:
  created:
    - crates/trackly-app/templates/_legacy_defaults/v23/act_handover.html
    - crates/trackly-app/templates/_legacy_defaults/v23/act_acceptance.html
  modified:
    - crates/trackly-app/templates/act_handover.html
    - crates/trackly-app/templates/act_acceptance.html
    - crates/trackly-app/src/pdf/html_templates.rs
    - crates/trackly-app/tests/html_field_row_underline_gate.rs
    - crates/trackly-app/tests/pdf_render_act.rs

key-decisions:
  - "v23-снимок снят строго ДО CSS-правки (порядок операций из плана соблюдён дословно) — precondition-guard assert_ne! в новом тесте не тривиален"
  - "Task 3 (человеческая UAT-проверка на обоих транспортах, gate=blocking) разрешена как approved — пользователь подтвердил отсутствие обрезания/переполнения длинного вымышленного ФИО на десктопе и в LAN-браузере для обоих актов"

patterns-established: []

requirements-completed: [DOC-08]  # переподтверждено — Success Criterion #4 закрыт этим планом после re-verification gaps_found

# Metrics
duration: ~2h (включая ожидание блокирующей человеческой проверки Task 3)
completed: 2026-08-12
---

# Phase 35 Plan 07: Gap Closure — signature-name ФИО-перенос (DOC-08/SC#4) Summary

**`.signature-row .signature-name` в обоих печатных шаблонах больше не форсирует `nowrap` — длинное кириллическое ФИО переносится внутри доступной ширины строки подписи вместо ухода за печатную область; закрыто структурным CSS-гейтом, тестами полного пайплайна, срезом `_legacy_defaults/v23/` и человеческой UAT-проверкой на обоих транспортах (approved).**

## Performance

- **Duration:** ~2h (включая время ожидания блокирующего human-verify Task 3)
- **Started:** 2026-08-12T~06:32Z
- **Completed:** 2026-08-12T~08:34Z
- **Tasks:** 3 of 3 completed
- **Files modified:** 7

## Accomplishments

- **VERIFICATION.md missing item 1 закрыт:** `.signature-row .signature-name` в обоих
  `act_handover.html` и `act_acceptance.html` заменено с голого `white-space: nowrap;` на
  `min-width: 0; white-space: normal; overflow-wrap: break-word;` — единственная правка CSS-тела
  одного правила, продублированная в обеих копиях (REVIEW.md WR-08, партиал не рефакторится в
  этом плане). Разметка блока подписей не тронута.
- **Ловушка db-backed-templates-upgrade-trap закрыта:** новый срез
  `_legacy_defaults/v23/{act_handover,act_acceptance}.html` захватывает пре-фикс тело (снят ДО
  CSS-правки, порядок операций соблюдён), зарегистрирован четвёртым элементом
  `KNOWN_LEGACY_DEFAULTS` — установленные копии на пред-этим-планом теле получат фикс через
  существующий механизм `upgrade_untouched_defaults_on_startup`. Новый тест
  `upgrade_replaces_v23_legacy_default_with_current_bundled_body` (структурный сиблинг v22-теста,
  `bodies.get(3)`) доказывает недекоративность снимка (`assert_ne!` precondition guard проходит
  нетривиально) и реальный апгрейд.
- **VERIFICATION.md missing item 2 закрыт двумя уровнями доказательства:**
  (a) структурный CSS-гейт `signature_name_css_permits_wrap_for_long_names` в
  `html_field_row_underline_gate.rs` — читает `<style>`-блок по селектору, проверяет наличие
  `min-width: 0`/`white-space: normal`/`overflow-wrap: break-word` и отсутствие голого `nowrap`
  в обоих шаблонах;
  (b) два новых теста полного пайплайна в `pdf_render_act.rs`
  (`render_handover_with_long_giver_name_preserves_full_name_in_signature_block`,
  `render_acceptance_with_long_giver_name_preserves_full_name_in_signature_block`) с новой
  вымышленной фикстурой `LONG_GIVER_NAME_FICTIONAL` («Сидоров-Петроградский-Константинов Иван
  Александрович», 53 символа) — доказывают, что длинное ФИО доходит целиком именно до
  `<span class="signature-name">`, без обрезания и без искажения.
- **Бэкенд не тронут:** `git diff --stat -- crates/trackly-app/src/services/` пуст за весь план.
- **Task 3 (human-UAT, gate=blocking): APPROVED.** Перед запросом подтверждения выполнено:
  `./scripts/check-privacy-requisites.sh` (зелёный), ручной просмотр diff всех 7 изменённых файлов
  (только утверждённые вымышленные значения — «Сидоров-Петроградский-Константинов Иван
  Александрович» и «Получилов П.П.», ни одного реального ФИО/реквизита), удаление устаревшей
  материализованной копии `target/debug/templates/` (оставшейся с прошлого раунда UAT Плана
  35-06 и всё ещё содержавшей дефект `white-space: nowrap`). Пользователь подтвердил обезличенно:
  длинное вымышленное ФИО полностью видно в пределах печатной ширины страницы на ОБОИХ транспортах
  (десктоп + LAN-браузер) для ОБОИХ актов (приёма-передачи и приёмки) — без обрезания и без ухода
  за край листа.
- Полный `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1
  --skip login_remember_persistent_cookie` зелёный (90 test-result блоков, все `ok`, 0 `FAILED`,
  `EXIT_CODE=0`, полный лог перепроверен вручную — без маскировки через `tail`).

## Task Commits

Each task was committed atomically:

1. **Task 1: Снять фикс CSS ФИО-переноса, снять v23-снимок ДО правки, зарегистрировать в
   KNOWN_LEGACY_DEFAULTS** — `f162c79` (fix)
2. **Task 2: Структурный CSS-гейт + тест полного пайплайна с длинным вымышленным ФИО** —
   `35f6a49` (test)
3. **Task 3: Ручная проверка печати длинного ФИО на обоих транспортах** — checkpoint:human-verify,
   gate=blocking; approved by user; no code commit (verification-only task, plus build-artifact
   housekeeping — deletion of stale `target/debug/templates/`, not tracked by git)

**Plan metadata:** this commit (docs: SUMMARY.md + STATE.md + ROADMAP.md + REQUIREMENTS.md).

## Files Created/Modified

- `crates/trackly-app/templates/act_handover.html` — `.signature-row .signature-name` CSS-тело
  заменено на `min-width: 0; white-space: normal; overflow-wrap: break-word;` (3 строки,
  разметка не тронута).
- `crates/trackly-app/templates/act_acceptance.html` — идентичная замена в дублирующей копии
  CSS-блока.
- `crates/trackly-app/templates/_legacy_defaults/v23/act_handover.html` (новый) — снимок тела ДО
  этой правки.
- `crates/trackly-app/templates/_legacy_defaults/v23/act_acceptance.html` (новый) — то же для
  act_acceptance.
- `crates/trackly-app/src/pdf/html_templates.rs` — четвёртый элемент в обоих слайсах
  `KNOWN_LEGACY_DEFAULTS` (`act_handover.html`/`act_acceptance.html`) + новый тест
  `upgrade_replaces_v23_legacy_default_with_current_bundled_body`.
- `crates/trackly-app/tests/html_field_row_underline_gate.rs` — новый тест
  `signature_name_css_permits_wrap_for_long_names` (структурный CSS-гейт по обоим шаблонам).
- `crates/trackly-app/tests/pdf_render_act.rs` — новая константа `LONG_GIVER_NAME_FICTIONAL` +
  два новых теста полного пайплайна (handover + acceptance), доказывающих целостность имени в
  `<span class="signature-name">`.

## Decisions Made

- Порядок операций Task 1 (снимок → правка) соблюдён буквально — иначе `assert_ne!`
  precondition guard нового теста прошёл бы тривиально (тот же класс ловушки Pitfall 5, что
  задокументирован для v21/v22).
- Task 3 разрешён как approved: пользователь подтвердил визуально на живом приложении (не
  синтетическим Playwright/Chromium-харнесом — по проектной памяти `synthetic-harness-not-
  verification`, это единственный достоверный способ проверить перенос текста для
  HTML+Paged.js-пайплайна, RESEARCH.md Pitfall 5).
- Устаревший `target/debug/templates/` (build-артефакт вне git, оставшийся с прошлого раунда
  UAT Плана 35-06) удалён перед человеческой проверкой — без этого шага следующий
  `cargo tauri dev` показал бы старое, ещё дефектное CSS-поведение, и Task 3 ничего бы не
  доказал.

## Deviations from Plan

None — план выполнен дословно для всех 3 задач. Task 3 завершился предписанным планом исходом
"approved", без расхождений. Удаление `target/debug/templates/` — не отклонение от плана, а
прямое действие, прописанное в `<action>` Task 3 ("если... осталась материализованная копия
шаблонов с прошлого раунда UAT — удалить её").

## Issues Encountered

- Первый прогон полного тест-сьюта `cargo test -p trackly-app -- --test-threads=1 --skip
  login_remember_persistent_cookie` был запущен через `| tail -60`, что маскирует истинный код
  завершения `cargo test` (в pipe без `pipefail` возвращается код завершения последней команды —
  `tail`, а не `cargo`). Перепрогнан начисто: вывод перенаправлен в файл, код завершения записан
  явно (`EXIT_CODE=0`), полный лог проверен на предмет любых `test result:` блоков без
  `0 failed` и любых вхождений `FAILED` — ничего не найдено (90/90 блоков `ok`, 0 `FAILED`).
  Чисто процедурная предосторожность выполнения, не влияет на корректность кода.

## Known Follow-ups

Нет новых. Единственный отложенный из 35-06 (снимок `_legacy_defaults/v23/` для правки Task 1
Плана 06, гейт `length==1`) закрыт этим планом заодно тем же новым срезом v23 — сама правка
Плана 06 (`length==1`) была снята и включена в снимок как часть пре-фикс-состояния наравне с
CSS-дефектом WR-02, поэтому установленные копии на теле Плана 06 получат ОБА накопленных
изменения одним апгрейдом.

## User Setup Required

None — конфигурация внешних сервисов не требуется.

## Next Phase Readiness

Единственный оставшийся гэп Фазы 35 из VERIFICATION.md (status: gaps_found, 4/5, 2026-08-12)
закрыт: Success Criterion #4 / DOC-08 подтверждён структурным CSS-гейтом, тестами полного
пайплайна и человеческой UAT-проверкой на обоих транспортах для обоих актов. Бэкенд не тронут.
Полный тест-сьют зелёный. Готово к повторному `/gsd-verify-work` на Фазе 35 (ответственность
оркестратора, не этого агента).

---
*Phase: 35-act-handover-body*
*Completed: 2026-08-12*

## Self-Check: PASSED

Verified before finalizing this summary:
- `[ -f .planning/phases/35-act-handover-body/35-07-SUMMARY.md ]` → FOUND
- `git log --oneline --all | grep f162c79` → FOUND (Task 1 commit)
- `git log --oneline --all | grep 35f6a49` → FOUND (Task 2 commit)
- `grep -c "min-width: 0" crates/trackly-app/templates/act_handover.html` → 1
- `grep -c "min-width: 0" crates/trackly-app/templates/act_acceptance.html` → 1
- `grep -A2 "signature-row .signature-name" crates/trackly-app/templates/act_handover.html | grep -c "white-space: nowrap"` → 0
- `grep -c "v23/act_handover.html" crates/trackly-app/src/pdf/html_templates.rs` → 1
- `grep -c "v23/act_acceptance.html" crates/trackly-app/src/pdf/html_templates.rs` → 1
- `grep -c "fn upgrade_replaces_v23_legacy_default_with_current_bundled_body" crates/trackly-app/src/pdf/html_templates.rs` → 1
- `git diff --stat -- crates/trackly-app/src/services/` (full plan span) → empty (backend untouched)
- `./scripts/check-privacy-requisites.sh` → green, run before both task commits and again before this final commit
- `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1 --skip login_remember_persistent_cookie` (clean run, output to file, explicit exit code) → `EXIT_CODE=0`, 90/90 `test result: ok` blocks, 0 `FAILED` occurrences
