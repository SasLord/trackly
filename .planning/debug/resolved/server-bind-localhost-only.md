---
slug: server-bind-localhost-only
status: resolved
trigger: "На платформе Windows запускаю сервер, работает только на localhost. Пробовал в Настройках указать bind 0.0.0.0 - нет доступа к этому серверу. Раньше в тестовой сборке v0.1.0-test работало корректно."
created: 2026-06-29
updated: 2026-06-29
---

# Debug Session: server-bind-localhost-only

## Symptoms

- **Expected behavior:** В режиме сервера приложение должно быть доступно из локальной сети по `https://<IP-Windows>:<порт>`, чтобы сотрудники подключались через браузер.
- **Actual behavior:** Сервер на Windows доступен только с самой машины (localhost). При указании в Настройках bind `0.0.0.0` доступ снаружи отсутствует.
- **Симптом снаружи:** При заходе с другого компьютера по `https://<IP-Windows>:<порт>` — **connection refused / timeout** (соединение отклонено или зависает).
- **Протокол/порт:** HTTPS, тот же порт, что и в рабочей сборке `v0.1.0-test`.
- **Localhost при 0.0.0.0:** Не подтверждён (пользователь не смог проверить; `cargo tauri dev` на dev-маке завис на этапе `Building trackly-infra` — отдельная сборочная заминка, НЕ предмет данного бага).
- **Timeline:** Раньше корректно работало в тестовой сборке `v0.1.0-test`. Это регрессия.

## Key facts established by orchestrator

- `v0.1.0-test` НЕ существует среди git-тегов репозитория (есть только `v1.1` и `v1.1.0`). «Рабочая сборка» — отдельный локальный артефакт, не текущий тег. Сравнение «что сломалось» нужно вести по истории кода резолва bind-адреса, а не по diff с тегом.
- Дефолт bind-host: `127.0.0.1` ([crates/trackly-infra/src/config.rs:52,69](crates/trackly-infra/src/config.rs:52)).
- Резолв адреса при старте: `format!("{host}:{port}").parse::<SocketAddr>()` → `TcpListener::bind(addr)` ([crates/trackly-app/src/main.rs:176](crates/trackly-app/src/main.rs:176)).
- Hot-toggle сети (POST /api/v1/settings_set_network) повторно парсит и биндит листенер ([crates/trackly-app/src/http/settings.rs:207-227](crates/trackly-app/src/http/settings.rs:207)).
- Self-signed cert генерируется по `host` ([crates/trackly-app/src/main.rs:157](crates/trackly-app/src/main.rs:157), [crates/trackly-app/src/http/settings.rs:193](crates/trackly-app/src/http/settings.rs:193)) — при `host=0.0.0.0` это кандидат на сбой/некорректный SAN.

## Current Focus

ROOT CAUSE CONFIRMED — wiring gap, not firewall, not cert, not TOML regression.

