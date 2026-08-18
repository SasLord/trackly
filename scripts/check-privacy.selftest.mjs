#!/usr/bin/env node
// [check-privacy-selftest] Фикстур-driven регрессионный self-test для
// scripts/check-privacy.mjs (PRIV-02).
//
// Почему он существует: сам гейт приватности не должен зависеть от
// production-файла хэшей (scripts/privacy-tokens.sha256 ещё не существует —
// его создаёт план 37-04) или от реального содержимого HEAD, чтобы быть
// проверяемым уже сейчас, только на вымышленных данных (C-01). Каждая
// проверка ниже вызывает check-privacy.mjs как отдельный процесс через
// execFileSync с явными fixture-путями и сверяет только exit-код и
// отсутствие утечки значения токена в выводе (D-16) — никогда не сам вывод
// целиком.
//
// Покрывает: R6 (n-грамм хэши), R7 (fail-closed --hashes), R8 (контроль
// бинарных расширений), C-01 (никогда не production-файл), C-02 (регрессия
// режима 1), D-16 (нет утечки значения при срабатывании), а также
// регрессии код-ревью фазы 37: CR-01 (--add и сканер обязаны считать один
// и тот же хэш), CR-02 (нечитаемая цель — отказ, а не молчаливый пропуск),
// WR-01/WR-02/WR-03 (режим 1 видит структурированные конфиги, регистр
// ключа и значение без кавычек), WR-04 (состав fixture-каталога, который
// авто-сканирование не проверяет, зафиксирован явным списком).
//
// Zero-dependency: только node:fs/os/path/url/child_process.
//
// Usage:
//   node scripts/check-privacy.selftest.mjs

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
// Импорт из самого гейта: файл выполняет main() только при прямом вызове,
// поэтому импорт безопасен. Нужен, чтобы проверить CR-01 сквозным
// round-trip'ом — --add требует TTY и напрямую из теста не вызывается.
import { canonicalizeAddValue, normalize, sha256Hex } from './check-privacy.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const GATE = path.join(REPO_ROOT, 'scripts', 'check-privacy.mjs');
const FIXTURES = path.join(REPO_ROOT, 'scripts', 'fixtures', 'privacy');
const TAG = '[check-privacy-selftest]';

const TOKENS_HASHES = path.join(FIXTURES, 'tokens.fixture.sha256');
const EMPTY_HASHES = path.join(FIXTURES, 'empty.sha256');
const NONEXISTENT_HASHES = path.join(FIXTURES, 'this-file-does-not-exist.sha256');

let failures = 0;

/** Запускает check-privacy.mjs как дочерний процесс; никогда не бросает —
 * возвращает {code, output} с объединённым stdout+stderr для инспекции. */
function runGate(args) {
  try {
    const output = execFileSync('node', [GATE, ...args], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { code: 0, output };
  } catch (err) {
    const code = typeof err.status === 'number' ? err.status : 1;
    const output = `${err.stdout || ''}${err.stderr || ''}`;
    return { code, output };
  }
}

function assertExitCode(name, args, expectedCode) {
  const { code } = runGate(args);
  if (code !== expectedCode) {
    console.error(`${TAG} FAIL — ${name}: ожидался exit ${expectedCode}, получен ${code}`);
    failures++;
    return null;
  }
  console.error(`${TAG} ok — ${name}`);
  return code;
}

function assertTrue(name, condition) {
  if (!condition) {
    console.error(`${TAG} FAIL — ${name}`);
    failures++;
    return;
  }
  console.error(`${TAG} ok — ${name}`);
}

/** Одноразовый каталог вне репозитория: файлы-регрессии CR-01/CR-02/WR-*
 * намеренно «грязные», их нельзя оставлять в дереве проекта. */
function withTempDir(fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-privacy-selftest-'));
  try {
    return fn(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function assertNoLeak(name, args, secret) {
  const { output } = runGate(args);
  if (output.includes(secret)) {
    console.error(`${TAG} FAIL — ${name}: вывод гейта содержит значение токена (утечка D-16)`);
    failures++;
    return;
  }
  console.error(`${TAG} ok — ${name} (значение токена не встречается в выводе)`);
}

// ---------------------------------------------------------------------------
// 1. with-marker.md — вымышленный маркер смежными словами → exit 1, без
//    утечки значения в выводе (D-05, D-16).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'with-marker.md');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('with-marker.md: смежный маркер найден', args, 1);
  assertNoLeak('with-marker.md: значение маркера не утекает в вывод', args, 'Пчёлкин Артём');
}

// ---------------------------------------------------------------------------
// 2. without-marker.md — те же слова, но не смежные → exit 0.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('without-marker.md: несмежные слова проходят гейт', args, 0);
}

// ---------------------------------------------------------------------------
// 3. Несуществующий --hashes путь → exit 1 (R7, fail-closed).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', NONEXISTENT_HASHES, target];
  assertExitCode('несуществующий --hashes падает закрыто (R7)', args, 1);
}

// ---------------------------------------------------------------------------
// 4. Пустой --hashes (empty.sha256) → exit 1 (R7, fail-closed).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', EMPTY_HASHES, target];
  assertExitCode('пустой --hashes падает закрыто (R7)', args, 1);
}

