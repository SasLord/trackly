---
phase: 35
slug: act-handover-body
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-11
---

# Phase 35 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `35-RESEARCH.md` §Validation Architecture.

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

> Заполняется планировщиком/исполнителем по мере появления `35-NN-PLAN.md`.
> Строки ниже — требование-ориентированный каркас из RESEARCH.md §Validation Architecture;
> `Task ID` проставляется при создании планов.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | DOC-08 | — | `act.giver_name` интерполируется под `AutoEscape::Html` (существующий sink, новое место) | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_produces_cyrillic_pdf` | ✅ существует (комментарий про Фазу 15 D-09 устарел) | ⬜ pending |
| TBD | TBD | TBD | DOC-07, DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act signature_renders_two_line_labels` | ✅ существует — **требует переписывания** (D-06 отменяет двухстрочные подписи) | ⬜ pending |
| TBD | TBD | TBD | DOC-09 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_act_contains_d09_intro_phrase` | ✅ существует — **не трогать** (D-01: текст образца сохраняется) | ⬜ pending |
| TBD | TBD | TBD | DOC-07 | — | N/A | integration | `cargo test -p trackly-app --test pdf_render_act render_handover_multi_device_wraps_long_fields` | ✅ существует — сверить с D-02/D-11 | ⬜ pending |
| TBD | TBD | TBD | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test html_act_render html_handover_contains_required_blocks_and_logo` | ✅ существует — **требует переписывания** (`html.contains("ФИО")` ломается D-07) | ⬜ pending |
| TBD | TBD | TBD | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test html_act_render html_acceptance_contains_required_blocks` | ✅ существует — **расширить** под D-09 | ⬜ pending |
| TBD | TBD | TBD | DOC-08 | — | N/A | integration | `cargo test -p trackly-app --test acts_e2e_smoke handover_pdf_render_within_e2e` | ✅ существует — комментарий обновить | ⬜ pending |
| TBD | TBD | TBD | DOC-08 (preview) | — | Strict-контекст предпросмотра не падает на новом ключе | integration | `cargo test -p trackly-app --test template_edit` | ✅ существует — косвенное покрытие правки `demo_context_for_kind` | ⬜ pending |
| TBD | TBD | TBD | DOC-07 | — | N/A | structural (опц.) | `cargo test -p trackly-app --test html_field_row_underline_gate` | ❌ Wave 0 (опционально — Open Question 1) | ⬜ pending |
| TBD | TBD | TBD | — (регресс-гейт) | — | N/A | structural | `cargo test -p trackly-app --test html_page_parity` | ✅ существует — **не трогать** | ⬜ pending |
| TBD | TBD | TBD | — (регресс-гейт) | — | N/A | structural | `cargo test -p trackly-app --test html_header_parity` | ✅ существует — **не трогать** (шапка = Фаза 34) | ⬜ pending |
| TBD | TBD | TBD | — (регресс-гейт) | — | N/A | lint/structural | `pnpm --dir ui lint` (`check-print-isolation.mjs`) | ✅ существует — **не трогать** (C-06) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/src/services/template_service.rs` — `demo_context_for_kind`, `_`-ветка: добавить `act.giver_name`. **Обязательно, не опционально** — без этого живой редактор шаблонов падает под `UndefinedBehavior::Strict` на любом теле `act_handover.html`, включая нетронутый бандл. (Находка RESEARCH.md, не названная в CONTEXT.md.)
- [ ] `crates/trackly-app/templates/_legacy_defaults/v22/{act_handover,act_acceptance}.html` — новый срез в состоянии **до** правок этой фазы (текущий HEAD, post-Фаза-34) + регистрация в `KNOWN_LEGACY_DEFAULTS` (C-01/C-04). Фактический номер `v22` подтверждён: на диске только `v20`/`v21`. Существующие тесты слайсов проверяют существование, а не полноту — они пройдут и без этого шага, поэтому пропуск даст «тихую» регрессию класса Фазы 34 D-15.
- [ ] Операционный шаг перед UAT (не тест): удалить `target/debug/templates/` — уже материализован с датой Фазы 34; `cargo tauri dev` пересоздаст его. (Та же ловушка, что в 34-06.)
- [ ] *(Опционально)* `crates/trackly-app/tests/html_field_row_underline_gate.rs` — структурный regex-гейт «между include `_header.html` и `.signatures` нет `border-bottom`», по образцу `html_page_parity.rs`. Пользователь обязательным не делал; дёшев.

---

## Manual-Only Verifications

**Обязательны — критерий #5 фазы и C-05 CONTEXT.md. Text-extraction тесты не видят ни исчезнувших полосок, ни перестроившихся подписей.**

> ⚠️ Проверка = **браузерный Paged.js-предпросмотр** (десктоп + LAN), а **не** `qlmanage`-PDF. Память `act-pdf-word-fidelity` относится к устаревшей krilla-эпохе (до Фазы 16).

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Полоски-подчёркивания отсутствуют под автоподставляемыми полями; остаются только под подписью и под пустым «Сроком до» | DOC-07 (критерий #3) | CSS/геометрия невидимы text-extraction тестам | Открыть предпросмотр акта приёма-передачи в десктоп-вебвью; визуально подтвердить |
| Горизонтальный блок подписей: две строки, напечатанные ФИО, «Подпись» только под полоской | DOC-08 (критерий #4) | Геометрия невидима text-extraction тестам | В том же предпросмотре сверить с D-06/D-07/D-08 |
| Длинные значения («Комплектация», «Технические характеристики») переносятся естественно, без «лесенки» под метку | DOC-07 (D-11) | Перенос — CSS-поведение движка печати | Завести акт с длинными значениями, открыть предпросмотр |
| Множественное число при N > 1: одна строка «были получены устройства:» + список | DOC-09 (D-02) | Формулировка + вёрстка списка вместе видны только в рендере | Завести акт с 2+ устройствами, открыть предпросмотр |
| Всё вышеперечисленное на LAN-транспорте | DOC-07/DOC-08 (критерий #5) | Сервер-режим раздаёт `ui/dist`; десктоп HMR браузер не покрывает (`dev-browser-testing-needs-ui-build`) | `pnpm --dir ui build` → сервер-режим → предпросмотр в LAN-браузере → повторить визуальное сравнение |
| Акт приёмки: блок подписей того же вида, дубль ФИО в таблице убран | DOC-08 (D-09) | Геометрия и дубли не проверяются text-extraction | Открыть предпросмотр «Документ приёма устройства на склад» |
| Редактор шаблонов (Settings → Шаблоны) не падает на предпросмотре `act_handover` / `act_acceptance` | DOC-08 (косвенно) | Требует живого UI; `ui/` без тест-раннера (constraint Фазы 34) | Settings → Шаблоны → выбрать шаблон → «Предпросмотр» без изменений |

🔒 **Приватность при UAT:** описывать результаты обезличенно («ФИО получателя обрезалось»), реальные ФИО и реквизиты организации в SUMMARY/VERIFICATION не попадают (C-07).

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (`demo_context_for_kind`, срез `v22`)
- [ ] No watch-mode flags
- [ ] Feedback latency < 300s (full) / < 30s (quick)
- [ ] Ручной визуальный проход выполнен на **обоих** транспортах
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