reasoning_checkpoint:
  hypothesis: "Settings UI host change to 0.0.0.0 has zero effect on the bind address because settings_set_network only WRITES server_host/server_port to the app_settings table, but NO code path ever READS those keys back. The server (both startup in main.rs and the server_toggle path) binds using ctx.config.server.host, which is loaded exclusively from trackly.config.toml (default 127.0.0.1) and never merged with app_settings overrides. So the listener always binds 127.0.0.1 regardless of the saved UI value → reachable only from localhost."
  confirming_evidence:
    - "settings.rs:130-153 (HTTP) + tauri_cmds/auth.rs:274-296 (Tauri) both upsert server_host/server_port into app_settings; grep across crates/ shows app_settings READS only for low_stock_threshold, ad_enabled, ad_auto_accept, desktop_lock_enabled, backup_folder — server_host/server_port are NEVER read back."
    - "build_server_toggle (settings.rs:165,192-217) reads config = &ctx.config.server; binds format!(\"{}:{}\", host, port) from ctx.config — not from app_settings."
    - "main.rs:143-176 startup path binds from ctx.config.server.host/port (TOML) only."
    - "context.rs:127-137 AppCtx::build stores the TOML-loaded AppConfig directly into ctx.config (Arc<AppConfig>); no app_settings merge for host/port."
    - "tauri_cmds/auth.rs:313 comment literally states: host/port/cert_path — из стартового config (D-Desktop-02)."
    - "git log on settings.rs: feature added in 2a88cd9 wrote app_settings but never wired the read — never worked via UI; v0.1.0-test 'worked' because its trackly.config.toml had host bound externally directly."
  falsification_test: "If the hypothesis were false, there would exist a code path reading server_host/server_port from app_settings and applying it to the bind addr. grep finds none. Additionally: on the Windows box, with UI host=0.0.0.0 and server running, `netstat -ano | findstr :PORT` would show 0.0.0.0:PORT if false — hypothesis predicts it shows 127.0.0.1:PORT."
  fix_rationale: "Make the bind paths read the persisted app_settings.server_host/server_port (live source of truth) instead of (or layered over) the TOML bootstrap value. This addresses the root cause: the UI write now actually reaches TcpListener::bind. Firewall/cert are downstream and irrelevant until the socket binds the external interface."
  blind_spots: "Cannot live-reproduce on macOS (no Windows). Cannot rule out a SECONDARY firewall issue that would surface AFTER this fix lands (per-exe-path inbound rule). The netstat checkpoint on Windows confirms the bind-address mechanism; a follow-up firewall check may still be needed if LAN access still fails post-fix with 0.0.0.0:PORT correctly listening."
- **next_action:** Implement fix — read server_host/server_port/server_cert_path from app_settings at the point of bind (startup + server_toggle), falling back to ctx.config (TOML) when the key is absent. Then verify via Windows-side netstat checkpoint.

## Evidence

- timestamp: 2026-06-29 — Симптом снаружи: connection refused/timeout (не cert-ошибка). HTTPS, тот же порт что в рабочей v0.1.0-test. (источник: пользователь)
- timestamp: 2026-06-29 — `v0.1.0-test` отсутствует среди git-тегов (`git tag -l` → только v1.1, v1.1.0). (источник: orchestrator)
- timestamp: 2026-06-29 — checked: grep `server_host`/`server_port` across crates/. found: записываются в `app_settings` в двух местах (http/settings.rs:137-143, tauri_cmds/auth.rs:281-287), но НИ ОДНОГО чтения этих ключей обратно. Все READ из app_settings — только low_stock_threshold/ad_enabled/ad_auto_accept/desktop_lock_enabled/backup_folder. implication: сохранённый в UI host=0.0.0.0 никогда не доходит до bind.
- timestamp: 2026-06-29 — checked: build_server_toggle (http/settings.rs:165,192-217) и startup (main.rs:143-176). found: оба биндят из `ctx.config.server.host` (TOML, дефолт 127.0.0.1), не из app_settings. implication: даже явный start/stop из UI слушает 127.0.0.1.
- timestamp: 2026-06-29 — checked: context.rs:127-137 AppCtx::build. found: TOML-config кладётся в ctx.config как есть, без merge app_settings для host/port. implication: единственный источник bind-адреса — trackly.config.toml.
- timestamp: 2026-06-29 — checked: tauri_cmds/auth.rs:313 комментарий. found: «host/port/cert_path — из стартового config (D-Desktop-02)» — намеренно read-only из TOML. implication: подтверждает wiring-gap.
- timestamp: 2026-06-29 — checked: UI ui/src/features/settings/NetworkSettings.svelte. found: saveSettings()→settings_set_network (disabled while running, line 218); toggleServer(true)→server_toggle. Между сохранением и стартом нет передачи host в bind. implication: цепочка UI→app_settings→bind разорвана.

## Eliminated

- hypothesis: Windows Firewall блокирует входящие на порт для нового exe-пути.
  evidence: листенер вообще не биндится на внешний интерфейс — он всегда слушает 127.0.0.1 (bind-адрес берётся из TOML-config 127.0.0.1, а не из сохранённого app_settings 0.0.0.0). Firewall нерелевантен, пока сокет не открыт на внешнем интерфейсе. (Может всплыть как ВТОРИЧНАЯ проблема ПОСЛЕ фикса — отмечено в blind_spots.)
  timestamp: 2026-06-29
