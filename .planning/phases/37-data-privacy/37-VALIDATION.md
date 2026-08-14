---
phase: 37
slug: data-privacy
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-14
---

# Phase 37 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Источник: `37-RESEARCH.md` §Validation Architecture. Требования R1–R10 — `37-SPEC.md`.

> **🔒 Дисциплина файла.** Репозиторий публичный. Ни в одном тестовом артефакте этой фазы,
> ни в этом файле не должно появиться реального значения класса A/B/C. Все фикстуры строятся
> на вымышленных токенах («Иванов И.И.», `example.local`, `+7 495 000-00-00`). См. C-01:
> самотест гейта **не может** гоняться против боевого файла хэшей.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Нет внешнего test-runner'а. Гейт следует паттерну соседних скриптов проекта (`ui/scripts/check-print-isolation.mjs`, `check-contrast.mjs`) — сам себе тест через аргумент-путь + exit-код |
| **Config file** | none — Wave 0 создаёт фикстуры |
| **Quick run command** | `node scripts/check-privacy.mjs --hashes tests/fixtures/privacy/tokens.fixture.sha256 <fixture-path>` |
| **Full suite command** | `node scripts/check-privacy.selftest.mjs` (набор вызовов с фикстурами + проверка exit-кодов), затем `node scripts/check-privacy.mjs` (весь HEAD, боевой список) |
| **Estimated runtime** | самотест ~1 s; полный HEAD-скан ~5–6 s (измерено, см. RESEARCH §Pitfall 1); `--staged` — десятки мс |

*Точные имена файлов гейта/списка хэшей — Claude's Discretion (CONTEXT). Здесь использованы
рабочие имена; план фиксирует окончательные.*

---

## Sampling Rate

- **After every task commit:** `node scripts/check-privacy.mjs --staged --hashes <боевой список>`
  перед каждым `git commit` фазы (C-04). До R10 хук ещё не подключён (C-03) — шаг выполняется
  явной задачей плана, а не автоматикой.
- **After every plan wave:** полный `node scripts/check-privacy.mjs` по HEAD.
- **Before `/gsd-verify-work`:** `ci-fast.yml` зелёный целиком, включая новый шаг гейта.
- **Rust-тесты:** затрагиваются только косвенно (через «`ci-fast` зелёный»). Один `cargo test`
  за раз; прогон со `--skip login_remember_persistent_cookie`; env `TRACKLY_AD_MOCK=1
  TRACKLY_SNMP_MOCK=1` (C-07).
- **Max feedback latency:** ~6 s (полный скан), ~1 s (самотест).

---

## Per-Task Verification Map

*Заполняется после планирования — план ещё не разбит на задачи. Ниже — требование-ориентированная
карта из RESEARCH §Phase Requirements → Test Map; планировщик обязан привязать каждую строку
к конкретной задаче.*

