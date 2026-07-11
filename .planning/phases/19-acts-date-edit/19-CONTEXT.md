# Phase 19: Акты — дата и редактирование - Context

**Gathered:** 2026-07-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Фаза 19 доставляет две вещи:

1. **ACT-01 — дата акта.** Введённая пользователем дата «Когда отдали» (`handover_date_utc`) становится официальной «Датой» акта во всём приложении — в списке, карточке, PDF/печати и сортировке. Текущая дата больше не выступает как дата акта.
2. **ACT-02 — редактирование акта.** Существующий handover-акт можно открыть в рабочей форме редактирования (кнопка «Редактировать» активна) и сохранить изменения без ошибок — включая шапку и позиции устройств.

**Не в scope:** новые типы актов, изменение нумерации/формата номеров, редактирование return-актов, печать/шапка организации (Phase 20).
</domain>

<decisions>
## Implementation Decisions

### Дата акта (ACT-01)
- **D-01:** «Дата» акта = `handover_date_utc` **везде** — список ([ActListRow.svelte:36](ui/src/features/acts/ActListRow.svelte:36)), карточка ([ActDetail.svelte:43](ui/src/features/acts/ActDetail.svelte:43)), PDF/HTML-печать и сортировка/группировка списка. Сейчас все эти места используют `created_at_utc`.
- **D-02:** `created_at_utc` остаётся, но становится **чисто внутренним** таймстампом «когда запись создана» — не отображается пользователю как дата акта.
- **D-03:** Backend уже корректно сохраняет `handover_date_utc` при создании ([act_service.rs:270](crates/trackly-app/src/services/act_service.rs:270)) — правка ACT-01 в первую очередь на стороне отображения/сортировки/рендера, не на пути записи. Отчёты уже фильтруют/группируют по `handover_date_utc` — консистентно с D-01.

### Объём редактирования (ACT-02)
- **D-04:** Редактируются **и шапка, и позиции** акта. Шапка: №, даты («Когда отдали»/«Сроком до»), Сдал/Принял, комплектация/технические характеристики. Позиции: добавить / убрать / сменить устройства.

### Побочные эффекты на технику (ACT-02)
- **D-05:** **Поля шапки меняются свободно** — состояние устройств (статус/локация/история) при этом не затрагивается.
- **D-06:** При **изменении позиций** — пересобрать эффекты на устройства **по дельте**: сравнить старый и новый списки позиций; убранные устройства откатить к прежнему состоянию/локации; добавленные перевести так же, как при `create` (статус «в работе», локация); писать запись истории/аудита на каждое изменение. (Пути side-effect'ов при create: [act_service.rs:200+](crates/trackly-app/src/services/act_service.rs:200).)

### Какие акты редактируемы (ACT-02)
- **D-07:** Редактируемы **только handover-акты** (`act_type = 'handover'`), включая те, по которым уже был возврат, и архивные. Return-акты — кнопка «Редактировать» неактивна.
- **D-08:** Правка **позиций** у акта, по которому уже есть возврат, должна **валидироваться против существующих возвратов** (нельзя убрать/сменить устройство, уже завязанное на выполненный return-акт). Правка шапки такого акта — свободна (D-05).

### Claude's Discretion
- UX формы редактирования: переиспользовать ли `ActFormModal`/`ActFormBody` в edit-режиме или отдельный компонент — на усмотрение планировщика (переиспользование предпочтительно для консистентности).
- Оптимистическая конкурентность при сохранении: у актов есть поле `version` (см. INSERT в [acts_sqlite.rs:94](crates/trackly-infra/src/repos/acts_sqlite.rs:94)) — механизм проверки версии при update определяет планировщик.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — ACT-01, ACT-02 (полные формулировки).
- `.planning/ROADMAP.md` §"Phase 19" — Goal, Success Criteria, и заметка: ACT-02 требует шага диагностики перед реализацией (первопричина: пути `update` в бэкенде нет вообще).

### Backend — акты
- `crates/trackly-app/src/services/act_service.rs` — `create` (side-effects на устройства, `handover_date_utc` at :270), `do_return`, `delete_soft`, `get`, `search`, `render_pdf`/`render_acceptance_pdf`. **Функции `update`/`edit` нет** — её нужно построить.
- `crates/trackly-app/src/dto/act.rs` — `ActCreateDto` (:172), `handover_date_utc` (:196). Понадобится DTO для update.
- `crates/trackly-infra/src/repos/acts_sqlite.rs` — INSERT/маппинг строк, поле `version` (:94), `handover_date_utc` (:111).
- `crates/trackly-core/src/domain/acts.rs` — доменная модель, семантика `handover_date_utc` (:141).

### Frontend — акты
- `ui/src/features/acts/ActFormBody.svelte` — форма создания; шлёт `handover_date_utc` (:115), `handoverDateISO` (:44). База для edit-режима.
- `ui/src/features/acts/ActFormModal.svelte` — модалка создания.
- `ui/src/features/acts/ActDetail.svelte` — кнопка «Редактировать» (:70, `disabled={!onEdit}`), «Дата» из `created_at_utc` (:43).
- `ui/src/features/acts/ActListRow.svelte` — «Дата» из `created_at_utc` (:36).
- `ui/src/features/acts/ActsPage.svelte` — оркестрация модалок/onCreate/onEdit (:218+).
- `ui/src/lib/api/acts.ts` — клиент API актов (нужен `update`).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ActFormBody.svelte` / `ActFormModal.svelte`: create-форма уже собирает всю шапку и позиции — основа для edit-режима (prefill из существующего акта + submit в новый `update`-путь).
- `acts.create` + `do_return` + `delete_soft` в `act_service.rs`: паттерн транзакции с device side-effects и аудитом — образец для реализации `update` с дельта-согласованием (D-06).
- `format_ru_date`/`format_iso_date` ([act_service.rs:1655](crates/trackly-app/src/services/act_service.rs:1655)) и `formatDate` во фронте — переключить источник на `handover_date_utc`.

### Established Patterns
- Все записи идут через единый data/service-слой (`act_service`), Tauri- и HTTP-адаптеры тонкие — новый `update` должен следовать тому же паттерну (одна транзакция, один writer).
- Оптимистическая конкурентность через `version` (используется в `delete_soft(id, version)`) — применить и к update.
- Return-акты наследуют `handover_date_utc` от parent ([act_service.rs:714](crates/trackly-app/src/services/act_service.rs:714)) — учесть при валидации D-08.

### Integration Points
- Новый `acts_update` Tauri-команда + `/api/v1` HTTP-хэндлер + `acts.ts` клиент.
- `onEdit` проброс: `ActDetail` → `ActsPage` → `ActFormModal` в edit-режиме.
- Смена источника даты затрагивает: список, карточку, оба PDF-рендера, сортировку в `search`/`list`.

</code_context>

<specifics>
## Specific Ideas

- ACT-01 симптом «всегда текущая дата» = дата вводится и хранится (`handover_date_utc`), но UI/PDF показывают `created_at_utc`. Первопричина — на стороне отображения, а не записи.
- ACT-02 «не работает» = функции обновления акта в бэкенде **не существует**, кнопка «Редактировать» задизейблена (`disabled={!onEdit}`, `onEdit` не проброшен). Диагностика из ROADMAP-заметки этим и исчерпывается — планирование строит update-путь с нуля.

</specifics>

<deferred>
## Deferred Ideas

None — обсуждение осталось в границах фазы.

</deferred>

---

*Phase: 19-acts-date-edit*
*Context gathered: 2026-07-11*
