#!/usr/bin/env node
// [check-write-site-gate-untouched] Единая проверка «счётный гейт
// crates/trackly-app/tests/group_write_sites.rs не ослаблен» для планов фазы 41.2
// (04, 07, 09, 11, 13, 16).
//
// Почему он существует: прежняя проверка грепала дифф по токенам
// `WINDOW|count_calls|paired|...`. Она слепа к смене литерала счёта: rustfmt
// разносит `assert_eq!(count_calls(src, &WRITE_SITE_CALLS), 8, ..)` по строкам,
// и удалённая строка `        8,` не содержит ни одного токена. Гейт мог стать
// зелёным при снятой охране.
//
// Как работает: `git diff -U0 <BASE> -- <файл>`; из заголовков ханков
// `@@ -a,b +c,d @@` берутся номера СТАРЫХ строк; проверяется каждая УДАЛЁННАЯ
// строка (добавленные игнорируются: планы могут только дописывать). Область
// охраны — номера строк БАЗОВОЙ версии от единственной строки с
// `const WRITE_SITE_CALLS` до единственной строки с `assert_eq!(paired,`.
// Удалённая строка — ослабление, если:
//   (1) содержит WINDOW, count_calls, paired, WRITE_SITE, RELEASE_CALLS,
//       `fn s11_` или `anchor_group_of_device_in_tx`;
//   (2) это строка needle'а массива (`^\s*"[a-z_]+\(",$`);
//   (3) это строка кортежа реестра (`\.rs", [0-9]+\),?$`);
//   (4) это голая числовая строка `^\s*[0-9]+,$` и её старый номер лежит В
//       ОБЛАСТИ охраны (rustfmt ставит литерал счёта отдельной строкой; вне
//       области голые числа принадлежат сценариям поведения, которые правят
//       другие планы фазы).
// Нет файла, нет якоря в базе или якорь не один — код 2 и сообщение на stderr:
// молчаливого SKIP быть не должно.
//
// Вывод: `weakened: N` и список «старый номер: текст». Код 0 при N = 0, иначе 1.
//
// Self-test (`--selftest`): во временном каталоге поднимается git-репозиторий с
// ВЫМЫШЛЕННОЙ фикстурой, повторяющей форму гейта; мутанты прогоняются через ту
// же функцию разбора, каждый обязан дать weakened >= 1; контрольные случаи
// (только добавления, число вне области, пустой дифф) обязаны дать 0; отсутствие
// якоря в базе обязано дать код 2. Мутатор требует РОВНО одно вхождение якоря в
// фикстуре, иначе он мог бы снять охрану у соседнего фрагмента.
//
// Zero-dependency: node:child_process, node:fs, node:os, node:path, node:url.
//
// Usage:
//   node scripts/check-write-site-gate-untouched.mjs <BASE> [file]
//   node scripts/check-write-site-gate-untouched.mjs --selftest

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-write-site-gate-untouched]';
const DEFAULT_FILE = 'crates/trackly-app/tests/group_write_sites.rs';

const START_ANCHOR = 'const WRITE_SITE_CALLS';
const END_ANCHOR = 'assert_eq!(paired,';
const TOKENS = [
  'WINDOW',
  'count_calls',
  'paired',
  'WRITE_SITE',
  'RELEASE_CALLS',
  'fn s11_',
  'anchor_group_of_device_in_tx',
];
const NEEDLE_RE = /^\s*"[a-z_]+\(",$/;
const TUPLE_RE = /\.rs", [0-9]+\),?$/;
const BARE_NUMBER_RE = /^\s*[0-9]+,$/;

class GateConfigError extends Error {}

function git(repoDir, args) {
  return execFileSync('git', ['-C', repoDir, ...args], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 64 * 1024 * 1024,
  });
}

/** Номера (1-based) строк, содержащих подстроку. */
function linesContaining(text, needle) {
  const out = [];
  text.split('\n').forEach((line, i) => {
    if (line.includes(needle)) out.push(i + 1);
  });
  return out;
}

/** Область охраны в номерах строк базовой версии: [от, до] включительно. */
export function guardedRange(baseText) {
  const starts = linesContaining(baseText, START_ANCHOR);
  const ends = linesContaining(baseText, END_ANCHOR);
  if (starts.length !== 1) {
    throw new GateConfigError(
      `в базовой версии якорь «${START_ANCHOR}» встречается ${starts.length} раз(а), нужен ровно один`,
    );
  }
  if (ends.length !== 1) {
    throw new GateConfigError(
      `в базовой версии якорь «${END_ANCHOR}» встречается ${ends.length} раз(а), нужен ровно один`,
    );
  }
  if (starts[0] >= ends[0]) {
    throw new GateConfigError('якорь начала области стоит не выше якоря конца');
  }
  return [starts[0], ends[0]];
}

