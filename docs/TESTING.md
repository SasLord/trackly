# Тесты: раскладка и запуск

## Полный прогон

```bash
DEVELOPER_DIR=/Library/Developer/CommandLineTools TRACKLY_AD_MOCK=1 TRACKLY_SNMP_MOCK=1 \
  cargo test --workspace --no-fail-fast -- --test-threads=1
```

Нужен собранный `ui/dist` (`pnpm --dir ui build`). `DEVELOPER_DIR` нужен на macOS,
пока не принята лицензия Xcode (иначе линковка падает с exit 69).

## Suite-бинарники

В `trackly-app` и `trackly-infra` стоит `autotests = false`. Интеграционные тесты
собраны не по бинарнику на файл, а в несколько suite: корень `tests/suites/<suite>.rs`
подключает файлы через `#[path = "../<файл>.rs"] mod <файл>;`. Сами файлы лежат на
прежних местах в `tests/` — гейты, читающие их по путям (`entities_changed_gate`,
`group_write_sites`, `include_str!` и т.п.), работают как раньше.

| Цель `--test`     | Что внутри                                                      |
|-------------------|-----------------------------------------------------------------|
| `app_acts`        | акты                                                            |
| `app_attachments` | вложения, резервные копии, реквизиты организации                |
| `app_auth`        | вход, AD, пользователи, матрица ролей, заголовки безопасности   |
| `app_cartridges`  | картриджи, принтеры, дашборд, конкурентные записи               |
| `app_devices`     | устройства                                                      |
| `app_gates`       | гейты-сканеры исходников и реестры, `suite_wiring`              |
| `app_groups`      | группы и типы групп                                             |
| `app_places`      | места и журнал перемещений                                      |
| `app_repairs`     | ремонтные заявки                                                |
| `app_reports`     | отчёты, HTML/PDF-рендер, шаблоны номеров                        |
| `app_server`      | сервер, WebSocket, TLS, заявки                                  |
| `infra_tests`     | все интеграционные тесты `trackly-infra`                        |

Отдельными целями (своим процессом) остались: `export_bindings` и `attachments_heic`
(их зовут по имени `ui/package.json` и `heic-spike.yml`), `health_smoke` (ставит
глобальный tracing-subscriber), `template_edit`, `templates_status`, `phase06_stubs`
(меняют переменные окружения процесса). `trackly-core` не менялся.

### Запуск одного файла

Имя теста теперь начинается с имени модуля-файла:

```bash
# было: cargo test -p trackly-app --test entities_changed_gate
cargo test -p trackly-app --test app_gates entities_changed_gate::
# было: cargo test -p trackly-infra --test config_test
cargo test -p trackly-infra --test infra_tests config_test::
```

### Новый файл тестов

Добавьте пару строк в подходящий `tests/suites/<suite>.rs` (алфавитный порядок):

```rust
#[path = "../my_new_test.rs"]
mod my_new_test;
```

Забытый файл не потеряется молча: гейт `suite_wiring` (в `app_gates`) требует, чтобы
каждый `tests/*.rs` обоих крейтов входил ровно в одну цель. Если новому файлу нужен
свой процесс (меняет env, ставит глобальный subscriber), объявите его отдельным
`[[test]]` в `Cargo.toml` крейта.

## Почему так (замеры, macOS M-серия)

- `[profile.test] debug = 0` (корневой `Cargo.toml`). С отладочной информацией dev-профиль
  на macOS оставляет `*.rcgu.o` в `target/debug/deps` (`split-debuginfo = "unpacked"`),
  и cargo их не удаляет: один полный прогон добавлял ~22–26 тыс. файлов. Когда их
  накопилось 1,9 млн, каждый вызов rustc/rustdoc тратил ~30 с на чтение каталога
  (против 0,5 с), и полный прогон шёл 8–10 часов.
- Каждый свежий бинарник macOS при первом запуске проверяет ~0,7 с (syspolicyd), и каждый
  линкует весь крейт. 20 бинарников вместо 228 — меньше линковок и проверок.

Если `target/debug/deps` уже раздут (проверка: `ls target/debug/deps | wc -l` идёт
минутами), один раз выполните `cargo clean`.
