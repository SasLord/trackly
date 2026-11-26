---
phase: 42
slug: link-monorepo-foundation
status: verified
threats_total: 59
threats_closed: 59
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-11-26
---

# Фаза 42 — Безопасность

> Контракт безопасности фазы: реестр угроз (59 записей из 19 PLAN-файлов), принятые риски, журнал аудита.
> Каждая угроза `mitigate` закрыта только по наличию контроля в коде, CI или артефакте (файл:строка).
> Проверочные прогоны (только чтение), 2026-11-26: `--selftest` и прогон без флага у
> `check-gate-coverage` (17 случаев; PASS: 316 файлов, 18 обходчиков), `check-workspace-boundaries`
> (18 случаев; PASS: 59 файлов пакетов, 5 файлов link-ui/src), `check-lint-selftest-wiring`,
> `check-tokens`, `check-html-commands`, `check-attachments-block` (26 мутантов),
> `check-modal-isolation` (13), `check-action-menu-portal` (29), `check-transport-hooks` (5),
> `check-transport-tauri` (9), `link-ui/scripts/check-link-layout.mjs` (34 признака),
> `scripts/check-release-isolation.mjs`, `scripts/check-privacy.mjs --hashes` (0 нарушений).
> `cargo test` аудитор не запускал: зелёный сплошной прогон взят из 42-19-SUMMARY (3083/0).

---

## Границы доверия

| Граница | Описание | Что пересекает |
|---------|----------|----------------|
| репозиторий -> публичный GitHub | baseline, ADR, пилотные скрипты, `.planning/` | только пути кода и вымышленные данные |
| стенд -> localhost | `stand_ui_move` на 127.0.0.1 с вымышленным админом | тестовые учётные данные |
| домен заказчика -> репозиторий | итог пилота, вывод `diag.ps1` | обезличенный JSON (счётчики, версии, флаги) |
| webview Link -> Rust (IPC) | команда `link_info`, плагины core/window-state | версия, путь установки |
| webview Link -> сеть | должна отсутствовать (CSP, гейт [L4]) | — |
| `packages/*` -> домен Trackly | пакеты не тянут bindings/auth/features, `@tauri-apps/*` только по allowlist | код, привилегированный API |
| UI -> backend (`apiCall`) | реакции на 401/403 внедряются хуками | сессия |
| `workflow_dispatch`/тег -> shell | ввод `version` в release-link.yml | строка версии |
| release-link.yml -> GitHub Releases | публикации нет, только артефакт | установщик NSIS |
| npm / crates.io -> сборка | lockfile; новая зависимость `tauri-plugin-window-state` | цепочка поставок |

---

## Реестр угроз

*Статус: open · closed. Disposition: mitigate · accept · transfer.*

