---
phase: 44-link
plan: 17
subsystem: link-client
tags: [link-app, commands, runtime, tray, single-instance, autostart, window-policy]
requires: [44-09, 44-10, 44-12, 44-15]
provides:
  - commands.rs (8 команд link_*, COMMAND_NAMES, precheck_invite, LoginInput)
  - runtime.rs (Runtime, TauriSink, Session, AppControl, should_auto_show, tray_handlers)
  - main.rs (Builder: плагины, setup, трей, окно, автозапуск, RunEvent)
  - tests/commands_contract.rs
affects: [44-20]
key-files:
  modified:
    - crates/link-app/src/commands.rs
    - crates/link-app/src/runtime.rs
    - crates/link-app/src/main.rs
  created:
    - crates/link-app/tests/commands_contract.rs
decisions:
  - "Runtime хранит Option<Session {cfg, http: Arc<LinkHttp>, handle: Arc<SupervisorHandle>}>: команды входа и RealTransport используют ОДИН LinkHttp (cookie-сессия), TokenStore и Mutex<LocalState>"
  - "Автомат создаётся только при наличии конфига; после enroll сессия стартует заново, а sink публикует Starting, чтобы UI ушёл с экрана настройки"
  - "Решение о показе окна: should_auto_show(prev, next) = состояние требует действия И вид состояния сменился; Connected -> Offline окно не открывает"
  - "capabilities и tauri.conf.json не менялись: команды приложения не требуют разрешений, событие link-event покрыто core:default; гейт L3 цел"
requirements-completed: [LNK-02, LNK-03, LNK-04, LNK-05, LNK-06, LNK-07, LNK-11]
completed: 2026-11-28
---

# Phase 44 Plan 17: сборка клиента Link Summary

Link стал приложением: 8 команд webview над общим `Runtime`, реальный приёмник событий (`link-event`, `trackly-event`, `trackly-resync`, трей, показ окна), единственный экземпляр на сеанс, автозапуск при каждом старте, закрытие окна прячет в трей, выход только из меню трея или `link_quit`.

## Коммиты

- d903ff43: commands.rs, runtime.rs, tests/commands_contract.rs
- a2f856b4: main.rs

## Гейты

| Гейт | Результат |
|------|-----------|
| `cargo test -p link-app --test commands_contract` | 9/9 |
| `cargo test -p link-app` целиком | 48 lib + 2 + 17 + 9 + 21 + 7 + 17 + 5, 0 упавших |
| `cargo build -p link-app` | чисто |
| `cargo clippy -p link-app --all-targets -- -D warnings` | чисто |
| `cargo fmt --check -p link-app` | чисто |
| `pnpm --dir link-ui lint` | зелёный (link-ui не менялся; гейты L2/L3 и check-link-state целы) |
| грепы приёмки: `process::exit` в src пусто; `password_login(` один вызов в commands.rs; StateFlags::VISIBLE, single_instance, set_skip_taskbar (вне ветвления по трею), ensure_enabled, ActivationPolicy::Accessory найдены в main.rs | выполнены |

`pnpm --dir ui lint` не гонялся: ни ui, ни link-ui не правились.

## Согласование с планом 10 (UI)

- Имена аргументов команд `code`, `login`, `password` совпали с `linkApi.ts`: правки UI не нужны. `link_get_state` возвращает `LinkEvent`; остальные команды `()`.
- `pinDetails` в `SetupScreen` НЕ доставляется: ошибка `pin_mismatch` команды `link_submit_invite` несёт только текст, `EnrollError::PinMismatch{expected, got}` на границе команды теряется. UI показывает danger-callout без сведений для администратора. Если сведения нужны, это правка контракта `LinkCmdError` либо отдельного события (решение за планом 20 или отдельным quick).

## Отклонения от плана

1. **[Rule 3] Контрактный тест сверяет `generate_handler!` по исходнику `main.rs`** (include_str), потому что макрос недоступен снаружи бинарника; пути в списке записаны как `link_app::commands::link_*`, тест отбрасывает префикс.
2. **[Rule 3] `Runtime::start` вызывает `tauri::async_runtime::block_on`** вокруг запуска автомата: `setup` синхронный, `tokio::spawn` без контекста паникует.
3. **Трей собирается ДО `Runtime::start`** с состоянием Starting (чтобы `tray_available` попало в `EventMeta`); обработчики меню берут `Runtime` через `try_state`.
4. Два коммита вместо «по задаче»: контрактный тест читает `main.rs`, поэтому до второго коммита он не зелёный.
5. Test 4 плана («команды не возвращают секреты») реализован как проверки Debug `LoginInput`, отсутствия `tracing::` в commands.rs и единственности вызова `password_login(`.

## Не проверено (живая проверка в плане 20)

- Реальное поведение трея, показа окна по D-18, single-instance (в том числе RDS без `Global\`), автозапуска, Accessory на macOS, окна без трея на Linux.
- `ExitRequested` при завершении сеанса Windows (T-44-66): логика «код None блокируется» не проверена на выходе из сеанса.
- Битый или невалидный конфиг трактуется как `NeedsSetup` (с записью причины в лог); при машинном конфиге с ошибкой ввод кода вернёт `Internal`. Это поведение стоит осознанно подтвердить.
- В link-app нет инициализации `tracing-subscriber`: логи пока никуда не выводятся (вне объёма плана).
- Гонка «сначала Starting, потом внешнее событие» после enroll не покрыта интеграционным тестом (нужен Tauri-рантайм).

## Авто-одобренные чекпоинты

Нет (план автономный). Для человека на стенде: запустить Link без конфига (окно открывается само на экране кода), ввести код (экран входа), войти (окно скрыто только после ручного закрытия, трей зелёный); закрыть окно крестиком (уходит в трей); запустить второй экземпляр (окно первого выходит вперёд); «Закрыть» в меню трея (процесс завершается); на Linux без AppIndicator окно обычное и сворачивается.

## Known Stubs

Нет.

## Threat Flags

Новых поверхностей вне threat_model нет. T-44-65: capabilities не менялись, контрактный тест фиксирует набор команд. T-44-66: явный выход проходит. T-44-67: живая проверка в плане 20. T-44-68: пароль принимается как `String` на границе IPC и сразу оборачивается в `Secret`.

## Self-Check: PASSED

Файлы commands.rs, runtime.rs, main.rs, tests/commands_contract.rs существуют; коммиты d903ff43 и a2f856b4 в истории.