| Req | Behavior | Threat Ref | Test Type | Automated Command | File Exists | Status |
|-----|----------|------------|-----------|-------------------|-------------|--------|
| R1 | Класс A (реквизиты организации) отсутствует в HEAD | — | structural | `git grep -I -f <scratch-маркеры-вне-репо>` → 0 совпадений | ❌ W0 (список маркеров живёт вне репозитория, D-03) | ⬜ pending |
| R2 | Класс B (ФИО) отсутствует в HEAD | — | structural | то же, список класса B | ❌ W0 | ⬜ pending |
| R3 | Класс C (инфраструктурные идентификаторы) отсутствует в HEAD | — | structural | то же, список класса C | ❌ W0 | ⬜ pending |
| R4 | `act-word-source/` вне индекса, PHASE-BRIEF удалён, висячих ссылок нет | — | structural | `git ls-files .planning/reference/act-word-source/` пусто; `test ! -f .planning/PHASE-BRIEF-act-pdf-word-fidelity.md`; `git grep -l 'act-word-source\|PHASE-BRIEF-act-pdf-word-fidelity'` → только `migrations/V033` (D-02) + описательные упоминания в артефактах фазы 37 | ❌ W0 | ⬜ pending |
| R4b | `design-system-v2/` остаётся отслеживаемым (решение 2026-08-14) | — | structural | `git ls-files .planning/reference/design-system-v2/ \| wc -l` = 11 | ❌ W0 | ⬜ pending |
| R5 | `check-privacy-requisites.sh` поглощён и удалён; C-02 регрессия жива | T-37-04 | unit + structural | `test ! -f scripts/check-privacy-requisites.sh`; `grep check-privacy.mjs .github/workflows/ci-fast.yml`; фикстура `.rs` с нефиктивным `inn:`/`ogrn:` литералом → exit 1 | ❌ W0 | ⬜ pending |
| R6 | Список — хэши, не значения; занесённый токен ловится | T-37-02 | unit | `--hashes tokens.fixture.sha256 with-marker.md` → exit 1; `without-marker.md` → exit 0 | ❌ W0 | ⬜ pending |
| R7 | Fail-closed на отсутствующем/пустом/нечитаемом списке | T-37-01 | unit | `--hashes /nonexistent` → exit 1; `--hashes empty.sha256` → exit 1 | ❌ W0 | ⬜ pending |
| R8 | Контроль бинарных расширений | T-37-03 | unit | `.docx` вне allowlist → exit 1; `crates/trackly-app/icons/*` + `tests/fixtures/logo_test.png` → exit 0 | ❌ W0 (легитимные бинарники уже есть) | ⬜ pending |
| R9 | Исключения только по путям, без «снять проверку токена» | — | structural + review | зелёный прогон на `Cargo.lock`; каталог фазы 37 **не** исключён (D-13); code-review констант | ❌ W0 | ⬜ pending |
| R10 | Гейт подключён в двух точках и зелёный | T-37-05 | integration | `.githooks/pre-commit` существует и исполняем; шаг в `ci-fast.yml` сразу после `Checkout`; коммит с тестовым staged-маркером блокируется во временном клоне с `core.hooksPath` | ❌ W0 | ⬜ pending |
| R16 | Сообщение гейта не печатает значение (D-16) | T-37-06 | unit | stdout/stderr падения на `with-marker.md` содержит `путь:строка` и класс, и **не** содержит самого токена (assert по фикстурному токену) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/fixtures/privacy/tokens.fixture.sha256` — тестовый список хэшей на **вымышленных**
      токенах («Иванов И.И.», `example.local`). Обязателен по C-01: боевой список использовать
      нельзя, иначе фикстура сама стала бы утечкой.
- [ ] `tests/fixtures/privacy/with-marker.md` — содержит вымышленный токен из фикстурного списка (R6).
- [ ] `tests/fixtures/privacy/without-marker.md` — не содержит (R6, отрицательный случай).
- [ ] `tests/fixtures/privacy/empty.sha256` — пустой файл (R7).
- [ ] `tests/fixtures/privacy/allowlist-regression.rs.txt` — `.rs`-подобная фикстура с
      нефиктивным `inn:`/`ogrn:` литералом (C-02). Расширение `.txt`, чтобы файл не попал в
      `cargo build`; гейту путь передаётся явно.
- [ ] `tests/fixtures/privacy/binary-regression.docx` — нулевой файл вне allowlist (R8).
- [ ] `scripts/check-privacy.selftest.mjs` — раннер: набор вызовов гейта с фикстурами + проверка
      exit-кодов и отсутствия токена в выводе.
- [ ] Framework install: **не требуется** — Node builtin (`node:crypto`, `node:fs`), zero-dependency (D-09).
- [ ] Списки реальных маркеров классов A/B/C для R1–R3: **не файл в репозитории**. Живут в
      scratchpad автора чистки либо вводятся интерактивно (D-03/D-15). Не коммитятся никогда.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Осмысленность обезличенного текста (смысл абзаца сохранён, а не выпилен) | R1–R3 | Требует человеческого чтения diff; автоматика видит только отсутствие токена | Прочитать `git diff` каждого обезличенного файла: плейсхолдер стоит на месте значения, окружающая фраза остаётся грамматичной и информативной |
| Отсутствие маркеров в **сообщениях коммитов** фазы | C-05 | Хук проверяет содержимое файлов, а не commit message | Перед пушем: `git log --format=%B origin/main..HEAD` — глазами; ни одного значения класса A/B/C |
| Полнота стартового списка токенов | R6 | Нельзя доказать автоматически, что список покрывает все формы (склонения, транслитерация, путь-подобные формы) | После чистки: прогнать гейт по HEAD **до** коммита чистки на локальной копии — он обязан упасть на каждом из 17 файлов; всё, что он пропустил, — дыра в списке |
| Гейт зелёный на очищенном HEAD без ложных срабатываний | R5 acceptance | «Ложное срабатывание» — суждение, а не предикат | `node scripts/check-privacy.mjs` после чистки → exit 0; каждое исключение по пути обосновано комментарием в коде (D-14) |
| `design-system-v2/` не содержит классов A/B | решение 2026-08-14 | 2 из 11 файлов содержат кавычный кириллический текст — гейт даст ответ, но решение о судьбе файла человеческое | Прогнать гейт по каталогу после сборки списка хэшей; при срабатывании — почистить или untrack'ить, не ослаблять гейт |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] Per-Task Verification Map заполнена после планирования (задачи привязаны к R1–R10)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