/** Удалённые строки диффа `-U0`: [{ oldLine, text }]. Добавленные игнорируются. */
export function removedLines(diffText) {
  const out = [];
  let oldLine = 0;
  let inHunk = false;
  for (const raw of diffText.split('\n')) {
    const header = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(raw);
    if (header) {
      inHunk = true;
      // При -U0 и нулевой длине старого диапазона номер указывает на строку ПЕРЕД
      // вставкой; удалений в таком ханке нет, поэтому номер не используется.
      oldLine = Number(header[1]);
      continue;
    }
    if (!inHunk) continue;
    if (raw.startsWith('\\')) continue; // «\ No newline at end of file»
    if (raw.startsWith('-')) {
      out.push({ oldLine, text: raw.slice(1) });
      oldLine += 1;
    }
    // '+' строки не двигают счётчик старых строк.
  }
  return out;
}

/** Ослабления: удалённые строки, нарушающие правила (1)-(4). */
export function findWeakened(baseText, diffText) {
  const [from, to] = guardedRange(baseText);
  const weakened = [];
  for (const { oldLine, text } of removedLines(diffText)) {
    const inArea = oldLine >= from && oldLine <= to;
    const hit =
      TOKENS.some((t) => text.includes(t)) ||
      NEEDLE_RE.test(text) ||
      TUPLE_RE.test(text) ||
      (inArea && BARE_NUMBER_RE.test(text));
    if (hit) weakened.push({ oldLine, text });
  }
  return weakened;
}

/** Проверить файл относительно BASE. Возвращает { code, weakened, message }. */
export function checkFile(repoDir, base, relFile) {
  let baseText;
  try {
    baseText = git(repoDir, ['show', `${base}:${relFile}`]);
  } catch (e) {
    return {
      code: 2,
      weakened: [],
      message: `не удалось прочитать ${relFile} в ${base}: файла нет в базе или база неверна`,
    };
  }
  try {
    guardedRange(baseText);
  } catch (e) {
    if (e instanceof GateConfigError) return { code: 2, weakened: [], message: e.message };
    throw e;
  }
  let diffText;
  try {
    diffText = git(repoDir, ['diff', '-U0', '--no-color', '--no-ext-diff', base, '--', relFile]);
  } catch (e) {
    return { code: 2, weakened: [], message: `git diff не выполнился: ${e.message}` };
  }
  const weakened = findWeakened(baseText, diffText);
  return { code: weakened.length === 0 ? 0 : 1, weakened, message: '' };
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

const FIXTURE_FILE = 'tests/fixture_gate.rs';

// Вымышленная фикстура, повторяющая форму реального гейта: многострочные
// assert_eq! со счётом отдельной строкой, массив needle'ов, реестр кортежей,
// WINDOW, самопроверочный блок с голыми числами и сценарий поведения ВНЕ
// области с голым числом.
const FIXTURE = `// fixture (fictional) shaped like the write-site gate
fn scenario_before_area() {
    assert_eq!(
        sample_rows(),
        3,
        "behaviour scenario before the guarded area"
    );
}

const WRITE_SITE_CALLS: [&str; 2] = [
    "alpha_in_tx(",
    "beta_in_tx(",
];
const RELEASE_CALLS: [&str; 2] = ["release_a(", "release_b("];

const WRITE_SITE_REGISTRY: [(&str, usize); 2] = [
    ("src/one.rs", 8),
    ("src/two.rs", 1),
];

fn counter_selftest() {
    assert_eq!(
        count_calls(fixture, &WRITE_SITE_CALLS),
        2,
        "self-check block"
    );
}

fn every_site_has_release() {
    assert_eq!(
        count_calls(src, &WRITE_SITE_CALLS),
        8,
        "write-site count"
    );
    assert_eq!(
        count_calls(src, &RELEASE_CALLS),
        8,
        "release count"
    );
    const WINDOW: usize = 14;
    let mut paired = 0;
    assert_eq!(paired, 8, "paired sites");
}

fn s11_anchor_guards_present() {
    let _ = "anchor_group_of_device_in_tx(";
}

fn scenario_after_area() {
    assert_eq!(
        other_rows(),
        5,
        "behaviour scenario after the guarded area"
    );
}
`;

function mutate(text, from, to) {
  const count = text.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`self-test: якорь мутанта должен встречаться ровно один раз, найдено ${count}: ${JSON.stringify(from)}`);
  }
  return text.replace(from, () => to);
}

