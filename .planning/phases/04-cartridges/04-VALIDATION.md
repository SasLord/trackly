---
phase: 04
slug: cartridges
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-07
---

# Phase 04 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source of truth: `04-RESEARCH.md` → "Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (стандарт), `cargo nextest` (опционально) + `pnpm svelte-check` / `pnpm lint` для фронта |
| **Config file** | `Cargo.toml` `[dev-dependencies]` |
| **Quick run command** | `cargo test -p trackly-app --test <новый тест> -- --nocapture` |
| **Full suite command** | `cargo test && pnpm svelte-check && pnpm lint` |
| **Estimated runtime** | ~60–90 секунд (cargo test), +20–30с фронт |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p trackly-app --test <новый тест>`
- **After every plan wave:** Run `cargo test && pnpm svelte-check`
- **Before `/gsd-verify-work`:** полный `cargo test && pnpm svelte-check && pnpm lint` зелёный
- **Max feedback latency:** ~90 секунд

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists |
|--------|----------|-----------|-------------------|-------------|
| CART-04 | Авто-код `C-000001` атомарен, нет коллизий | integration | `cargo test --test cartridges_numbering` | ❌ W0 |
| CART-04 | Custom override пишется в `audit_log` | integration | `cargo test --test cartridges_crud` | ❌ W0 |
| CART-03/05 | CRUD создание/получение/удаление + counts по статусам | integration | `cargo test --test cartridges_crud` | ❌ W0 |
| CART-06/07/08/09 | Transition меняет status + пишет `audit_log` | integration | `cargo test --test cartridges_lifecycle` | ❌ W0 |
| CART-11 | FTS/LIKE поиск по коду/модели/расположению | integration | `cargo test --test cartridges_search` | ❌ W0 |
| CART-12 | low_stock возвращает модели ниже порога | integration | `cargo test --test cartridges_low_stock` | ❌ W0 |
| CART-10 | History из `audit_log` для экземпляра | integration | `cargo test --test cartridges_history` | ❌ W0 |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/trackly-app/tests/cartridges_crud.rs` — CART-03/04/05 создание/получение/удаление + counts
- [ ] `crates/trackly-app/tests/cartridges_lifecycle.rs` — CART-06/07/08/09 переходы + audit_log
- [ ] `crates/trackly-app/tests/cartridges_numbering.rs` — CART-04 авто-код + коллизия retry
- [ ] `crates/trackly-app/tests/cartridges_search.rs` — CART-11 FTS + LIKE + JOIN модель
- [ ] `crates/trackly-app/tests/cartridges_low_stock.rs` — CART-12 подсчёт ниже порога
- [ ] `crates/trackly-app/tests/cartridges_history.rs` — CART-10 audit_log чтение
- [ ] Обновить `crates/trackly-infra/src/test_support/test_db.rs:41` assertion с `15` → `16` (новая миграция)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Контекстное меню меняется по статусу картриджа | CART-06 | Зависит от UI-рендера и наведения | Открыть раздел «Картриджи», убедиться что для «На складе»/«В работе»/«На заправке» набор действий отличается согласно 04-UI-SPEC.md |
| Баннер «низкий остаток» виден в разделе | CART-12 | Визуальная проверка баннера | Опустить остаток модели ниже порога, проверить баннер с моделью и количеством |
| Автокомплит совместимых принтеров/расположений | CART-01/CART-08 | UX-взаимодействие | Проверить предложения из ранее введённых значений |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
