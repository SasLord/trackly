# Phase 8: Релизный пайплайн — Pattern Map

**Mapped:** 2026-06-19
**Files analyzed:** 5 (новых/изменяемых)
**Analogs found:** 3 / 5

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `.github/workflows/release.yml` | config/ci | event-driven (tag push) | `.github/workflows/ci-full.yml` | role-match (setup steps verbatim) |
| `crates/trackly-app/tauri.conf.json` | config | — | self (текущий файл) | exact (modify in place) |
| `README.md` (корневой) | doc | — | none | no analog |
| `README-portable.md` | doc | — | none | no analog |
| `trackly.config.toml.example` | config template | — | none | no analog |

---

## Pattern Assignments

### `.github/workflows/release.yml` (config/ci, event-driven)

**Analog:** `.github/workflows/ci-full.yml`

**Критическое замечание — Rust toolchain версия:**
`ci-full.yml` и `ci-fast.yml` используют `toolchain: '1.88'`, но `rust-toolchain.toml` пинит `channel = "1.92.0"` (MSRV из `Cargo.toml`: `rust-version = "1.92"`). `release.yml` ДОЛЖЕН использовать `'1.92'`, а не `'1.88'`. Это также закрывает питфол из RESEARCH.md о krilla MSRV.

**Trigger pattern** (ci-full.yml строки 7-10 — инвертировать: тег вместо branch):
```yaml
on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to build (e.g. 1.2.3)'
        required: true
```

**Permissions pattern** (ci-full.yml строка 16-17 — изменить на write):
```yaml
permissions:
  contents: write   # Обязательно для создания Release и загрузки артефактов
```
Аналог использует `contents: read`; release.yml требует `contents: write`.

**Checkout step** (ci-full.yml строка 30-32 — копировать verbatim):
```yaml
      - name: Checkout
        uses: actions/checkout@v4
```

**Rust toolchain step** (ci-full.yml строки 33-37 — скопировать, заменить версию на 1.92):
```yaml
      - name: Install Rust toolchain (1.92)
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.92'
          components: clippy, rustfmt
```
Примечание: `components: clippy, rustfmt` нужны только для CI-проверок; в release.yml можно опустить, но безопаснее оставить для консистентности.

**Cargo cache step** (ci-full.yml строка 39-40 — копировать verbatim):
```yaml
      - name: Cache Cargo registry + build
        uses: Swatinem/rust-cache@v2
```

**Linux system deps step** (ci-full.yml строки 44-54 — копировать verbatim, с условием `if: runner.os == 'Linux'`):
```yaml
      - name: Install Tauri 2 Linux system dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            libxdo-dev \
            build-essential curl wget file libssl-dev
```

**pnpm setup step** (ci-full.yml строки 56-59 — копировать verbatim):
```yaml
      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 10
```

**Node setup step** (ci-full.yml строки 61-66 — копировать verbatim):
```yaml
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
          cache-dependency-path: ui/pnpm-lock.yaml
```

**pnpm install step** (ci-full.yml строка 83-85 — копировать verbatim):
```yaml
      - name: pnpm install (frozen lockfile)
        working-directory: ui
        run: pnpm install --frozen-lockfile
```

**Cargo cache key pattern** (из ci-full.yml — Swatinem/rust-cache@v2 без доп. параметров; кэш ключ строится автоматически по OS + toolchain + Cargo.lock):
В matrix-job release.yml нужен уникальный ключ на runner, чтобы кэши не конкурировали. `Swatinem/rust-cache@v2` делает это автоматически.

**upload-artifact pattern** (ci-full.yml строки 134-141 — аналог для failure-артефактов; в release.yml применяется иначе):
```yaml
        uses: actions/upload-artifact@v4
        with:
          name: release-artifacts-${{ matrix.platform }}   # уникальное имя на matrix-джоб
          path: |
            target/release/bundle/**/*.exe
            target/release/bundle/**/*.msi
            target/release/bundle/**/*.deb
            target/release/bundle/**/*.AppImage
            target/release/bundle/**/*.dmg
            *-portable.zip
          if-no-files-found: warn
          retention-days: 1    # нужны только для checksums-джоба, потом удалить
```

---

### `crates/trackly-app/tauri.conf.json` (config, modify in place)

**Analog:** self — текущий файл (строки 1-32)

**Текущий bundle блок** (строки 24-30 — исходное состояние, требует изменений):
```json
"bundle": {
  "active": false,
  "targets": "all",
  "icon": [
    "icons/icon.png"
  ]
}
```

**Целевой bundle блок** (на основе RESEARCH.md + реального содержимого `icons/`):
```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ],
  "macOS": {
    "signingIdentity": "-"
  }
}
```

Иконки подтверждены: в `crates/trackly-app/icons/` присутствуют `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`.