// ---------------------------------------------------------------------------
// 5. Регрессия режима 1 (allowlist) — C-02.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'allowlist-regression.rs.txt');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('allowlist-regression.rs.txt: неразрешённый литерал ловится (C-02)', args, 1);
  assertNoLeak('allowlist-regression.rs.txt: значение литерала не утекает в вывод', args, '9998887770');
}

// ---------------------------------------------------------------------------
// 6. Контроль бинарных расширений — R8.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'binary-regression.docx');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('binary-regression.docx вне allowlist падает (R8)', args, 1);
}
{
  const iconsDir = path.join(REPO_ROOT, 'crates/trackly-app/icons');
  const iconFiles = fs.readdirSync(iconsDir).map((f) => path.join(iconsDir, f));
  const logoFixture = path.join(REPO_ROOT, 'crates/trackly-app/tests/fixtures/logo_test.png');
  const args = ['--hashes', TOKENS_HASHES, ...iconFiles, logoFixture];
  assertExitCode('icons/ + logo_test.png в BINARY_ALLOWLIST проходят (R8)', args, 0);
}

// ---------------------------------------------------------------------------
// 7. CR-01 — --add обязан хэшировать ту же каноническую форму, что строит
//    сканер. Значения только вымышленные (C-01).
// ---------------------------------------------------------------------------
{
  // Дефис внутри значения: токенизатор сканера его отбрасывает, поэтому
  // канонической формой становится «Иванов Петров» — и именно её хэш
  // обязан записывать --add. До исправления хэшировался сырой ввод, и
  // запись не срабатывала никогда (мёртвая строка в доверенном списке).
  const raw = 'Иванов-Петров';
  const canon = canonicalizeAddValue(raw);
  assertTrue('CR-01: значение с дефисом принимается и канонизируется', canon.ok);
  assertTrue(
    'CR-01: каноническая форма — слова через один пробел ASCII',
    canon.ok && canon.canonical === 'Иванов Петров',
  );
  assertTrue(
    'CR-01: хэш считается от канонической формы, а не от сырого ввода',
    canon.ok && canon.hash !== sha256Hex(normalize(raw)),
  );

  withTempDir((dir) => {
    const doc = path.join(dir, 'cr01-doc.md');
    fs.writeFileSync(doc, `Акт подписан представителем ${raw} по доверенности.\n`, 'utf8');

    const fixedHashes = path.join(dir, 'cr01-canonical.sha256');
    fs.writeFileSync(fixedHashes, `# scratch\n${canon.hash} B\n`, 'utf8');
    assertExitCode(
      'CR-01: хэш из --add срабатывает на дефисной форме в тексте',
      ['--hashes', fixedHashes, doc],
      1,
    );
    assertNoLeak('CR-01: значение не утекает в вывод', ['--hashes', fixedHashes, doc], raw);

    // Контроль «до исправления»: хэш сырого ввода не воспроизводим сканером.
    const buggyHashes = path.join(dir, 'cr01-raw.sha256');
    fs.writeFileSync(buggyHashes, `# scratch\n${sha256Hex(normalize(raw))} B\n`, 'utf8');
    assertExitCode(
      'CR-01: хэш сырого ввода (старое поведение) не срабатывает — потому он и запрещён',
      ['--hashes', buggyHashes, doc],
      0,
    );
  });

  // Непредставимые значения обязаны отклоняться, а не записываться молча.
  const tooLong = canonicalizeAddValue('улица Вымышленная дом двенадцать строение три');
  assertTrue(
    'CR-01: значение длиннее окна n-грамм отклоняется',
    tooLong.ok === false && tooLong.reason === 'too_many_words',
  );
  const noWords = canonicalizeAddValue('--- ...');
  assertTrue(
    'CR-01: значение без буквенно-цифровых слов отклоняется',
    noWords.ok === false && noWords.reason === 'no_words',
  );
}

