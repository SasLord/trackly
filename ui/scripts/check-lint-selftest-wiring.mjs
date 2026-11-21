#!/usr/bin/env node
// [check-lint-selftest-wiring] Мета-гейт подключения гейтов в `pnpm lint`
// (quick 261121-heg, WR-13).
//
// ЗАЧЕМ. Гейт `scripts/check-*.mjs`, который не вызывается в `pnpm lint`, не
// защищает ничего — а «зелёный» CI создаёт видимость защиты. Так же и selftest:
// гейт без прогона `--selftest` может молча ослабнуть (регулярка перестала
// ловить, мутант сломался), и этого никто не увидит. До этого пакета подключение
// держалось только дисциплиной; ослабление было невидимо.
//
// Читает НАСТОЯЩИЕ исходники: `scripts.lint` из package.json и каждый
// scripts/check-*.mjs (инвентарь от каталога, не вкомпилированный список).
// Проверки:
//   [W0] гейтов найдено > 0 и scripts.lint разобран (защита от вакуума);
//   [W1] каждый scripts/check-*.mjs вызывается в lint простым прогоном
//        `node scripts/<file>` (без `--selftest`);
//   [W2] если в исходнике гейта есть литерал '--selftest' ВНЕ комментариев
//        (`_strip.mjs`), в lint обязан быть `node scripts/<file> --selftest`.
//
// Чего гейт НЕ видит (честно): что selftest действительно что-то проверяет (это
// ответственность самого гейта); вызов через другую обёртку, не
// `node scripts/<file>`; ветку selftest, включаемую иным флагом.
//
// Корневые scripts/check-privacy.selftest.mjs и
// scripts/check-write-site-gate-untouched.mjs --selftest — вне ui/, их
// подключение держит шаг ci-fast.yml.
//
// Usage:
//   node scripts/check-lint-selftest-wiring.mjs                # реальный ui/
//   node scripts/check-lint-selftest-wiring.mjs --src=<dir>    # копия каталога ui/
//   node scripts/check-lint-selftest-wiring.mjs --selftest     # мутанты копии реального ui/
//
// Zero-dependency: только node:fs/node:os/node:path/node:url.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-lint-selftest-wiring]';
const UI_ROOT = path.resolve(__dirname, '..');
const GATE_RE = /^check-[\w.-]+\.mjs$/;

export function check(uiRoot) {
  const failures = [];
  let lint = null;
  try {
    const pkg = JSON.parse(fs.readFileSync(path.join(uiRoot, 'package.json'), 'utf8'));
    lint = typeof pkg?.scripts?.lint === 'string' ? pkg.scripts.lint : null;
  } catch (e) {
    failures.push(`[W0] package.json не читается: ${e.message}`);
  }
  if (lint === null || lint.trim() === '') {
    failures.push('[W0] в package.json нет scripts.lint');
    return { failures, gates: [] };
  }
  const commands = lint.split('&&').map((c) => c.trim().replace(/\s+/g, ' '));
  const scriptsDir = path.join(uiRoot, 'scripts');
  const gates = fs
    .readdirSync(scriptsDir)
    .filter((f) => GATE_RE.test(f))
    .sort();
  if (gates.length === 0) {
    failures.push('[W0] в scripts/ нет ни одного check-*.mjs — разбор пуст');
  }
  for (const f of gates) {
    const base = `node scripts/${f}`;
    const plain = commands.some(
      (c) => c === base || (c.startsWith(`${base} `) && !c.includes('--selftest')),
    );
    if (!plain) failures.push(`[W1] ${f}: нет простого прогона «${base}» в pnpm lint`);
    const code = stripComments(fs.readFileSync(path.join(scriptsDir, f), 'utf8'));
    if (code.includes('--selftest')) {
      const st = commands.some(
        (c) => c === `${base} --selftest` || c.startsWith(`${base} --selftest `),
      );
      if (!st) {
        failures.push(`[W2] ${f}: есть ветка --selftest, но «${base} --selftest» в pnpm lint нет`);
      }
    }
  }
  return { failures, gates };
}

// ---------------------------------------------------------------------------
// Self-test: мутанты копии реального ui/ (package.json + scripts/)
// ---------------------------------------------------------------------------

