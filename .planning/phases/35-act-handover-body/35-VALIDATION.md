---
phase: 35
slug: act-handover-body
status: audited
nyquist_compliant: true
wave_0_complete: true
manual_only_remaining: 7
created: 2026-08-11
audited: 2026-08-12
---

# Phase 35 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `35-RESEARCH.md` §Validation Architecture.
> **Ретроактивно проаудирован 2026-08-12** (`/gsd-validate-phase 35`): `Task ID`/`Plan`/`Wave`
> проставлены по факту исполнения, статусы перепроверены живым прогоном, добавлены строки для
> тестов волн 5–6, которых не было в исходном каркасе (дыры traceability), закрыты два пробела
> покрытия (G-A/G-B).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` — integration-таргеты в `crates/trackly-app/tests/*.rs` |
| **Config file** | нет отдельного — workspace `Cargo.toml` |
| **Quick run command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1` |
| **Full suite command** | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1` (требует реального `pnpm --dir ui build` в `ui/dist`) |
| **Estimated runtime** | ~20 с quick · ~3–5 мин full |

**Жёсткие ограничения (project memory — не переоткрывать):**
- Никогда не запускать два `cargo test` параллельно — контенция на `target/`-lock выглядит как зависание (`cargo-no-concurrent-test`).
- `cargo test --workspace` виснет на `auth_remember_cookie` — использовать только таргетированные `-p trackly-app` команды выше.
- Полный сьют требует реального `pnpm --dir ui build`: placeholder в `ui/dist` валит `security_headers` SPA-тест (`ci-test-requirements`).

---

## Sampling Rate

- **After every task commit:** `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --lib pdf:: -- --test-threads=1`
- **After every plan wave:** `pnpm --dir ui build` затем полный `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app -- --test-threads=1`
- **Before `/gsd-verify-work`:** полный сьют зелёный **и** ручной визуальный проход (см. Manual-Only) на обоих транспортах
- **Max feedback latency:** ~20 секунд (quick), ~300 секунд (full)

---

## Per-Task Verification Map

> Каркас составлен до исполнения (2026-08-11), **сверен с фактом 2026-08-12**.
> Все команды подразумевают префикс `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1` и суффикс
> `-- --test-threads=1`.
> Статусы ниже — результат живого прогона 2026-08-12, а не переписанное намерение плана.

**Исходный каркас (волны 1–4):**

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 35-02 T3 | 35-02 | 2 | DOC-08 | — | `act.giver_name` интерполируется под `AutoEscape::Html` (существующий sink, новое место) | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_produces_cyrillic_pdf` | ✅ существует | ✅ green |
| 35-04 T1 | 35-04 | 3 | DOC-07, DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act signature_renders_giver_name_horizontal_block` | ✅ **переписан** — `signature_renders_two_line_labels` переименован под D-06 (двухстрочные подписи отменены) | ✅ green |
| 35-02 T2 | 35-02 | 2 | DOC-09 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_contains_d09_intro_phrase` | ✅ существует — не тронут (D-01: текст образца сохранён) | ✅ green |
| 35-04 T1 | 35-04 | 3 | DOC-07 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_multi_device_wraps_long_fields` | ✅ сверен с D-02/D-11 | ✅ green |
| 35-04 T2 | 35-04 | 3 | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test html_act_render html_handover_contains_required_blocks_and_logo` | ✅ **переписан** — ассерция `html.contains("ФИО")` снята (D-07); метки уточнены в 35-06 T3 | ✅ green |
| 35-03 T2 / 35-04 T2 | 35-03, 35-04 | 2, 3 | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test html_act_render html_acceptance_contains_required_blocks` | ✅ **расширен** под D-09 (проверка дедупликации ФИО в таблице) | ✅ green |
| 35-04 T3 | 35-04 | 3 | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test acts_e2e_smoke handover_pdf_render_within_e2e` | ✅ ассерция была совместима, обновлён только комментарий | ✅ green |
| 35-01 T2 | 35-01 | 1 | DOC-08 (preview) | — | Strict-контекст предпросмотра не падает на новом ключе | integration + unit | `cargo test -p trackly-app --test template_edit` · `cargo test -p trackly-app --lib validate_preview_act_handover_returns_html_with_title_marker` | ✅ прямой гейт против Pitfall 1 (`demo_context_for_kind` → `act.giver_name`) | ✅ green (6 + 1) |
| 35-04 T3 | 35-04 | 3 | DOC-07 | — | N/A | structural | `cargo test -p trackly-app --test html_field_row_underline_gate field_row_css_has_no_border_bottom_and_only_two_legit_exceptions_remain` | ✅ **создан** — опциональный пункт Wave 0 всё-таки реализован | ✅ green |
| — | — | — | — (регресс-гейт) | — | N/A | structural | `cargo test -p trackly-app --test html_page_parity` | ✅ существует — не тронут | ✅ green (1) |
| — | — | — | — (регресс-гейт) | — | N/A | structural | `cargo test -p trackly-app --test html_header_parity` | ✅ существует — не тронут (шапка = Фаза 34) | ✅ green (5) |
| 35-05 T1 | 35-05 | 4 | — (регресс-гейт) | — | N/A | lint/structural | `pnpm --dir ui lint` (`check-print-isolation.mjs`) | ✅ существует — не тронут (C-06) | ✅ green |

**Добавлено волнами 5–6 (закрытие гэпов верификации; в исходном каркасе отсутствовало —
дыра traceability, закрыта этим аудитом):**

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 35-01 T1 | 35-01 | 1 | DOC-08 (доставка) | — | N/A | unit | `cargo test -p trackly-app --lib every_default_template_has_a_known_legacy_defaults_entry` | ✅ существующий WR-06 гейт, не сломан добавлением v22/v23 | ✅ green |
| 35-06 T1 | 35-06 | 5 | DOC-09 (CR-01 / D-02a) | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_multi_device_fields_attributable_to_own_device` | ✅ **создан** — регресс атрибуции полей своему `.device-block` при N=3 | ✅ green |
| 35-06 T2 | 35-06 | 5 | WR-01 (доставка среза) | — | N/A | unit | `cargo test -p trackly-app --lib upgrade_replaces_v22_legacy_default_with_current_bundled_body` | ✅ **создан** — доказывает реальный upgrade установленных копий с пред-Фазой-35 телом | ✅ green |
| 35-06 T3 | 35-06 | 5 | DOC-07 (IN-01) | — | N/A | structural | `cargo test -p trackly-app --test html_field_row_underline_gate acceptance_signature_line_css_has_exactly_one_legitimate_border_bottom` | ✅ **создан** — DOC-07-эквивалент для `act_acceptance.html` | ✅ green |
| 35-07 T1 | 35-07 | 6 | WR-01 (доставка среза) | — | N/A | unit | `cargo test -p trackly-app --lib upgrade_replaces_v23_legacy_default_with_current_bundled_body` | ✅ **создан** — срез пре-фикс-состояния Плана 07, с анти-вакуозным guard | ✅ green |
| 35-07 T2 | 35-07 | 6 | DOC-08 (WR-02, SC#4) | — | N/A | structural | `cargo test -p trackly-app --test html_field_row_underline_gate signature_name_css_permits_wrap_for_long_names` | ✅ **создан** — CSS-гейт по селектору: `min-width:0`/`white-space:normal`/`overflow-wrap:break-word` есть, голого `nowrap` нет, в обоих шаблонах | ✅ green |
| 35-07 T2 | 35-07 | 6 | DOC-08 (WR-02, SC#4) | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_with_long_giver_name_preserves_full_name_in_signature_block` | ✅ **создан** — полный пайплайн, вымышленная 53-символьная ФИО-фикстура | ✅ green |
| 35-07 T2 | 35-07 | 6 | DOC-08 (WR-02, SC#4) | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_acceptance_with_long_giver_name_preserves_full_name_in_signature_block` | ✅ **создан** — то же для акта приёмки | ✅ green |

**Добавлено ретроактивным Nyquist-аудитом 2026-08-12 (`/gsd-validate-phase 35`) — поведение
шаблона было верным, но ни один тест эти ветки не рендерил:**

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| G-A | 35-02 (поведение) | audit | DOC-09 (D-02, N=1) | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render html_handover_single_device_renders_singular_intro_not_plural_summary -- --test-threads=1` | ✅ добавлен (Nyquist, 2026-08-12) | ✅ green |
| G-A | 35-02 (поведение) | audit | DOC-09 (D-02 + D-02a, N>1) | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render html_handover_multi_device_renders_plural_summary_listing_every_name -- --test-threads=1` | ✅ добавлен (Nyquist, 2026-08-12) | ✅ green |
| G-B | 35-02 (поведение) | audit | DOC-07 (D-03 + D-12, пустой дедлайн) | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render html_handover_without_deadline_renders_row_with_blank_underline -- --test-threads=1` | ✅ добавлен (Nyquist, 2026-08-12) | ✅ green |
| G-B | 35-02 (поведение) | audit | DOC-07 (D-03 + D-12, заполненный дедлайн) | — | N/A | integration | `TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 cargo test -p trackly-app --test html_act_render html_handover_with_deadline_renders_ru_date_without_blank_underline -- --test-threads=1` | ✅ добавлен (Nyquist, 2026-08-12) | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Retroactive gap closure (Nyquist, 2026-08-12)

Две дыры покрытия закрыты четырьмя интеграционными тестами в
`crates/trackly-app/tests/html_act_render.rs` (шаблоны и `src/` не менялись):

- **G-A (DOC-09 / D-02):** ветвление «единственное число vs множественное» ранее не
  проверялось ни одним тестом (значилось только как Manual-Only визуальная проверка).
  Тесты утверждают точную разметку `.field-row` для N=1 и отсутствие в ней сводной строки/
  `<ul>`, а для N=3 — сводную строку «были получены устройства:» + `<li>` на каждое имя
  (ровно 3) + сохранение персональной строки «было получено устройство: ⟨имя⟩` в каждом
  `.device-block`. **Формулировка гэпа устарела:** D-02a (закрытие CR-01, план 35-06) снял
  гейт `{% if act.items | length == 1 %}` — при N>1 обе формулировки присутствуют
  одновременно и это ожидаемое поведение, поэтому «отсутствие второй фразы» проверяется
  только для N=1 и внутри сводного `<ul>`.
- **G-B (DOC-07 / D-03 + D-12):** заполненная ветка «Сроком до» не рендерилась ни одной
  фикстурой (везде `deadline_utc: None`). Добавлена фикстура
  `create_handover_with_deadline`; тесты проверяют обе ветки рендера — пустую (строка
  выводится безусловно + `<span class="value-blank"></span>`) и заполненную (RU-дата из
  `deadline_human`, полоски нет).

**Доказательство фальсифицируемости:** одноразовый тест (создан, прогнан, удалён до коммита —
тот же приём, что в 35-02) прогнал pre-Phase-35 срез
`_legacy_defaults/v22/act_handover.html` через тот же пайплайн и подтвердил, что срез **не
удовлетворяет ни одному** из четырёх новых утверждений (при этом RU-дата дедлайна в его вывод
попадает — то есть утверждение о заполненной ветке не вырожденное). Новые тесты действительно
различают старое и новое поведение, а не проходят тривиально.

Manual-Only строка «Множественное число при N > 1» ниже остаётся в силе: тесты покрывают
**формулировку и состав разметки**, но не вёрстку/геометрию списка при печати.

---

## Wave 0 Requirements

Все выполнены (подтверждено аудитом 2026-08-12).

- [x] `crates/trackly-app/src/services/template_service.rs` — `demo_context_for_kind`, `_`-ветка: добавить `act.giver_name`. **Обязательно, не опционально** — без этого живой редактор шаблонов падает под `UndefinedBehavior::Strict` на любом теле `act_handover.html`, включая нетронутый бандл. (Находка RESEARCH.md, не названная в CONTEXT.md.) → План 35-01 T2, коммит `1249e5e`; гейт `validate_preview_act_handover_returns_html_with_title_marker` зелёный.
- [x] `crates/trackly-app/templates/_legacy_defaults/v22/{act_handover,act_acceptance}.html` — новый срез в состоянии **до** правок этой фазы (текущий HEAD, post-Фаза-34) + регистрация в `KNOWN_LEGACY_DEFAULTS` (C-01/C-04). Фактический номер `v22` подтверждён: на диске только `v20`/`v21`. Существующие тесты слайсов проверяют существование, а не полноту — они пройдут и без этого шага, поэтому пропуск даст «тихую» регрессию класса Фазы 34 D-15. → План 35-01 T1, коммит `e0d2dca`. Предупреждение о «проверяют существование, а не полноту» сбылось: настоящий гейт полноты (`upgrade_replaces_v22_…`) пришлось добавлять постфактум в 35-06 T2 (WR-01). Срез `v23` добавлен в 35-07 T1 со своим гейтом.
- [x] Операционный шаг перед UAT (не тест): удалить `target/debug/templates/` — уже материализован с датой Фазы 34; `cargo tauri dev` пересоздаст его. (Та же ловушка, что в 34-06.) → План 35-05 T1.
- [x] *(Было опционально — реализовано)* `crates/trackly-app/tests/html_field_row_underline_gate.rs` — структурный гейт DOC-07. → План 35-04 T3; реализован не как regex по диапазону разметки, а как проверка тел CSS-правил по селектору (`extract_style_block`/`extract_rule_body`) — устойчивее к тому, что весь `<style>` лежит до любого маркера разметки. Позже расширен до 3 тестов (35-06 T3, 35-07 T2).

---

## Manual-Only Verifications

**Обязательны — критерий #5 фазы и C-05 CONTEXT.md. Text-extraction тесты не видят ни исчезнувших полосок, ни перестроившихся подписей.**

> ⚠️ Проверка = **браузерный Paged.js-предпросмотр** (десктоп + LAN), а **не** `qlmanage`-PDF. Память `act-pdf-word-fidelity` относится к устаревшей krilla-эпохе (до Фазы 16).

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Полоски-подчёркивания отсутствуют под автоподставляемыми полями; остаются только под подписью и под пустым «Сроком до» | DOC-07 (критерий #3) | CSS/геометрия невидимы text-extraction тестам | Открыть предпросмотр акта приёма-передачи в десктоп-вебвью; визуально подтвердить |
| Горизонтальный блок подписей: две строки, напечатанные ФИО, «Подпись» только под полоской | DOC-08 (критерий #4) | Геометрия невидима text-extraction тестам | В том же предпросмотре сверить с D-06/D-07/D-08 |
| Длинные значения («Комплектация», «Технические характеристики») переносятся естественно, без «лесенки» под метку | DOC-07 (D-11) | Перенос — CSS-поведение движка печати | Завести акт с длинными значениями, открыть предпросмотр |
| Множественное число при N > 1: одна строка «были получены устройства:» + список | DOC-09 (D-02) | **Частично автоматизировано 2026-08-12** (G-A): формулировка и состав разметки покрыты тестами; вручную остаётся только вёрстка/геометрия списка при печати | Завести акт с 2+ устройствами, открыть предпросмотр |
| Всё вышеперечисленное на LAN-транспорте | DOC-07/DOC-08 (критерий #5) | Сервер-режим раздаёт `ui/dist`; десктоп HMR браузер не покрывает (`dev-browser-testing-needs-ui-build`) | `pnpm --dir ui build` → сервер-режим → предпросмотр в LAN-браузере → повторить визуальное сравнение |
| Акт приёмки: блок подписей того же вида, дубль ФИО в таблице убран | DOC-08 (D-09) | Геометрия и дубли не проверяются text-extraction | Открыть предпросмотр «Документ приёма устройства на склад» |
| Редактор шаблонов (Settings → Шаблоны) не падает на предпросмотре `act_handover` / `act_acceptance` | DOC-08 (косвенно) | Требует живого UI; `ui/` без тест-раннера (constraint Фазы 34) | Settings → Шаблоны → выбрать шаблон → «Предпросмотр» без изменений |

🔒 **Приватность при UAT:** описывать результаты обезличенно («ФИО получателя обрезалось»), реальные ФИО и реквизиты организации в SUMMARY/VERIFICATION не попадают (C-07).

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — 19 задач в 7 планах; каждая `type="auto"` несёт `<automated>`-блок, три `checkpoint:human-verify` (35-05 T2, 35-06 T4, 35-07 T3) — блокирующие ручные гейты по критерию #5 фазы, подтверждены пользователем.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — максимальный разрыв 1 задача (ручной чекпоинт всегда следует за авто-задачей).
- [x] Wave 0 covers all MISSING references (`demo_context_for_kind`, срез `v22`) — оба выполнены в Плане 35-01.
- [x] No watch-mode flags — все команды одноразовые.
- [x] Feedback latency < 300s (full) / < 30s (quick) — фактически: таргетированные наборы 0.02–0.47 с, полный пакет ~3 мин.
- [x] Ручной визуальный проход выполнен на **обоих** транспортах — трижды (35-05 T2, 35-06 T4, 35-07 T3), каждый раз подтверждён пользователем.
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** ✅ approved 2026-08-12 (`/gsd-validate-phase 35`)

---

## Validation Audit 2026-08-12

| Metric | Count |
|--------|-------|
| Требований в фазе | 3 (DOC-07, DOC-08, DOC-09) |
| Строк карты до аудита | 12 (все `Task ID: TBD`, все `⬜ pending`) |
| Строк карты после аудита | 24 |
| Дыр traceability закрыто | 8 (тесты волн 1/5/6, отсутствовавшие в карте) |
| Gaps found | 3 (G-A, G-B, G-C) |
| Resolved | 2 (G-A, G-B — 4 новых теста) |
| Escalated / deferred | 1 (G-C — см. ниже) |
| Тестов прогнано для подтверждения статусов | 43 (8 integration-таргетов + `--lib pdf::` + preview-тест) + `pnpm --dir ui lint` |
| Red / flaky | 0 |

**Прогон подтверждения (2026-08-12, живой, не со слов планов):** `html_act_render` 15/15,
`pdf_render_act` 15/15, `html_field_row_underline_gate` 3/3, `acts_e2e_smoke` 4/4,
`template_edit` 6/6, `html_header_parity` 5/5, `html_page_parity` 1/1,
`--lib pdf::` (вкл. `upgrade_replaces_v2{1,2,3}_*`, `every_default_template_has_a_known_legacy_defaults_entry`) все зелёные,
`validate_preview_act_handover_returns_html_with_title_marker` зелёный,
`pnpm --dir ui lint` PASS (вкл. `check-print-isolation.mjs`, `check-pagedjs-csp-hash.mjs`),
`./scripts/check-privacy-requisites.sh` OK.

### G-C — отложено (не закрыто этим аудитом)

**Пробел:** нет durable-гейта «тело бандл-шаблона изменилось → нужен новый срез
`_legacy_defaults`». Тесты `upgrade_replaces_vNN_*` доказывают доставку **уже снятых** срезов,
но не заставляют снять новый при следующей правке тела. Это и есть открытая находка ревью
фазы WR-01 ([35-REVIEW.md:113](35-REVIEW.md), [35-VERIFICATION.md:104](35-VERIFICATION.md)):
из промежуточных тел Фазы 35 зарегистрированы только `v22` и `v23`.

**Почему отложено:** пользователь ограничил объём аудита пробелами по требованиям фазы
(DOC-07/08/09); G-C — инфраструктурный гейт вне их периметра, и он потребует обновления
константы при каждой правке шаблона. Верификация фазы уже классифицировала WR-01 как
WARNING, а не гэп: риск ограничен dev/UAT-машинами, ни один выпущенный релизный тег не несёт
непроверенного промежуточного тела.

**Предлагаемая реализация, если браться:** пин-тест в `pdf/html_templates.rs`, сверяющий
sha256 каждого бандл-тела акта с записанной константой — правка тела валит тест и вынуждает
одновременно снять срез и обновить константу. Кандидат в бэклог, не в Фазу 35.