**Остальные поля** (`productName`, `identifier`, `version`, `build`, `app`, `plugins`) — не трогать. `version` будет подставляться из тега через jq в CI-шаге (D-13); в файле оставить `"0.1.0"` как fallback.

---

### `README.md` (корневой, новый)

**Analog:** нет в кодовой базе.
**Источник паттерна:** CONTEXT.md §D-16 + RESEARCH.md §BLD-05. Язык — русский (CLAUDE.md).

Структура (определяется требованиями D-06, D-16):
- Описание проекта (что такое Trackly)
- Требования по ОС: Windows 10 x64 (WebView2 Evergreen), macOS aarch64, Linux x86_64
- Запуск: NSIS installer (Windows), portable ZIP (Windows), DMG (macOS), AppImage/deb (Linux)
- Portable-режим: что такое `portable.txt`, куда записываются данные
- Серверный режим: порт, self-signed TLS, как добавить доверие в браузере LAN
- Обход SmartScreen (Windows): правая кнопка → Свойства → Разблокировать, или «Подробнее» → «Выполнить в любом случае»
- Обход Gatekeeper (macOS): Ctrl+клик → Открыть, или Системные настройки → Безопасность

---

### `README-portable.md` (новый, для portable ZIP)

**Analog:** нет.
Краткий (10-20 строк): как запустить trackly.exe из ZIP, что `portable.txt` активирует portable-режим, куда идут данные (`trackly.db` рядом с .exe), как указать свой путь через `trackly.config.toml`.

---

### `trackly.config.toml.example` (новый, шаблон для portable ZIP)

**Analog:** нет в кодовой базе.
Содержимое: закомментированные поля с описанием (язык комментариев — русский).

Пример на основе CONTEXT.md §D-08:
```toml
# Trackly — шаблон конфигурации portable-режима
# Переименуйте в trackly.config.toml и раскомментируйте нужные поля

# [storage]
# Путь к файлу базы данных (по умолчанию: рядом с trackly.exe)
# db_path = "D:\\trackly-data\\trackly.db"

# [server]
# Порт сервера (по умолчанию: 8443)
# port = 8443
# Привязка (по умолчанию: 0.0.0.0 — все интерфейсы)
# bind = "0.0.0.0"
```

---

## Shared Patterns

### Rust Toolchain Version (КРИТИЧНО)

**Источник:** `rust-toolchain.toml` (строка 2: `channel = "1.92.0"`) и `Cargo.toml` (строка 12: `rust-version = "1.92"`)

**Несоответствие в кодовой базе:** `ci-full.yml` и `ci-fast.yml` используют `toolchain: '1.88'` — это баг, который CI не поймал (видимо, 1.88 собирает проект, но MSRV официально 1.92). `release.yml` ДОЛЖЕН использовать `'1.92'` для соответствия `rust-toolchain.toml`.

**Применить к:** все шаги `dtolnay/rust-toolchain@stable` в `release.yml`.

### Cargo cache (Swatinem/rust-cache@v2)

**Источник:** `ci-full.yml` строка 39, `ci-fast.yml` строка 28

**Паттерн:** `Swatinem/rust-cache@v2` без дополнительных параметров. Ключ строится автоматически.

**Применить к:** каждому job в matrix (`build`), а также `create-release` если там нет cargo-шагов (там нет — пропустить).

### pnpm frozen-lockfile

**Источник:** `ci-full.yml` строки 83-85, `ci-fast.yml` строка 73-75

**Паттерн:** `pnpm install --frozen-lockfile` из директории `ui/`.

**Применить к:** шаг перед `tauri-apps/tauri-action@v0` в каждом build matrix-джобе.

### upload-artifact уникальные имена в matrix

**Источник:** `ci-full.yml` строки 134-141 (procmon failure artifacts)
```yaml
name: procmon-failure-${{ github.run_id }}
```

**Паттерн:** уникализировать имя через `${{ matrix.platform }}` или другой суффикс, чтобы matrix-джобы не конфликтовали.

**Применить к:** `actions/upload-artifact@v4` в build matrix-джобе release.yml.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `README.md` (корневой) | doc | — | Корневого README нет в проекте |
| `README-portable.md` | doc | — | Нет документации по portable-режиму |
| `trackly.config.toml.example` | config template | — | Нет существующих шаблонов конфигов |

---

## Metadata

**Analog search scope:** `.github/workflows/`, `crates/trackly-app/`, корень репозитория

**Files scanned:** `ci-full.yml`, `ci-fast.yml`, `cargo-deny.yml`, `tauri.conf.json`, `Cargo.toml`, `rust-toolchain.toml`

**Key finding:** `rust-toolchain.toml` пинит `1.92.0`, а оба существующих CI-файла используют `1.88`. Планировщик должен учесть это при написании release.yml — использовать `'1.92'`.

**Pattern extraction date:** 2026-06-19
