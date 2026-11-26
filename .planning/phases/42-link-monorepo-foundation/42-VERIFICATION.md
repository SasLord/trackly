---
phase: 42-link-monorepo-foundation
verified: 2026-11-26T00:00:00Z
status: human_needed
score: 3/4 roadmap criteria verified automatically; criterion 4 and visual parts of 2 deferred to human
overrides_applied: 0
gaps: []
human_verification:
  - test: "Пилот DST-06 на 2-3 доменных ПК без прав администратора и одном RDS-сервере (docs/pilot/README.md, install-silent.ps1, diag.ps1)"
    expected: "Чек-лист заполнен, события AppLocker 8002/8003 без 8004, итог обезличенно в docs/pilot/RESULT.md, ADR 0004/0005 подтверждены (сейчас Accepted provisional)"
    why_human: "Нужны живые доменные Windows-ПК; D-07: не блокирует закрытие фазы, блокирует планирование Фазы 45"
  - test: "Запуск окна Link (trackly-link.exe из артефакта dry-run) на Windows"
    expected: "Логотип «Link» слева вверху, вкладка «Чат» справа от него, сайдбар слева, содержимое справа; размер и положение окна запоминаются; StatusLine заполнена (CSP и IPC link_info проходят)"
    why_human: "Окно Link ни разу не запускалось визуально; гейт check-link-layout проверяет только исходник"
  - test: "Десктопная сборка Trackly после переезда (ветка Tauri-invoke в packages/transport)"
    expected: "Вход/выход, onUnauthorized при протухшей сессии, блок вложений открывается"
    why_human: "Стенд D-21 проверил только LAN-браузер; десктоп остаётся UNVERIFIED (WR-02/WR-03 ревью указывают на эту ветку)"
  - test: "Карточка устройства с вложением и 403 в LAN-браузере на реальных данных"
    expected: "Блок вложений без console.error; запрещённое действие даёт onForbidden"
    why_human: "В БД стенда не было устройств с вложениями"
---

# Фаза 42: верификация

**Цель:** репозиторий готов к двум клиентам: Link как отдельное приложение со своей версией на общих пакетах, решения в ADR, пилот выполнимости раскатки.
**Статус:** human_needed. Автоматически проверяемое подтверждено, расхождений с целью в коде не найдено; остаются живые проверки.

## Критерии roadmap

| # | Критерий | Статус | Доказательства |
|---|----------|--------|----------------|
| 1 | Link собирается отдельным приложением со своей версией; dry-run не задевает Trackly; прежние гейты зелёные | VERIFIED | crates/link-app: version = "0.1.0" литерально (не version.workspace), tauri.conf.json: version 0.1.0, identifier org.trackly.link, mainBinaryName trackly-link. release-link.yml на теги link-v*; `gh run view 45754264682`: success на fc6b56e1; список релизов без Link-релиза. `scripts/check-release-isolation.mjs` PASS. Я прогнал `pnpm --dir ui lint` (exit 0, 75 гейтов в lint-selftest-wiring), `pnpm --dir link-ui build` успешно. cargo test --workspace 3083/0 принято по логу оркестратора, сам не перезапускал |
| 2 | Каркас окна Link: логотип слева вверху, вкладка «Чат» справа, сайдбар слева, содержимое справа | VERIFIED по исходнику, визуально UNVERIFIED | link-ui/src/App.svelte: grid topbar (LinkLogo, Tabs с одной вкладкой «Чат»), aside-сайдбар слева, main справа; `check-link-layout` selftest и прогон PASS (33 признака). Окно не запускалось, см. human_verification |
| 3 | Компонент из общего пакета виден в Trackly и Link без копирования | VERIFIED | packages/kit/src/components/Button.svelte единственный; ui импортирует его в 96 файлах, link-ui (StatusLine) тоже; Tabs/ThemeSwitcher общие; реэкспорт-прокладок нет; check-workspace-boundaries в lint зелёный |
| 4 | Пилот проведён, итог обезличенно, ADR принят | DEFERRED (human) | ADR 0001-0005 и README есть, 0004 в статусе Accepted (provisional); набор пилота (README, install-silent.ps1, diag.ps1, sign-notes.md, RESULT-TEMPLATE.md) есть; RESULT.md = «ОТЛОЖЕНО: human-UAT (D-07)». Допустимо по D-07 |

## Дополнительные must-haves (выборочно)

- D-20 гейт покрытия: check-gate-coverage с baseline и perGate подключён в ui lint парой selftest + прогон, зелёный.
- D-18 workspace: pnpm-workspace.yaml (ui, link-ui, packages/*), единый pnpm-lock.yaml в корне.
- D-21 стенд: 42-STAND-BASELINE.md и 42-STAND-RESULT.md присутствуют; новых ошибок консоли нет (по заявлению оркестратора, браузерный прогон мной не повторялся).
- D-22: пять ADR в docs/adr.

## Требования

| ID | Статус | Комментарий |
|----|--------|-------------|
| LNK-01 | SATISFIED для фазы 42 | Отдельная сборка с версией и общие пакеты (kit, design, transport, attachments) подтверждены. Часть «компоненты чата общие» намеренно отложена (D-19: чат заявок остаётся в ui/ до v1.6); пометка Complete в REQUIREMENTS.md в конце фазы оправдана, но формулировку про чат стоит учитывать при аудите вехи |
| LNK-10 | Код на месте, визуально не подтверждён | Оставить Pending до запуска окна (human 2) |
| DST-06 | Не выполнено (отложено) | Pending до пилота (human 1) |

Все три ID присутствуют в REQUIREMENTS.md и в планах; осиротевших нет. REQUIREMENTS.md не редактировался.

## Предупреждения ревью (42-REVIEW.md)

- WR-01 (check-link-layout не в CI): лишь частично смягчено. ci-fast собирает link-ui и запускает svelte-check, но `link-ui lint` (check-link-layout) нигде не вызывается и не входит в ui lint. Гейт верстки критерия 2 в CI не защищён. WARNING, не блокер: критерий 2 сегодня выполнен и проверен локально.
- WR-04 (вакуумный проход при отсутствии link-ui/src): гейт _eslint-workspaces молча пропускает каталог. Защиту критерия 3 не подрывает (его держит check-workspace-boundaries и сборка), но это дыра вакуума. WARNING.
- WR-02/WR-03 (гонка connectWs и непокрытая Tauri-ветка apiCall): относятся к десктопной ветке, которая вживую не проверена; вынесено в human 3.

Рекомендация: закрыть WR-01 и WR-04 одной правкой (добавить `pnpm --dir link-ui lint` шагом ci-fast, падать при отсутствии src) до фазы 43.

## Анти-паттерны

Маркеров TBD/FIXME/XXX без ссылки в проверенных файлах целенаправленно не искал сверх прогона privacy- и lint-гейтов (зелёные). Блокеров нет.

## Итог

Блокирующих расхождений нет: критерии 1 и 3 подтверждены кодом и прогонами, критерий 2 подтверждён по исходнику, критерий 4 по D-07 отложен. Статус human_needed из-за четырёх живых проверок выше; пилот блокирует планирование Фазы 45.

_Verifier: Claude (gsd-verifier)_
