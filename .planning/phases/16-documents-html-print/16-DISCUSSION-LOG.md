# Phase 16: documents-html-print - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-05
**Phase:** 16-documents-html-print
**Areas discussed:** Движок HTML-шаблонов, Папка templates + fallback, Доставка + печать, Тесты + судьба krilla

---

## Движок HTML-шаблонов

### Q1: Что именно отказываемся использовать из «document_templates/MiniJinja»?

| Option | Description | Selected |
|--------|-------------|----------|
| Только БД-хранение + DocSpec JSON | Отказ от document_templates + DocSpec-JSON, но MiniJinja-крейт переиспользуется для рендера HTML | |
| И MiniJinja-крейт тоже | Полный уход: HTML собирается Rust-кодом (format!/build-time) | |
| Другой HTML-движок | Новый крейт под HTML (askama/tera) | ✓ |

**User's choice:** «Отказываемся от MiniJinja-крейт. Что за новый крейт под HTML?» + уточнил: хочет сам писать HTML+CSS файл в VSCode с местами под переменные и циклы.
**Notes:** Открыло ключевой констрейнт — Req 4 (правка файла без пересборки) исключает compile-time движки (askama/maud). Нужен рантайм-движок.

### Q2: Какой рантайм-движок под HTML?

| Option | Description | Selected |
|--------|-------------|----------|
| MiniJinja → HTML (рекомендовано) | Тот же синтаксис {{}}/{%%}, 0 новых deps, autoescape, safe-mode+timeout готовы | ✓ |
| Tera → HTML | Новый крейт, Jinja2, но дублирует MiniJinja и тяжелее | |
| Другое / обсудить | | |

**User's choice:** MiniJinja → HTML (рекомендовано).
**Notes:** После разбора: «новым механизмом» является связка файлы+HTML (вместо БД+DocSpec), а не смена крейта. Пользователь верстает `templates/*.html` сам; стили инлайн `<style>`; логотип `data:`-URI.

---

## Папка templates + fallback

### Q3: Как пользователь получит файл шаблона для правки, если папки нет?

| Option | Description | Selected |
|--------|-------------|----------|
| Авто-материализация при старте (рекомендовано) | Записать вшитый дефолт в templates/ при первом старте | ✓ |
| Только include_str! (не писать) | Папка не создаётся; всегда вшитый дефолт пока юзер сам не создаст файл | |
| Материализация по кнопке | Кнопка в Настройках «Экспортировать шаблоны» | |

**User's choice:** Авто-материализация при старте.

### Q4: Как разрулить dev vs prod путь к templates/?

| Option | Description | Selected |
|--------|-------------|----------|
| ENV-override для dev (рекомендовано) | Прод — current_exe().parent()/templates/; dev/тесты — TRACKLY_TEMPLATES_DIR | ✓ |
| Не важно, target/debug ок | Пусть пишет в target/debug/templates/ | |
| Решит планировщик | | |

**User's choice:** ENV-override для dev.

### Q5: Когда перечитывать файл шаблона?

| Option | Description | Selected |
|--------|-------------|----------|
| На каждую генерацию (рекомендовано) | Read-on-render; правка применяется сразу | ✓ |
| Кеш + notify-watch | Читать в память, следить notify-крейтом | |

**User's choice:** На каждую генерацию.

---

## Доставка + печать

### Q6: Как юзер открывает и печатает HTML-акт единообразно desktop+LAN?

| Option | Description | Selected |
|--------|-------------|----------|
| iframe в модалке + print() (рекомендовано) | HTML-строка → srcdoc в PdfPreviewModal → iframe.print() | ✓ |
| Новая вкладка/окно | Отдельная вкладка + печать; в Tauri сложнее | |
| Оба: preview + «Открыть/Печать» | iframe-preview + опц. «Открыть в браузере» | |

**User's choice:** iframe в модалке + print().

### Q7: Что делать с текущими Vec<u8>-командами/эндпоинтами?

| Option | Description | Selected |
|--------|-------------|----------|
| Заменить на HTML-строку (рекомендовано) | Те же имена, возврат String (HTML); HTTP text/html; open_pdf_in_system убрать | ✓ |
| Новые команды render_html | Оставить старые Vec<u8> + добавить новые | |

**User's choice:** Заменить на HTML-строку.

### Q8: Как встраивать логотип для офлайн-печати?

| Option | Description | Selected |
|--------|-------------|----------|
| data:-URI в HTML (рекомендовано) | base64 data:-URI в <img src>; self-contained, офлайн | ✓ |
| Локальный endpoint | <img src="/api/v1/org/logo">; ломается в desktop-без-сервера | |

**User's choice:** data:-URI в HTML.

### Q9: Как гарантировать чистый print-вывод (A4, без браузерных колонтитулов)?

| Option | Description | Selected |
|--------|-------------|----------|
| @page CSS + инструкция (рекомендовано) | @page A4 + page-break; колонтитулы отключаются галочкой в диалоге + UAT-подсказка | ✓ |
| Обсудить глубже | | |

**User's choice:** @page CSS + инструкция.
**Notes:** Мультиустройство/page-break решается CSS (`page-break-inside: avoid` + браузерная A4-пагинация).

---

## Тесты + судьба krilla

### Q10: Судьба замороженных krilla-тестов?

| Option | Description | Selected |
|--------|-------------|----------|
| Оставить зелёными | Все krilla-тесты гоняются и остаются зелёными | |
| Пометить #[ignore] | Заморозить все через #[ignore] | |
| Гибрид | Быстрые единичные зелёные, тяжёлые/медленные #[ignore] | ✓ |

**User's choice:** Гибрид.

### Q11: Какие HTML-тесты добавить (Req 8)? (multiselect)

| Option | Description | Selected |
|--------|-------------|----------|
| Наличие блоков/полей | Обязательные блоки + логотип data:-URI в разметке | ✓ |
| 1 vs N устройств | Все позиции присутствуют, длинные поля не обрезаны | ✓ |
| Fallback vs файл | Дефолт при отсутствии папки; файл при наличии; правка меняет вывод | ✓ |
| Офлайн/без-CDN | Нет http(s)-ссылок в href/src кроме data: | ✓ |

**User's choice:** Все четыре.

---

## Claude's Discretion

- Точная HTML/CSS-вёрстка образца Word (порядок/стили блоков) — воспроизвести результат Phase 15.
- Точное имя ENV-переменной dev-override пути templates/.
- Механика передачи HTML-строки во фронт (srcdoc vs blob) в деталях.

## Deferred Ideas

None — обсуждение осталось в границах фазы.