| Threat ID | Категория | Компонент | Disposition | Mitigation / доказательство | Статус |
|-----------|-----------|-----------|-------------|-----------------------------|--------|
| T-42-01-01 | Information Disclosure | gate-coverage.baseline.json | mitigate | ключи файла только `files/totals/perGate/pinned`, все `files` — пути кода (`ui/...`, один `packages/transport/src/client.ts`); `scripts/check-privacy.mjs --hashes` PASS; в CI `.github/workflows/ci-fast.yml:31`, локально `.githooks/pre-commit` | closed |
| T-42-01-02 | Tampering | check-gate-coverage (вакуумный PASS) | mitigate | `ui/scripts/check-gate-coverage.mjs:60-61` ([C0] пустой baseline/скан), selftest 17 случаев; в lint парой: `ui/package.json:16` (`check-gate-coverage.mjs --selftest && check-gate-coverage.mjs`) | closed |
| T-42-01-03 | Denial of Service | snapshot-gate-coverage.mjs | accept | см. AR-01 | closed |
| T-42-02-01 | Information Disclosure | 42-STAND-BASELINE.md | mitigate | данные стенда вымышленные (`stand_admin`, `crates/trackly-app/tests/stand_ui_move.rs:19-20`); privacy-гейт PASS (режим хэшей покрывает `.md`); ручной греп артефактов фазы на ФИО/реквизиты/IP/домены — только «Иванов И.И.»/«Петров П.П.» и 127.0.0.1 | closed |
| T-42-02-02 | Elevation | адрес стенда | mitigate | `stand_ui_move.rs:59` `([127, 0, 0, 1], port)`, сертификат на 127.0.0.1 (`:60`), предел `STAND_MAX_SECS` по умолчанию 3600 (`:73-76`, цикл `:84`), `#[ignore]` (`:23`) | closed |
| T-42-02-03 | Tampering | suite_wiring | mitigate | отдельный `[[test]] name = "stand_ui_move"` — `crates/trackly-app/Cargo.toml:137-138` при `autotests = false` (`:12`); гейт `tests/suite_wiring.rs` сверяет цели манифеста; зелёный по 42-02-SUMMARY (5/5) и сплошному прогону 42-19 | closed |
| T-42-03-01 | Information Disclosure | docs/adr/*.md | mitigate | privacy-гейт PASS; ручной греп ADR: имён ПК, доменов, IP, реквизитов, ФИО нет | closed |
| T-42-03-02 | Repudiation | статусы ADR | accept | см. AR-02 | closed |
| T-42-04-01 | Information Disclosure | diag.ps1, RESULT.md | mitigate | `docs/pilot/diag.ps1`: в JSON только версии/флаги/счётчики (`pathKind` вместо пути `:57`, `sessionsCount` без имён `:161`, события — только `id`+`count` `:109,142`); Subject подписи только на экран (`:91`); `RESULT-TEMPLATE.md:3` запрещает идентификаторы; `RESULT.md` — заглушка «ОТЛОЖЕНО» | closed |
| T-42-04-02 | Tampering | ключи подписи | mitigate | `git ls-files` без `.pfx/.p12/.key/.pem/.cer`; в `release-link.yml` нет подписи и `secrets.*`; `docs/pilot/sign-notes.md:30` запрещает PFX/ключи в репо и CI | closed |
| T-42-04-03 | Elevation | install-silent.ps1 | accept | см. AR-03 | closed |
| T-42-05-01 | Tampering | правила гейтов при правке обхода | mitigate | `perGate` baseline заморожен в e7932133 и с тех пор не менялся (сверено программно); selftest всех перенацеленных гейтов зелёный; [C5] `check-gate-coverage.mjs:88-91` | closed |
| T-42-05-02 | Repudiation | вакуумный PASS | mitigate | `ui/scripts/_roots.mjs:68` пишет `GATE_COVERAGE_REPORT`; `check-gate-coverage.mjs:90` «count > 0», `:159` запуск с отчётом | closed |
| T-42-06-01 | Tampering | ослабление правил гейта | mitigate | как T-42-05-01; [C4] `check-gate-coverage.mjs:79-84` — множество обходчиков равно ключам `perGate` | closed |
| T-42-06-02 | Repudiation | вакуумный PASS check-tokens | mitigate | `tokens: 226` в `perGate`; `check-tokens.mjs:27,69` обход через `walk()`; [C5] count > 0 и ≥ baseline | closed |
| T-42-07-01 | Tampering | снижение perGate | mitigate | [C5] `check-gate-coverage.mjs:91` (count < baseline -> красный); `perGate` не менялся после заморозки; правки `pinned` (см. наблюдение N-3) видны в `git log -p` с обоснованием в SUMMARY | closed |
| T-42-07-02 | Denial of Service | долгий прогон гейта | accept | см. AR-04 | closed |
| T-42-08-SC | Tampering | npm supply chain | mitigate | множество пакетов в корневом `pnpm-lock.yaml` совпадает с `ui/pnpm-lock.yaml` до фазы (288 = 288, добавлено 0, удалено 0); `pnpm install --frozen-lockfile` — `ci-fast.yml:88`, `ci-full.yml:85`, `release-link.yml` | closed |
| T-42-08-01 | Tampering | release.yml | mitigate | за фазу release.yml менялся дважды: d078b1f4 (кеш-путь `pnpm-lock.yaml`, установка из корня) и 4f0a1d99 (снят пин `version: 10`); триггеры и инъекция версии не тронуты; гейт `scripts/check-release-isolation.mjs` [I1]/[I6] в `ci-fast.yml:41` | closed |
| T-42-08-02 | Denial of Service | ci без link-ui/dist | mitigate | шаг «Seed link-ui/dist placeholder» — `ci-fast.yml:103-107`, `ci-full.yml:100-104` и `:192-196` | closed |
| T-42-09-01 | Tampering | кодмод портит импорты | mitigate | `ui/scripts/_move-codemod.mjs:91-103,133` (`--apply` сам вызывает `classify` и отказывает); svelte-check `ci-fast.yml:142-145` (tsconfig включает `packages/*/src`, `link-ui/src`), сборка `ci-fast.yml:112-114` | closed |
| T-42-09-02 | Elevation | `@tauri-apps/*` в пакетах | mitigate | [B2] `check-workspace-boundaries.mjs:131`, allowlist `:60-71`; фактические импорты совпадают с allowlist | closed |
| T-42-09-03 | Repudiation | потеря покрытия при переезде | mitigate | move-map 61 запись, [C1]/[C2] `check-gate-coverage.mjs:66-70`; PASS | closed |
| T-42-10-SC | Tampering | tauri-plugin-window-state | mitigate | `Cargo.toml:86` `= "2.5.0"`; `Cargo.lock:6992-6995` (2.5.0, crates.io, checksum); `THIRD-PARTY-NOTICES.md:25-28`; `42-RESEARCH.md:113` Package Legitimacy Audit | closed |
| T-42-10-01 | Elevation | capabilities webview | mitigate | `crates/link-app/capabilities/main.json`: `core:default`, `window-state:default`, `core:window:allow-set-background-color`; shell/fs/dialog/notification нет; точный набор держит [L3] `link-ui/scripts/check-link-layout.mjs:117`. Отклонение от формулировки плана — см. N-1 | closed |
| T-42-10-02 | Tampering | XSS/внешние ресурсы | mitigate | `crates/link-app/tauri.conf.json:27` CSP `default-src 'self'; script-src 'self'; connect-src 'self' ipc: http://ipc.localhost`; внешних источников нет. Отклонения (`'self'` в connect-src, `style-src 'unsafe-inline'`) — см. N-2 | closed |
| T-42-10-03 | Information Disclosure | link_info и путь установки | accept | см. AR-05 | closed |
| T-42-10-04 | Tampering | инъекция версии в Link | mitigate | `crates/link-app/Cargo.toml:4` литеральная `version = "0.1.0"`; [B5] `check-workspace-boundaries.mjs:192-194`; [I6] не пускает `link-app` в release.yml | closed |
| T-42-11-01 | Tampering | CSP и inline-скрипт | mitigate | `packages/design/vite-plugin-theme-init.mjs` — `emitFile` asset, не инлайн; `ui/index.html:9`, `link-ui/index.html:9` `<script src="/theme-init.js">`; собранные `ui/dist`, `link-ui/dist` без inline-скриптов | closed |
| T-42-11-02 | Repudiation | check-tokens не видит токены | mitigate | путь `packages/design/src/styles/_tokens.scss` — `check-tokens.mjs:239`; при отсутствии файла множество определений пусто, и каждый `var(--tr-*)` становится нарушением (`:180-199`) | closed |
| T-42-11-03 | Denial of Service | сборка без ui/public | accept | см. AR-06 | closed |
| T-42-12-01 | Repudiation | гейт зелёный на заглушке | mitigate | `ui/scripts/_rune-harness.mjs:58-68` резолвит `@trackly/*` в `packages/*/src`; ручные мутации реальных исходников с уникальным якорем описаны в 42-12/13/14-SUMMARY | closed |
| T-42-12-02 | Elevation | saveFile и plugin-* | mitigate | `check-workspace-boundaries.mjs:57-61` (`packages/kit/src/utils/saveFile.ts` с обоснованием) | closed |
| T-42-12-03 | Tampering | кодмод переписывает чужие строки | mitigate | svelte-check, сборка ui, `check-gate-coverage` в CI (`ci-fast.yml:112-149`) | closed |
| T-42-13-01 | Repudiation | устаревший путь / неуникальный якорь | mitigate | `check-modal-isolation.mjs:42` путь пакета; selftest 13 мутантов; `check-action-menu-portal` 29 фикстур; [C7] `pinned` | closed |
| T-42-13-02 | Tampering | потеря modal-isolation/portal | mitigate | оба гейта парами в `ui/package.json:16`; selftest и прогон зелёные | closed |
| T-42-14-01 | Information Disclosure | домен утекает в kit/Link | mitigate | `_move-codemod.mjs:100-103` (bindings/auth/features); [B1] `check-workspace-boundaries.mjs:128`; [B4] `:158-179` для link-ui; греп `packages/*/src` на `$lib`/bindings/auth/features — 0 | closed |
| T-42-14-02 | Repudiation | гейт потерял покрытие | mitigate | [C5] per-gate ≥ baseline, PASS | closed |
| T-42-15-01 | Elevation | потеря реакции на 401/403 | mitigate | `ui/src/main.ts:11-19` `setApiHooks` до `mount` (`:35`); [T5] `check-transport-hooks.mjs:17,84`; Tauri-ветка `check-transport-tauri`; стенд 42-STAND-RESULT: 401 -> `#/login` подтверждён, 403 на стенде не воспроизведён (закрыт поведенческими гейтами, UAT п.6 со слов пользователя) | closed |
| T-42-15-02 | Tampering | дрейф WsEvent | mitigate | `ui/src/packages-compat.ts:4-10` взаимная присваиваемость; `ui/tsconfig.json:26-33`; svelte-check `--fail-on-warnings` в `ci-fast.yml:145` | closed |
| T-42-15-03 | Information Disclosure | пакет тянет auth/toast | mitigate | [B1] запрещает auth/bindings/features; `client.ts` без auth/toast (хуки). Остаток: `ws.ts` лениво импортирует `@trackly/kit/stores/toast.svelte` (kit — не домен), см. 42-15-SUMMARY отступление 1 | closed |
| T-42-15-04 | Repudiation | HTML_COMMANDS не парсится | mitigate | `packages/transport/src/client.ts:29`; [H0] `check-html-commands.mjs:72,104`; мутант «объявление переименовано» пойман | closed |
| T-42-16-01 | Tampering | XSS через служебную строку | mitigate | `link-ui/src/components/StatusLine.svelte` — `{text}`; [L4] `check-link-layout.mjs:127-129` запрещает `{@html` | closed |
| T-42-16-02 | Elevation | сеть из webview | mitigate | [L4] `check-link-layout.mjs:127` (fetch/XMLHttpRequest/WebSocket); CSP `tauri.conf.json:27`; в link-ui/src совпадений нет | closed |
| T-42-16-03 | Information Disclosure | путь в буфере обмена | accept | см. AR-07 | closed |
| T-42-16-04 | Repudiation | вакуумный гейт раскладки | mitigate | [L0] `check-link-layout.mjs:69`, мутант `:212-216`; в lint `link-ui/package.json:11`, в CI `ci-fast.yml:155-156`, `ci-full.yml:154-156`; подключение держит [B6] | closed |
| T-42-17-01 | Elevation | права вложений без auth | mitigate | `packages/attachments/src/host.ts:14` по умолчанию `currentUser: () => null`; `ui/src/main.ts:24-29` регистрация до mount; серверная авторизация не менялась. Регистрация хоста гейтом не защищена (REVIEW IN-02) | closed |
| T-42-17-02 | Elevation | `@tauri-apps/api/core` в пакете | mitigate | allowlist `check-workspace-boundaries.mjs:68` — только `packages/attachments/src/api.ts`; фактические импорты (`api.ts:10,97`) совпадают | closed |
| T-42-17-03 | Tampering | дрейф DTO | mitigate | `ui/src/packages-compat.ts:12-41` (5 DTO) + svelte-check в CI | closed |
| T-42-17-04 | Repudiation | check-attachments-block теряет охват | mitigate | `check-attachments-block.mjs:55` `DIR` = `packages/attachments/src`; [G0] на пропажу файла; 26 мутантов; `pinned` перенацелен | closed |
| T-42-18-01 | Tampering | shell-инъекция через version | mitigate | `release-link.yml`: `INPUT_VERSION` через `env:`, regex-проверка до использования, эхо `grep -Fq` и `jq` точной строкой | closed |
| T-42-18-02 | Tampering | выпуск Link перезаписывает Trackly | mitigate | теги `link-v*.*.*` и guard `if:`; нет `tagName/releaseId`; `scripts/check-release-isolation.mjs:98-139` [I0]-[I6], в `ci-fast.yml:41`; `gh release list` — релизов Link нет | closed |
| T-42-18-03 | Elevation | права GITHUB_TOKEN | mitigate | `release-link.yml` `permissions: contents: read`; [I4] `check-release-isolation.mjs:130` | closed |
| T-42-18-04 | Information Disclosure | артефакт/логи с адресами | mitigate | шаг «Summarize installer» выводит только имя файла и sha256; адресов/отпечатков в workflow нет | closed |
| T-42-18-05 | Tampering | ключи подписи | accept | см. AR-08 | closed |
| T-42-19-01 | Information Disclosure | STAND-RESULT / HUMAN-UAT | mitigate | вымышленные данные стенда; HUMAN-UAT обезличен; privacy-гейт PASS; ручной греп чист | closed |
| T-42-19-02 | Tampering | случайный запуск release.yml | mitigate | dry-run только `release-link.yml` (42-STAND-RESULT, попытка 3); `gh release list` — только черновики Trackly; позднейшие запуски release.yml — отдельные ручные `workflow_dispatch`, см. N-4 | closed |
| T-42-19-03 | Repudiation | «зелёный» без доказательств | mitigate | 42-19-SUMMARY: команды и счётчики (3083/0), раздел UNVERIFIED (`:54`); 42-HUMAN-UAT фиксирует отложенное | closed |
| T-42-19-04 | Denial of Service | параллельный cargo test | mitigate | 42-19-SUMMARY:45 — один сплошной прогон в фоне | closed |

---

## Журнал принятых рисков

| Risk ID | Threat Ref | Обоснование (проверено по коду) | Принял | Дата |
|---------|------------|----------------------------------|--------|------|
| AR-01 | T-42-01-03 | Шапка `ui/scripts/snapshot-gate-coverage.mjs:4-5` запрещает повторный запуск после начала переездов, а `:111` отказывает в пересъёме `--per-gate` при непустом move-map; перезапись baseline видна в git diff, `perGate` после заморозки не менялся. | владелец проекта (план 42-01) | 2026-11-26 |
| AR-02 | T-42-03-02 | ADR, зависящие от пилота, помечены `Accepted (provisional)`: `docs/adr/0004-updates-distribution-rds.md:4`, `docs/adr/0005-install-path-and-user-data.md:4`. Остальные (0001-0003) от пилота не зависят. | владелец проекта (план 42-03) | 2026-11-26 |
| AR-03 | T-42-04-03 | `docs/pilot/install-silent.ps1:4-5,18,20` — цель `%LOCALAPPDATA%\Programs`, ключ удаления в HKCU, повышения прав нет; NSIS `installMode: currentUser` (`crates/link-app/tauri.conf.json`). AppLocker не обходится (D-04). | владелец проекта (план 42-04) | 2026-11-26 |
| AR-04 | T-42-07-02 | Каждый дочерний гейт запускается с `timeout: 120000` — `ui/scripts/check-gate-coverage.mjs:161`. | владелец проекта (план 42-07) | 2026-11-26 |
| AR-05 | T-42-10-03 | `crates/link-app/src/main.rs` не содержит логирования (нет `tracing`/`log`/`println!`); в `link-ui/src` нет `console.*`; строка видна только в `StatusLine`. | владелец проекта (план 42-10) | 2026-11-26 |
| AR-06 | T-42-11-03 | `ui/public` удалён; `theme-init.js` выпускается плагином в `ui/dist` и `link-ui/dist` (проверено по содержимому dist); сборка ui в `ci-fast.yml:112-114`. | владелец проекта (план 42-11) | 2026-11-26 |
| AR-07 | T-42-16-03 | Копирование только кнопкой «Скопировать диагностику» (`link-ui/src/components/StatusLine.svelte`, `navigator.clipboard.writeText` в обработчике `onclick`); автоматической записи нет. | владелец проекта (план 42-16) | 2026-11-26 |
| AR-08 | T-42-18-05 | В `release-link.yml` нет подписи и обращений к `secrets.*`; подпись — Фаза 45. | владелец проекта (план 42-18) | 2026-11-26 |

*Принятые риски не всплывают в последующих аудитах.*

---

## Наблюдения (не блокеры, без новой угрозы)

| ID | Связь | Наблюдение |
|----|-------|------------|
| N-1 | T-42-10-01 | План объявлял «только `core:default` и `window-state:default`». После фазы quick-задача 261126-rbg добавила `core:window:allow-set-background-color`; гейт [L3] обновлён на точный набор из трёх. Право не даёт доступа к файлам, сети или процессам; привилегированных плагинов нет. Формулировку в реестре стоит обновить при следующем планировании. |
| N-2 | T-42-10-02 | В CSP `connect-src` есть `'self'` помимо ipc (своё происхождение приложения, не сеть), а `style-src` содержит `'unsafe-inline'` (REVIEW IN-04). Внешние источники закрыты; на ASVS L1 не блокер. |
| N-3 | T-42-07-01 / T-42-09-03 | Счётчики `pinned` ([C7]) снижались при переездах (`place-tree-invalidation`: 309 -> 250, `attachments-block` перенацелен). Механизм разрешён планом, каждое снижение — в коммите переезда с обоснованием (напр. 42-14-SUMMARY:46). Гейт `place-tree-invalidation` по-прежнему обходит только `ui/src`; доменные write-site'ы остались в `ui/src/features`, перенесённые файлы их не содержат (classify «чисто»). Защищает `perGate` ([C5]), он не менялся. |
| N-4 | T-42-19-02 | Через `gh run list` видны два запуска `release.yml` за 2026-11-26 (`workflow_dispatch`, вскоре после dry-run Link) и черновик релиза Trackly. Это отдельные ручные запуски, не следствие release-link.yml; опубликованных (не черновых) релизов нет. 42-STAND-RESULT фиксирует состояние на момент dry-run. |
| N-5 | T-42-09-02 | В allowlist после фазы добавлен `packages/kit/src/stores/theme.svelte.ts` (quick 261126-rbg) с обоснованием в гейте; импорт динамический, за guard'ом `__TAURI_INTERNALS__`. |

Флагов угроз (`## Threat Flags`) в SUMMARY-файлах фазы нет; незарегистрированных флагов нет.

Проверка приватности артефактов фазы: реальных данных организации и людей в `.planning/phases/42-*`, `docs/adr/`, `docs/pilot/` не найдено (ФИО только «Иванов И.И.»/«Петров П.П.», IP только 127.0.0.1, сертификат только «Link Pilot Test»).

---

## Журнал аудита безопасности

| Дата аудита | Угроз всего | Закрыто | Открыто | Кто проводил |
|-------------|-------------|---------|---------|--------------|
| 2026-11-26 | 59 | 59 | 0 | gsd-security-auditor (Claude) |

---

## Подпись

- [x] У всех угроз есть disposition (mitigate / accept / transfer)
- [x] Принятые риски внесены в журнал принятых рисков
- [x] `threats_open: 0` подтверждено
- [x] `status: verified` в frontmatter

**Утверждение:** verified 2026-11-26