// ---------------------------------------------------------------------------
// 8. CR-02 — нечитаемая цель обязана давать exit 1, а не «PASS — 0 нарушений».
// ---------------------------------------------------------------------------
{
  withTempDir((dir) => {
    const missing = path.join(dir, 'this-target-does-not-exist.md');
    assertExitCode(
      'CR-02: нечитаемая цель падает закрыто, а не пропускается молча',
      ['--hashes', TOKENS_HASHES, missing],
      1,
    );

    const clean = path.join(dir, 'clean.md');
    fs.writeFileSync(clean, 'Совершенно обычный текст без маркеров.\n', 'utf8');
    assertExitCode(
      'CR-02: чистая цель рядом с нечитаемой всё равно роняет прогон',
      ['--hashes', TOKENS_HASHES, clean, missing],
      1,
    );
    assertExitCode(
      'CR-02: одна чистая цель по-прежнему проходит (нет ложного отказа)',
      ['--hashes', TOKENS_HASHES, clean],
      0,
    );
  });
}

// ---------------------------------------------------------------------------
// 9. WR-01/WR-02/WR-03 — режим 1 видит структурированные конфиги, регистр
//    ключа и значение без кавычек. Цифры ниже вымышлены и отсутствуют в
//    ALLOWED (C-01).
// ---------------------------------------------------------------------------
{
  withTempDir((dir) => {
    const json = path.join(dir, 'org.json');
    fs.writeFileSync(json, '{ "inn": "9998887774", "kpp": "999888777" }\n', 'utf8');
    assertExitCode(
      'WR-01: неразрешённый литерал в .json ловится режимом 1',
      ['--hashes', TOKENS_HASHES, json],
      1,
    );

    const upper = path.join(dir, 'upper.rs');
    fs.writeFileSync(upper, 'struct FakeOrg {\n    INN: "9998887773",\n}\n', 'utf8');
    assertExitCode(
      'WR-02: ключ в верхнем регистре ловится режимом 1',
      ['--hashes', TOKENS_HASHES, upper],
      1,
    );

    const bare = path.join(dir, 'bare.rs');
    fs.writeFileSync(bare, 'struct FakeOrg {\n    inn: 9998887772,\n}\n', 'utf8');
    assertExitCode(
      'WR-03: значение без кавычек ловится режимом 1',
      ['--hashes', TOKENS_HASHES, bare],
      1,
    );

    // Разрешённое значение не должно ловиться ни в одной из новых форм —
    // расширялось только обнаружение, ALLOWED не менялся.
    const allowedShapes = path.join(dir, 'allowed.rs');
    fs.writeFileSync(
      allowedShapes,
      'struct Demo {\n    INN: "7700000000",\n    inn: 7700000000,\n    kpp: 7_700_000_00,\n}\n',
      'utf8',
    );
    assertExitCode(
      'WR-01/02/03: разрешённые значения в новых формах не дают ложных срабатываний',
      ['--hashes', TOKENS_HASHES, allowedShapes],
      0,
    );
  });
}

// ---------------------------------------------------------------------------
// 10. WR-04 — scripts/fixtures/privacy/ исключён из авто-сканирования
//     (AUTO_SCAN_EXCLUDED_PREFIXES), поэтому реальные данные, попавшие сюда,
//     не поймал бы ни хук, ни CI. Состав каталога зафиксирован явным
//     списком: любое добавление файла обязано пройти через правку этого
//     теста, то есть через code review.
// ---------------------------------------------------------------------------
{
  const EXPECTED_FIXTURES = [
    'README.md',
    'allowlist-regression.rs.txt',
    'binary-regression.docx',
    'empty.sha256',
    'tokens.fixture.sha256',
    'with-marker.md',
    'without-marker.md',
  ];
  const actual = fs.readdirSync(FIXTURES).sort();
  const expected = [...EXPECTED_FIXTURES].sort();
  const same =
    actual.length === expected.length && actual.every((f, i) => f === expected[i]);
  if (!same) {
    const added = actual.filter((f) => !expected.includes(f));
    const removed = expected.filter((f) => !actual.includes(f));
    console.error(
      `${TAG} FAIL — состав scripts/fixtures/privacy/ изменился (добавлено: ${added.join(', ') || '—'}; удалено: ${removed.join(', ') || '—'}). Этот каталог исключён из авто-сканирования гейта: убедись, что новый файл полностью вымышлен (C-01), и обнови EXPECTED_FIXTURES.`,
    );
    failures++;
  } else {
    console.error(`${TAG} ok — состав scripts/fixtures/privacy/ совпадает с ожидаемым (WR-04)`);
  }
}

// ---------------------------------------------------------------------------

if (failures > 0) {
  console.error(`${TAG} FAIL — ${failures} нарушений`);
  process.exit(1);
}
console.error(`${TAG} PASS — 0 нарушений`);
process.exit(0);