- hypothesis: при смене host на 0.0.0.0 пересоздание cert/листенера падает и откатывается на 127.0.0.1.
  evidence: никакого пересоздания при сохранении не происходит вообще — settings_set_network только пишет в БД и не трогает листенер. server_toggle создаёт листенер, но из config.host (127.0.0.1), а не из сохранённого значения. Откатываться не с чего — значение 0.0.0.0 никогда не читается.
  timestamp: 2026-06-29

## Resolution

root_cause: |
  Wiring-gap. `settings_set_network` (HTTP http/settings.rs:130-153 и Tauri tauri_cmds/auth.rs:274-296) сохраняет
  `server_host`/`server_port`/`server_cert_path` в таблицу `app_settings`, но НИ ОДИН путь bind'а их не читал обратно.
  И старт сервера (main.rs:143-176), и оба hot-toggle (build_server_toggle в http/settings.rs, build_server_toggle_tauri
  в tauri_cmds/auth.rs) биндили из `ctx.config.server.host` — значения, загруженного исключительно из trackly.config.toml
  (дефолт 127.0.0.1, config.rs:69) и НЕ смёрженного с app_settings (context.rs:127-137). Поэтому выбранный в Настройках
  `0.0.0.0` никогда не доходил до TcpListener::bind: сокет всегда слушал 127.0.0.1 → доступ только с localhost, снаружи
  connection refused/timeout. Это объясняет, почему изменение host в UI давало нулевой эффект. v0.1.0-test «работал»,
  т.к. его trackly.config.toml, по-видимому, задавал внешний bind напрямую в TOML, минуя UI.

fix: |
  Добавлен резолвер `resolve_effective_network(ctx)` (http/settings.rs) — читает live `app_settings`
  (server_host/server_port/server_cert_path) поверх TOML-bootstrap `ctx.config.server`, с откатом на TOML при отсутствии/
  пустом значении ключа. Применён во всех точках bind и чтения:
    - main.rs (startup bind)
    - build_server_toggle (HTTP hot-toggle)
    - build_server_toggle_tauri (Tauri hot-toggle — путь, который реально использует десктоп-UI)
    - build_settings_get_network (HTTP) и build_settings_get_network_tauri (Tauri) — UI теперь показывает live-значение.
  Теперь сохранённый в Настройках host=0.0.0.0 реально доходит до TcpListener::bind.

verification: |
  - Регрессионный тест tests/server_bind_effective_host.rs (2 теста, PASS):
    * effective_network_prefers_saved_host_over_toml_default — до сохранения резолвер возвращает 127.0.0.1 (фиксирует баг),
      после сохранения server_host=0.0.0.0/server_port=9000 возвращает 0.0.0.0:9000.
    * effective_network_falls_back_on_blank_saved_host — пробельный сохранённый host откатывается на TOML-дефолт.
  - Адъяцентные тесты PASS без регрессий: server_hot_toggle, settings_ad, tls_server_smoke, security_headers.
  - cargo build -p trackly-app: OK. cargo clippy -p trackly-app --tests: без warnings.
  - HUMAN-VERIFIED 2026-06-30: пользователь скачал обновлённую сборку v1.1.0 (релиз пересобран с фиксом),
    подтвердил «теперь всё работает правильно» — bind на внешний интерфейс и LAN-доступ восстановлены.
    Вторичная проблема firewall не всплыла. RESOLVED.

files_changed:
  - crates/trackly-app/src/http/settings.rs (resolve_effective_network + применение в toggle/get_network)
  - crates/trackly-app/src/main.rs (startup bind использует resolve_effective_network)
  - crates/trackly-app/src/tauri_cmds/auth.rs (build_server_toggle_tauri + build_settings_get_network_tauri)
  - crates/trackly-app/tests/server_bind_effective_host.rs (новый регрессионный тест)