function copyUi(from) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'check-lint-wiring-'));
  fs.copyFileSync(path.join(from, 'package.json'), path.join(tmp, 'package.json'));
  fs.mkdirSync(path.join(tmp, 'scripts'));
  for (const f of fs.readdirSync(path.join(from, 'scripts'))) {
    const p = path.join(from, 'scripts', f);
    if (fs.statSync(p).isFile()) fs.copyFileSync(p, path.join(tmp, 'scripts', f));
  }
  return tmp;
}

function mutateLint(root, from, to) {
  const p = path.join(root, 'package.json');
  const src = fs.readFileSync(p, 'utf8');
  const count = src.split(from).length - 1;
  if (count !== 1)
    throw new Error(`якорь в package.json встречается ${count} раз, нужен 1: ${from}`);
  fs.writeFileSync(p, src.replace(from, to));
}

const MUTANTS = [
  {
    name: 'из lint убран «check-reorder.mjs --selftest»',
    apply: (root) => mutateLint(root, 'node scripts/check-reorder.mjs --selftest && ', ''),
    code: '[W2]',
  },
  {
    name: 'новый scripts/check-zz-new.mjs с веткой --selftest не подключён',
    apply: (root) =>
      fs.writeFileSync(
        path.join(root, 'scripts', 'check-zz-new.mjs'),
        "if (process.argv.includes('--selftest')) process.exit(0);\n",
      ),
    code: '[W1]',
  },
  {
    name: 'из lint убран простой прогон check-tokens.mjs (гейт без selftest)',
    apply: (root) => mutateLint(root, 'node scripts/check-tokens.mjs && ', ''),
    code: '[W1]',
  },
  {
    name: 'гейт с простым прогоном получил ветку --selftest, а lint нет',
    apply: (root) =>
      fs.appendFileSync(
        path.join(root, 'scripts', 'check-focus-outline.mjs'),
        "\nif (process.argv.includes('--selftest')) process.exit(0);\n",
      ),
    code: '[W2]',
  },
  {
    name: 'в scripts/ не осталось гейтов (вакуум)',
    apply: (root) => {
      for (const f of fs.readdirSync(path.join(root, 'scripts'))) {
        if (GATE_RE.test(f)) fs.rmSync(path.join(root, 'scripts', f));
      }
    },
    code: '[W0]',
  },
];

function selftest(from) {
  const problems = [];
  const control = copyUi(from);
  try {
    const res = check(control);
    if (res.failures.length !== 0) {
      problems.push(`контроль (немутированная копия) красный: ${res.failures.join(' | ')}`);
    }
  } finally {
    fs.rmSync(control, { recursive: true, force: true });
  }
  for (const mu of MUTANTS) {
    const tmp = copyUi(from);
    try {
      mu.apply(tmp);
      const res = check(tmp);
      if (!res.failures.some((f) => f.startsWith(mu.code))) {
        problems.push(
          `мутант «${mu.name}» не пойман кодом ${mu.code}: ${res.failures.join(' | ') || 'зелёный'}`,
        );
      } else {
        console.log(`${TAG} мутант «${mu.name}»: пойман ${mu.code}`);
      }
    } catch (e) {
      problems.push(`мутант «${mu.name}»: ${e.message}`);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  }
  return problems;
}

function main() {
  const srcArg = process.argv.find((a) => a.startsWith('--src='));
  const root = srcArg ? path.resolve(srcArg.slice('--src='.length)) : UI_ROOT;
  if (process.argv.includes('--selftest')) {
    const problems = selftest(root);
    if (problems.length > 0) {
      console.error(`${TAG} SELFTEST FAIL:`);
      for (const p of problems) console.error(`  ${p}`);
      process.exit(1);
    }
    console.log(`${TAG} selftest OK — контроль зелёный, ${MUTANTS.length} мутантов красные`);
    return;
  }
  const { failures, gates } = check(root);
  if (failures.length > 0) {
    console.error(`${TAG} FAIL:`);
    for (const f of failures) console.error(`  ${f}`);
    process.exit(1);
  }
  console.log(`${TAG} OK — ${gates.length} гейтов подключены в pnpm lint, selftest-ветки тоже`);
}

main();
