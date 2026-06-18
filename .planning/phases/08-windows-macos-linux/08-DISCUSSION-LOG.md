# Phase 8: Релизный пайплайн (Windows/macOS/Linux) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-19
**Phase:** 08-windows-macos-linux
**Areas discussed:** Целевые платформы, Подпись и нотаризация, Сборка portable ZIP, Триггер и версионирование, Release notes, Linux форматы, Структура README

---

## Целевые платформы (Win7/32-bit)

**User's choice (free-text):** «Windows 7 не интересует, запускаться будет на Windows 10 x64».
**Notes:** Win7 и 32-bit (i686) полностью убраны из scope. Не применять Win7-рекомендации CLAUDE.md (embedBootstrapper, i686 target).

---

## Подпись и нотаризация

| Option | Description | Selected |
|--------|-------------|----------|
| Нет серта — без подписи + SHA256 | v1 без подписи, только SHA256-checksums, SmartScreen в README | ✓ (Windows) |
| Есть OV/EV cert | Подписывать в CI через GitHub secret | |
| Пока нет, но заложить крючок | Условный шаг подписи | |

| Option | Description | Selected |
|--------|-------------|----------|
| Без подписи (ad-hoc) | macOS без Developer ID, Gatekeeper в README | ✓ (macOS) |
| Есть Apple Developer ID | Подпись + notarytool в CI | |
| macOS не приоритет | .dmg без подписи | |

**User's choice:** Windows — без серта (SHA256 only); macOS — ad-hoc без подписи.
**Notes:** Подпись/нотаризация отложены до появления сертификатов. SmartScreen/Gatekeeper обход документируется в README.

---

## Сборка portable ZIP

| Option | Description | Selected |
|--------|-------------|----------|
| Системный (Evergreen) | Не бандлить WebView2 runtime | ✓ |
| Bundled fixed-version | Вложить фиксированный WebView2 (~150MB) | |

| Option | Description | Selected |
|--------|-------------|----------|
| exe + portable.txt + README | Рав бинарник + маркер + краткий README | |
| Только exe + portable.txt | Минимум | |
| + пример config | exe + portable.txt + README + шаблон trackly.config.toml | ✓ |

**User's choice:** Системный WebView2 Evergreen; ZIP = exe + portable.txt + README + закомментированный trackly.config.toml.

---

## Триггер и версионирование

| Option | Description | Selected |
|--------|-------------|----------|
| Draft-релиз (ручная публикация) | tauri-action в DRAFT, публикация вручную | ✓ |
| Авто-publish | Сразу публичный релиз | |

| Option | Description | Selected |
|--------|-------------|----------|
| Тег — источник истины | Версия из тега, CI подставляет в Cargo/tauri.conf | ✓ |
| Файлы — источник истины | Версия в файлах, тег должен совпадать | |

**User's choice:** Draft-релиз + ручная публикация; тег = источник версии.

---

## Release notes

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub auto-generated | Notes из PR/коммитов между тегами | ✓ |
| Пустой draft — вручную | CI создаёт draft без notes | |
| Из CHANGELOG.md | Ручной CHANGELOG, CI берёт секцию | |

**User's choice:** GitHub auto-generated.

---

## Linux форматы/приоритет

| Option | Description | Selected |
|--------|-------------|----------|
| .AppImage + .deb (как в BLD-02) | Оба формата | ✓ |
| Только .AppImage | Универсальный, проще | |
| Linux отложить | Только Win + macOS в v1 | |

**User's choice:** .AppImage + .deb (BLD-02 без изменений).

---

## Структура README

| Option | Description | Selected |
|--------|-------------|----------|
| Корневой README.md, полный | Полный RU README со всеми разделами | ✓ |
| Короткий + ссылки на docs/ | Краткий README + детали в docs/ | |

**User's choice:** Корневой /README.md, полный, на русском.

---

## Claude's Discretion

- Формат checksums-файла (единый `SHA256SUMS` vs per-file `.sha256`) — ориентир: единый `SHA256SUMS`.
- Структура `release.yml` (отдельный workflow vs расширение существующего) — рекомендуется отдельный `release.yml`.
- Механизм подстановки версии из тега.
- Retention/имена артефактов в Release.

## Deferred Ideas

- Code-signing Windows (OV/EV) — когда появится сертификат.
- macOS подпись + нотаризация (Apple Developer ID) — когда появится Developer ID.
- Windows 7 / 32-bit (i686) — вне scope, не возвращать без запроса.
- Bundled fixed-version WebView2 runtime — на случай оффлайн-машин.
- AD-вход / заявки / авто-приём (USR-08..12, REQ-06, SET-10) — будущая Phase 9, авто-SSO.