function runSelftest() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wsg-selftest-'));
  const failures = [];
  try {
    const g = (...args) => git(dir, args);
    g('init', '-q');
    g('config', 'user.email', 'selftest@example.invalid');
    g('config', 'user.name', 'selftest');
    g('config', 'commit.gpgsign', 'false');
    fs.mkdirSync(path.join(dir, 'tests'), { recursive: true });
    const abs = path.join(dir, FIXTURE_FILE);
    fs.writeFileSync(abs, FIXTURE);
    fs.writeFileSync(path.join(dir, 'tests/no_anchor.rs'), 'fn nothing() {}\n');
    g('add', '.');
    g('commit', '-q', '-m', 'fixture');
    const base = g('rev-parse', 'HEAD').trim();

    const reset = () => fs.writeFileSync(abs, FIXTURE);
    const weakenedOf = (mutated) => {
      fs.writeFileSync(abs, mutated);
      const r = checkFile(dir, base, FIXTURE_FILE);
      reset();
      return r;
    };

    const mutants = [
      ['write-site count literal 8 -> 7 on its own line', mutate(FIXTURE, '        8,\n        "write-site count"', '        7,\n        "write-site count"')],
      ['release count literal 8 -> 7 on its own line', mutate(FIXTURE, '        8,\n        "release count"', '        7,\n        "release count"')],
      ['registry tuple number changed', mutate(FIXTURE, '("src/one.rs", 8)', '("src/one.rs", 9)')],
      ['WINDOW 14 -> 13', mutate(FIXTURE, 'const WINDOW: usize = 14;', 'const WINDOW: usize = 13;')],
      ['paired, 8 -> paired, 7', mutate(FIXTURE, 'assert_eq!(paired, 8,', 'assert_eq!(paired, 7,')],
      [
        'needle line removed together with array length',
        mutate(
          mutate(FIXTURE, '    "beta_in_tx(",\n', ''),
          'const WRITE_SITE_CALLS: [&str; 2]',
          'const WRITE_SITE_CALLS: [&str; 1]',
        ),
      ],
      [
        'RELEASE_CALLS shortened',
        mutate(FIXTURE, 'const RELEASE_CALLS: [&str; 2] = ["release_a(", "release_b("];', 'const RELEASE_CALLS: [&str; 1] = ["release_a("];'),
      ],
      ['bare number in the self-check block', mutate(FIXTURE, '        2,\n        "self-check block"', '        3,\n        "self-check block"')],
      ['fn s11_ renamed', mutate(FIXTURE, 'fn s11_anchor_guards_present', 'fn s12_anchor_guards_present')],
      ['anchor guard reference removed', mutate(FIXTURE, '    let _ = "anchor_group_of_device_in_tx(";\n', '')],
    ];

    let caught = 0;
    for (const [name, mutated] of mutants) {
      const r = weakenedOf(mutated);
      if (r.code === 1 && r.weakened.length >= 1) {
        caught += 1;
        console.log(`  caught  ${name} (weakened: ${r.weakened.length})`);
      } else {
        failures.push(`mutant NOT caught: ${name} (code ${r.code}, weakened ${r.weakened.length})`);
      }
    }

    const controls = [
      ['only added lines', FIXTURE + '\nfn appended() {\n    assert_eq!(\n        x(),\n        7,\n        "new"\n    );\n}\n'],
      ['bare number changed in behaviour scenario outside the area', mutate(FIXTURE, '        5,\n        "behaviour scenario after the guarded area"', '        6,\n        "behaviour scenario after the guarded area"')],
      ['bare number changed in behaviour scenario before the area', mutate(FIXTURE, '        3,\n        "behaviour scenario before the guarded area"', '        4,\n        "behaviour scenario before the guarded area"')],
      ['empty diff', FIXTURE],
    ];
    let clean = 0;
    for (const [name, content] of controls) {
      const r = weakenedOf(content);
      if (r.code === 0 && r.weakened.length === 0) {
        clean += 1;
        console.log(`  clean   ${name}`);
      } else {
        failures.push(`control NOT clean: ${name} (code ${r.code}, weakened ${r.weakened.length})`);
      }
    }

    const missing = checkFile(dir, base, 'tests/no_anchor.rs');
    if (missing.code === 2) {
      console.log('  code 2  no anchor in base');
    } else {
      failures.push(`no-anchor case must give code 2, got ${missing.code}`);
    }
    const absent = checkFile(dir, base, 'tests/does_not_exist.rs');
    if (absent.code === 2) {
      console.log('  code 2  file absent in base');
    } else {
      failures.push(`absent-file case must give code 2, got ${absent.code}`);
    }

    console.log(`${TAG} selftest: mutants caught ${caught}/${mutants.length}, controls clean ${clean}/${controls.length}`);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
  if (failures.length > 0) {
    for (const f of failures) console.error(`${TAG} FAIL: ${f}`);
    return 1;
  }
  console.log(`${TAG} selftest PASS`);
  return 0;
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function main(argv) {
  if (argv.includes('--selftest')) return runSelftest();
  const [base, file] = argv.filter((a) => !a.startsWith('--'));
  if (!base) {
    console.error(`${TAG} usage: check-write-site-gate-untouched.mjs <BASE> [file] | --selftest`);
    return 2;
  }
  const relFile = file ?? DEFAULT_FILE;
  const result = checkFile(REPO_ROOT, base, relFile);
  if (result.code === 2) {
    console.error(`${TAG} ${result.message}`);
    return 2;
  }
  console.log(`weakened: ${result.weakened.length}`);
  for (const w of result.weakened) console.log(`${w.oldLine}: ${w.text}`);
  return result.code;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) process.exit(main(process.argv.slice(2)));
