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
// режима 1), D-16 (нет утечки значения при срабатывании).
//
// Zero-dependency: только node:fs/path/url/child_process.
//
// Usage:
//   node scripts/check-privacy.selftest.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

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

if (failures > 0) {
  console.error(`${TAG} FAIL — ${failures} нарушений`);
  process.exit(1);
}
console.error(`${TAG} PASS — 0 нарушений`);
process.exit(0);
