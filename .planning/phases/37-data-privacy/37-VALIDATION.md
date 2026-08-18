---
phase: 37
slug: data-privacy
status: draft
nyquist_compliant: true
wave_0_complete: true
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

| Req | Plan/Task | Behavior | Threat Ref | Test Type | Automated Command | File Exists | Status |
|-----|-----------|----------|------------|-----------|-------------------|-------------|--------|
| R1 | 37-01 Task 2 (+ 37-02 Task 3 for the 3 overlap files) | Класс A (реквизиты организации) отсутствует в HEAD | — | structural | `git grep -I -f <scratch-маркеры-вне-репо>` → 0 совпадений | ✅ | ✅ |
| R2 | 37-01 Task 2 (+ 37-02 Task 3 for the 3 overlap files) | Класс B (ФИО) отсутствует в HEAD | — | structural | то же, список класса B | ✅ | ✅ |
| R3 | 37-01 Task 2 (+ 37-02 Task 3 for the 3 overlap files) | Класс C (инфраструктурные идентификаторы) отсутствует в HEAD | — | structural | то же, список класса C | ✅ | ✅ |
| R4 | 37-02 Task 1/2/3 | `act-word-source/` вне индекса, PHASE-BRIEF удалён, висячих ссылок нет | — | structural | `git ls-files .planning/reference/act-word-source/` пусто; `test ! -f .planning/PHASE-BRIEF-act-pdf-word-fidelity.md`; `git grep -l 'act-word-source\|PHASE-BRIEF-act-pdf-word-fidelity'` → только `migrations/V033` (D-02) + описательные упоминания в артефактах фазы 37 | ✅ | ✅ |
| R4b | 37-02 Task 1 | `design-system-v2/` остаётся отслеживаемым (решение 2026-08-14) | — | structural | `git ls-files .planning/reference/design-system-v2/ \| wc -l` = 11 | ✅ | ✅ |
| R5 | 37-04 Task 3 | `check-privacy-requisites.sh` поглощён и удалён; C-02 регрессия жива | T-37-04 | unit + structural | `test ! -f scripts/check-privacy-requisites.sh`; `grep check-privacy.mjs .github/workflows/ci-fast.yml`; фикстура `.rs` с нефиктивным `inn:`/`ogrn:` литералом → exit 1 | ✅ | ✅ |
| R6 | 37-03 Task 2 | Список — хэши, не значения; занесённый токен ловится | T-37-02 | unit | `--hashes tokens.fixture.sha256 with-marker.md` → exit 1; `without-marker.md` → exit 0 | ✅ | ✅ |
| R7 | 37-03 Task 1 | Fail-closed на отсутствующем/пустом/нечитаемом списке | T-37-01 | unit | `--hashes /nonexistent` → exit 1; `--hashes empty.sha256` → exit 1 | ✅ | ✅ |
| R8 | 37-03 Task 1/2 | Контроль бинарных расширений | T-37-03 | unit | `.docx` вне allowlist → exit 1; `crates/trackly-app/icons/*` + `tests/fixtures/logo_test.png` → exit 0 | ✅ | ✅ |
| R9 | 37-03 Task 1 / 37-04 Task 1 | Исключения только по путям, без «снять проверку токена» | — | structural + review | зелёный прогон на `Cargo.lock`; каталог фазы 37 **не** исключён (D-13); code-review констант | ✅ | ✅ |
| R10 | 37-04 Task 2/3 | Гейт подключён в двух точках и зелёный | T-37-05 | integration | `.githooks/pre-commit` существует и исполняем; шаг в `ci-fast.yml` сразу после `Checkout`; коммит с тестовым staged-маркером блокируется во временном клоне с `core.hooksPath` | ✅ | ✅ |
| R16 | 37-03 Task 1/2 | Сообщение гейта не печатает значение (D-16) | T-37-06 | unit | stdout/stderr падения на `with-marker.md` содержит `путь:строка` и класс, и **не** содержит самого токена (assert по фикстурному токену) | ✅ | ✅ |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Реализовано под `scripts/fixtures/privacy/` (не `tests/fixtures/privacy/`) — путь Claude's
Discretion, зафиксирован планом 37-03; содержимое и назначение файлов не изменились.*

- [x] `scripts/fixtures/privacy/tokens.fixture.sha256` — тестовый список хэшей на **вымышленных**
      токенах («Иванов И.И.», `example.local`). Обязателен по C-01: боевой список использовать
      нельзя, иначе фикстура сама стала бы утечкой.
- [x] `scripts/fixtures/privacy/with-marker.md` — содержит вымышленный токен из фикстурного списка (R6).
- [x] `scripts/fixtures/privacy/without-marker.md` — не содержит (R6, отрицательный случай).
- [x] `scripts/fixtures/privacy/empty.sha256` — пустой файл (R7).
- [x] `scripts/fixtures/privacy/allowlist-regression.rs.txt` — `.rs`-подобная фикстура с
      нефиктивным `inn:`/`ogrn:` литералом (C-02). Расширение `.txt`, чтобы файл не попал в
      `cargo build`; гейту путь передаётся явно.
- [x] `scripts/fixtures/privacy/binary-regression.docx` — нулевой файл вне allowlist (R8).
- [x] `scripts/check-privacy.selftest.mjs` — раннер: набор вызовов гейта с фикстурами + проверка
      exit-кодов и отсутствия токена в выводе.
- [x] Framework install: **не требуется** — Node builtin (`node:crypto`, `node:fs`), zero-dependency (D-09).
- [x] Списки реальных маркеров классов A/B/C для R1–R3: **не файл в репозитории**. Жили в
      scratchpad автора чистки (37-01/37-02) и в скретч-скрипте программного извлечения
      значений из истории (37-04 Task 1) — никогда не коммитились.

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

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] Per-Task Verification Map заполнена после планирования (задачи привязаны к R1–R10)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** green — full-HEAD `node scripts/check-privacy.mjs` exits 0 (including
`.planning/reference/design-system-v2/`'s 11 files scanned explicitly), the 18-file
pre-cleanup completeness proof (plan 37-04 Task 3) fired on every file with zero holes,
the pre-commit hook blocks a scratch-clone commit carrying an unapproved requisite
literal (mode 1; synthetic digit strings, never a real value) while letting a clean
commit through, refuses to run at all when `node` is absent (D-11 fail-closed), and
scopes its scan to staged blobs rather than the working tree; `cargo test` is green
under the C-07 constraints — 102 suites, 0 failed (see 37-04-SUMMARY.md).
