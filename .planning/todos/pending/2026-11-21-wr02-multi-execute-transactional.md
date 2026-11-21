---
created: 2026-11-21T12:00:00.000Z
title: Хвост WR-02 ревью 41.7 — сервисные методы с несколькими writer.execute (транзакционный редизайн) + живая UAT остановки сервера
area: quality
milestone: v1.4
files:
  - crates/trackly-app/src/server/mod.rs
  - crates/trackly-infra/src/db/writer_worker.rs
  - crates/trackly-app/src/services/printer_service.rs
  - crates/trackly-app/src/services/request_service.rs
  - crates/trackly-app/src/services/auth.rs
---

## Problem

Вынесено из `2026-10-09-review-41.7-residual-findings.md` (закрыт quick 261121-hjw). Правка
WR-02 ревью 41.7 (остановка сервера бросала начатый запрос, чья запись уже закоммичена) —
**смягчение, а не устранение**. Отделено в собственный todo, потому что редизайн нескольких
сервисных методов — объём отдельной фазы (стоимость контекста пакета), а не упрощение.

### 1. DRAIN_GRACE — принятый остаточный риск

`DRAIN_GRACE = 3 с` (`crates/trackly-app/src/server/mod.rs`, константа с doc-комментарием)
покрывает обычную LAN-мутацию с запасом, но не случай «очередь записи занята почти весь свой
`send_timeout` 5 с». Окно ограничено и задокументировано у константы. Полное закрытие —
cancellation-safety в писателе либо учёт дренажа на уровне запроса.

### 2. Инвентарь методов с ≥2 точками записи (HEAD 2026-11-21, `src/services/*.rs`, вне `#[cfg(test)]`)

Собрано скриптом по телам функций: прямые `writer.execute(` + self-вызовы методов того же файла,
которые сами владеют `writer.execute(`. Отмена между двумя замыканиями оставляет первое
закоммиченным, а второе — не поставленным в очередь.

Прямые (≥2 `writer.execute(` в одном теле):

- `printer_service.rs::poll_single` — 2

Транзитивные (запись через self-вызовы владельцев записи; часть может быть
взаимоисключающими ветками `match`/`if`, а не последовательностью — проверить при редизайне):

- `request_service.rs::transition` — 1 прямой + `cancel`, `reject_ad_register`
- `auth.rs::on_ad_bind_success` — через `auto_register_ad_user`, `create_pending_registration`,
  `reuse_or_create_pending_registration`, `sync_active_user_name`
- `auth.rs::force_admin_provisioning` — через `force_admin_activate_pending`,
  `force_admin_escalate_active`, `force_admin_insert_unknown`, `force_admin_revive_blocked`,
  `sync_active_user_name` (×4)

Ограничение инвентаря: запись через другой объект/репозиторий/файл скрипт не видит (то же
ограничение, что у реестрового гейта `entities_changed_gate`, см. шапку `scan.rs`). Перед
редизайном перепроверить по HEAD.

### 3. Живая UAT

«Остановить сервер» в момент создания акта по LAN — автотестом не покрыто
(`41.7-HUMAN-UAT.md`, пункт 3). Проверить на живом приложении: акт либо создан и клиент
получил ответ, либо не создан вовсе.

### 4. Реестр угроз

UF-1 (остановка сервера может оставить закоммиченную запись без ответа клиенту) и UF-2
(висящий таймер реконнекта — закрыт кодом в quick 261121-hjw, W6 гейта
`check-ws-disconnect-toast`) из `41.7-SECURITY.md` внести в реестр угроз фазы, которая возьмёт
этот todo: UF-1 — как основную угрозу редизайна, UF-2 — как закрытую со ссылкой на гейт.

## Solution

Отдельная фаза: каждый метод из п.2 — одна транзакция в одном замыкании писателя (или явное
обоснование, почему ветки взаимоисключающие); п.3 — живая UAT; п.4 — строки реестра угроз.
