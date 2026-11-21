#!/usr/bin/env node
// [check-write-off-choice] Гейт правила D-26 «выбор при списании участника группы
// через форму устройства» (фаза 41.8, план 22; GRW-02).
//
// Слои.
//  1. golden: чистый модуль src/features/devices/writeOffChoice.ts транспилируется
//     изолированно (он без импортов) и гоняется по
//     scripts/fixtures/write-off-choice/cases.json; каждый кейс подобран так,
//     чтобы потеря одного условия (участник, якорь, «уже списано») меняла результат;
//  2. паритет кода статуса: локальная константа WRITTEN_OFF_CODE в модуле равна
//     STATUS_WRITTEN_OFF, экспортируемой deviceRowMenu.ts (модуль без импортов
//     не может её импортировать, расхождение отдельно ловится здесь);
//  3. структура: DeviceFormBody.svelte (вне комментариев) зовёт `writeOffChoiceNeeded(`
//     и содержит `group_write_off` не меньше двух раз (первичный выбор и повтор
//     после серверного VALIDATION).
// Сервер остаётся границей безопасности; UI лишь заранее предлагает выбор.
//
// Self-test (--selftest): мутанты реального исходника (якорь каждого встречается
// ровно один раз) и структурные мутанты; у каждого «до» зелёная, «после» красная.
//
// Usage:
//   node scripts/check-write-off-choice.mjs
//   node scripts/check-write-off-choice.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-write-off-choice]';
const MODULE_FILE = 'src/features/devices/writeOffChoice.ts';
const MENU_FILE = 'src/features/devices/deviceRowMenu.ts';
const FORM_FILE = 'src/features/devices/DeviceFormBody.svelte';
const FIXTURE = path.join(__dirname, 'fixtures/write-off-choice/cases.json');
const MIN_CASES = 10;

const read = (rel) => fs.readFileSync(path.join(UI_ROOT, rel), 'utf8');

async function loadModule(tsSource) {
  const require = createRequire(import.meta.url);
  const ts = require('typescript');
  const js = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    reportDiagnostics: false,
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

async function checkGolden(tsSource, cases) {
  const failures = [];
  if (!Array.isArray(cases) || cases.length < MIN_CASES) {
    return [`в фикстуре меньше ${MIN_CASES} кейсов`];
  }
  if (/^\s*import\s/m.test(tsSource)) failures.push('writeOffChoice.ts содержит import');
  let mod;
  try {
    mod = await loadModule(tsSource);
  } catch (e) {
    return [`модуль не загрузился: ${e.message}`];
  }
  if (typeof mod.writeOffChoiceNeeded !== 'function') {
    return ['модуль не экспортирует writeOffChoiceNeeded'];
  }
  for (const c of cases) {
    const got = mod.writeOffChoiceNeeded({ ...c.input });
    if (got !== c.expected) failures.push(`${c.name}: получено ${got}, ожидалось ${c.expected}`);
  }
  return failures;
}

function checkParity(modSrc, menuSrc) {
  const a = /const\s+WRITTEN_OFF_CODE\s*=\s*'([^']*)'/.exec(stripComments(modSrc));
  const b = /export\s+const\s+STATUS_WRITTEN_OFF\s*=\s*'([^']*)'/.exec(stripComments(menuSrc));
  if (!a) return ["в writeOffChoice.ts нет const WRITTEN_OFF_CODE = '…'"];
  if (!b) return ["в deviceRowMenu.ts нет export const STATUS_WRITTEN_OFF = '…'"];
  if (a[1] !== b[1]) {
    return [`WRITTEN_OFF_CODE «${a[1]}» не равен STATUS_WRITTEN_OFF «${b[1]}»`];
  }
  return [];
}

function checkForm(formSrc) {
  const code = stripComments(formSrc);
  const failures = [];
  if (!code.includes('writeOffChoiceNeeded(')) {
    failures.push('DeviceFormBody не зовёт writeOffChoiceNeeded(');
  }
  const n = code.split('group_write_off').length - 1;
  if (n < 2) failures.push(`в DeviceFormBody group_write_off встречается ${n} раз, нужно >= 2`);
  return failures;
}

const count = (s, needle) => s.split(needle).length - 1;
function mutate(src, from, to) {
  if (count(src, from) !== 1)
    throw new Error(`якорь мутации встречается ${count(src, from)} раз: ${from}`);
  return src.replace(from, to);
}

async function run(modSrc, menuSrc, formSrc, cases) {
  return [
    ...(await checkGolden(modSrc, cases)),
    ...checkParity(modSrc, menuSrc),
    ...checkForm(formSrc),
  ];
}

async function selftest() {
  const modSrc = read(MODULE_FILE);
  const menuSrc = read(MENU_FILE);
  const formSrc = read(FORM_FILE);
  const cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  const bad = [];
  const base = await run(modSrc, menuSrc, formSrc, cases);
  if (base.length) bad.push(`до мутаций красно: ${base.join('; ')}`);

  const mutants = [
    [
      'снята проверка якоря',
      'mod',
      "if (input.anchorGroupId !== null) return 'anchor_refuse';",
      '',
    ],
    ['снята проверка не-члена', 'mod', "if (input.groupId === null) return 'none';", ''],
    [
      'снята проверка «уже списано»',
      'mod',
      "if (input.statusCodeBefore === WRITTEN_OFF_CODE) return 'none';",
      '',
    ],
    [
      'снята проверка «статус после»',
      'mod',
      "if (input.statusCodeAfter !== WRITTEN_OFF_CODE) return 'none';",
      '',
    ],
    [
      'расхождение кода статуса',
      'mod',
      "const WRITTEN_OFF_CODE = 'списано';",
      "const WRITTEN_OFF_CODE = 'списан';",
    ],
    ['форма без вызова правила', 'form', 'writeOffChoiceNeeded(', 'writeOffChoiceNeededX('],
  ];
  for (const [name, which, from, to] of mutants) {
    let m = modSrc;
    let f = formSrc;
    try {
      if (which === 'mod') m = mutate(modSrc, from, to);
      else f = mutate(formSrc, from, to);
    } catch (e) {
      bad.push(`${name}: ${e.message}`);
      continue;
    }
    const r = await run(m, menuSrc, f, cases);
    if (r.length === 0) bad.push(`мутант «${name}» не пойман`);
  }
  // Форма без group_write_off (все вхождения).
  const noField = formSrc.split('group_write_off').join('group_write_of');
  if ((await run(modSrc, menuSrc, noField, cases)).length === 0) {
    bad.push('мутант «форма без group_write_off» не пойман');
  }
  // Расхождение со стороны deviceRowMenu.ts.
  const menuMut = mutate(
    menuSrc,
    "export const STATUS_WRITTEN_OFF = 'списано';",
    "export const STATUS_WRITTEN_OFF = 'списан';",
  );
  if ((await run(modSrc, menuMut, formSrc, cases)).length === 0) {
    bad.push('мутант «STATUS_WRITTEN_OFF разошёлся» не пойман');
  }
  if (bad.length) {
    console.error(`${TAG} SELFTEST FAIL\n - ${bad.join('\n - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK (${mutants.length + 2} мутантов пойманы)`);
}

if (process.argv.includes('--selftest')) {
  await selftest();
} else {
  const cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  const r = await run(read(MODULE_FILE), read(MENU_FILE), read(FORM_FILE), cases);
  if (r.length) {
    console.error(`${TAG} FAIL\n - ${r.join('\n - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK (${cases.length} кейсов)`);
}
