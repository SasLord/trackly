#!/usr/bin/env node
// [check-device-status-menu] Гейт правил подменю «Статус» (фаза 41.8, план 29; D-35, D-26).
//
// Слои.
//  1. golden: чистый модуль src/features/devices/deviceStatusMenu.ts (без импортов)
//     транспилируется изолированно и гоняется по
//     scripts/fixtures/device-status-menu/cases.json;
//  2. паритеты локальных литералов модуля с источниками правды:
//     код «списано» = STATUS_WRITTEN_OFF (deviceRowMenu.ts); роли = MUTATE_ROLES
//     (deviceRowMenu.ts); текст причины якоря = anchorWriteOffRefusal (writeOffTexts.ts,
//     вызов на одном имени группы); текст причины ремонтной заявки = REPAIR_STATUS_NOTE
//     (repairLocks.ts, литерал читается регулярным выражением);
//  3. структура: deviceStatusMenu.ts не содержит import.
// Сервер остаётся границей безопасности; UI лишь заранее делает варианты неактивными.
//
// Self-test (--selftest): мутанты реального исходника (якорь каждого встречается
// ровно один раз); у каждого «до» зелёная, «после» красная.
//
// Usage:
//   node scripts/check-device-status-menu.mjs
//   node scripts/check-device-status-menu.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-status-menu]';
const MODULE_FILE = 'src/features/devices/deviceStatusMenu.ts';
const MENU_FILE = 'src/features/devices/deviceRowMenu.ts';
const TEXTS_FILE = 'src/features/groups/writeOffTexts.ts';
const REPAIR_FILE = 'src/features/requests/repairLocks.ts';
const FIXTURE = path.join(__dirname, 'fixtures/device-status-menu/cases.json');
const MIN_CASES = 15;
const FUNCTIONS = ['statusMenuOptions', 'canChangeStatus'];

const read = (rel) => fs.readFileSync(path.join(UI_ROOT, rel), 'utf8');
const canonical = (v) => JSON.stringify(v);

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
  if (/^\s*import\s/m.test(stripComments(tsSource))) {
    failures.push('deviceStatusMenu.ts содержит import');
  }
  let mod;
  try {
    mod = await loadModule(tsSource);
  } catch (e) {
    return [`модуль не загрузился: ${e.message}`];
  }
  for (const fn of FUNCTIONS) {
    if (typeof mod[fn] !== 'function') return [`модуль не экспортирует ${fn}`];
  }
  const covered = new Set();
  for (const c of cases) {
    covered.add(c.fn);
    if (!FUNCTIONS.includes(c.fn)) {
      failures.push(`${c.name}: неизвестная функция «${c.fn}»`);
      continue;
    }
    const input = JSON.parse(canonical(c.input));
    const got = mod[c.fn](input);
    if (canonical(input) !== canonical(c.input)) failures.push(`${c.name}: мутировал вход`);
    if (canonical(got) !== canonical(c.expected)) {
      failures.push(`${c.name}: получено ${canonical(got)}, ожидалось ${canonical(c.expected)}`);
    }
  }
  for (const fn of FUNCTIONS) {
    if (!covered.has(fn)) failures.push(`фикстура не покрывает ${fn}`);
  }
  return failures;
}

async function checkParity(modSrc, menuSrc, textsSrc, repairSrc) {
  const failures = [];
  const code = stripComments(modSrc);
  const a = /const\s+WRITTEN_OFF_CODE\s*=\s*'([^']*)'/.exec(code);
  const b = /export\s+const\s+STATUS_WRITTEN_OFF\s*=\s*'([^']*)'/.exec(stripComments(menuSrc));
  if (!a) failures.push("в deviceStatusMenu.ts нет const WRITTEN_OFF_CODE = '…'");
  else if (!b) failures.push("в deviceRowMenu.ts нет export const STATUS_WRITTEN_OFF = '…'");
  else if (a[1] !== b[1]) {
    failures.push(`WRITTEN_OFF_CODE «${a[1]}» не равен STATUS_WRITTEN_OFF «${b[1]}»`);
  }

  const ra = /const\s+STATUS_MENU_ROLES\s*=\s*\[([^\]]*)\]/.exec(code);
  const rb = /const\s+MUTATE_ROLES\s*=\s*\[([^\]]*)\]/.exec(stripComments(menuSrc));
  if (!ra) failures.push('в deviceStatusMenu.ts нет const STATUS_MENU_ROLES = […]');
  else if (!rb) failures.push('в deviceRowMenu.ts нет const MUTATE_ROLES = […]');
  else {
    const norm = (s) =>
      s
        .replace(/['"\s]/g, '')
        .split(',')
        .filter(Boolean)
        .sort()
        .join(',');
    if (norm(ra[1]) !== norm(rb[1])) {
      failures.push(`STATUS_MENU_ROLES [${norm(ra[1])}] не равен MUTATE_ROLES [${norm(rb[1])}]`);
    }
  }

  const rr = /const\s+REPAIR_REASON\s*=\s*'([^']*)'/.exec(code);
  const rn = /export\s+const\s+REPAIR_STATUS_NOTE\s*=\s*'([^']*)'/.exec(stripComments(repairSrc));
  if (!rr) failures.push("в deviceStatusMenu.ts нет const REPAIR_REASON = '…'");
  else if (!rn) failures.push("в repairLocks.ts нет export const REPAIR_STATUS_NOTE = '…'");
  else if (rr[1] !== rn[1]) {
    failures.push(`REPAIR_REASON «${rr[1]}» не равен REPAIR_STATUS_NOTE «${rn[1]}»`);
  }

  try {
    const mod = await loadModule(modSrc);
    const texts = await loadModule(textsSrc);
    const input = {
      statuses: [{ id: 1, code: a ? a[1] : 'списано', name: 'X' }],
      currentStatusId: null,
      isAnchor: true,
      anchorGroupName: 'АРМ #3',
    };
    const got = mod.statusMenuOptions(input)[0]?.disabledReason;
    const want = texts.anchorWriteOffRefusal('АРМ #3');
    if (got !== want)
      failures.push(`причина якоря «${got}» не равна anchorWriteOffRefusal «${want}»`);
  } catch (e) {
    failures.push(`паритет причины якоря не проверен: ${e.message}`);
  }
  return failures;
}

const count = (s, needle) => s.split(needle).length - 1;
function mutate(src, from, to) {
  if (count(src, from) !== 1)
    throw new Error(`якорь мутации встречается ${count(src, from)} раз: ${from}`);
  return src.replace(from, to);
}

async function run(srcs, cases) {
  return [
    ...(await checkGolden(srcs.mod, cases)),
    ...(await checkParity(srcs.mod, srcs.menu, srcs.texts, srcs.repair)),
  ];
}

async function selftest() {
  const srcs = {
    mod: read(MODULE_FILE),
    menu: read(MENU_FILE),
    texts: read(TEXTS_FILE),
    repair: read(REPAIR_FILE),
  };
  const cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  const bad = [];
  const base = await run(srcs, cases);
  if (base.length) bad.push(`до мутаций красно: ${base.join('; ')}`);

  const mutants = [
    ['снято условие якоря', 'mod', 'input.isAnchor && s.code === WRITTEN_OFF_CODE', 'false'],
    [
      'якорь блокирует любой статус',
      'mod',
      'input.isAnchor && s.code === WRITTEN_OFF_CODE',
      'input.isAnchor',
    ],
    ['текущий не отмечается', 'mod', 'current: s.id === input.currentStatusId,', 'current: false,'],
    [
      'роли расширены на employee',
      'mod',
      "['admin', 'manager']",
      "['admin', 'manager', 'employee']",
    ],
    [
      'порядок перепутан',
      'mod',
      'return input.statuses.map(',
      'return [...input.statuses].reverse().map(',
    ],
    ['снята ветка открытой заявки', 'mod', 'if (repairLocked) {', 'if (false) {'],
    [
      'заявка приоритетнее не стоит: якорь раньше',
      'mod',
      '    if (repairLocked) {\n      disabled = true;\n      disabledReason = REPAIR_REASON;\n    } else if (input.isAnchor && s.code === WRITTEN_OFF_CODE) {',
      '    if (input.isAnchor && s.code === WRITTEN_OFF_CODE) {\n      disabled = true;\n      disabledReason = anchorReason(input.anchorGroupName);\n    } else if (repairLocked) {',
    ],
    [
      'null считается заявкой',
      'mod',
      'input.openRepairRequestId !== null && input.openRepairRequestId !== undefined',
      'input.openRepairRequestId !== undefined',
    ],
    [
      'код статуса разошёлся',
      'mod',
      "const WRITTEN_OFF_CODE = 'списано';",
      "const WRITTEN_OFF_CODE = 'списан';",
    ],
    [
      'текст заявки разошёлся',
      'mod',
      "'Статус меняется заявкой на ремонт.'",
      "'Статус меняется заявкой.'",
    ],
    ['текст якоря разошёлся', 'mod', 'спишите его через группу', 'спишите через группу'],
    [
      'добавлен import',
      'mod',
      'const WRITTEN_OFF_CODE',
      "import { x } from './y';\nconst WRITTEN_OFF_CODE",
    ],
    [
      'MUTATE_ROLES разошёлся',
      'menu',
      "const MUTATE_ROLES = ['admin', 'manager'];",
      "const MUTATE_ROLES = ['admin'];",
    ],
    [
      'STATUS_WRITTEN_OFF разошёлся',
      'menu',
      "export const STATUS_WRITTEN_OFF = 'списано';",
      "export const STATUS_WRITTEN_OFF = 'списан';",
    ],
    [
      'REPAIR_STATUS_NOTE разошёлся',
      'repair',
      "'Статус меняется заявкой на ремонт.'",
      "'Статус меняется заявкой.'",
    ],
  ];
  for (const [name, which, from, to] of mutants) {
    const s = { ...srcs };
    try {
      s[which] = mutate(srcs[which], from, to);
    } catch (e) {
      bad.push(`${name}: ${e.message}`);
      continue;
    }
    const r = await run(s, cases);
    if (r.length === 0) bad.push(`мутант «${name}» не пойман`);
  }
  if (bad.length) {
    console.error(`${TAG} SELFTEST FAIL\n - ${bad.join('\n - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK (${mutants.length} мутантов пойманы)`);
}

if (process.argv.includes('--selftest')) {
  await selftest();
} else {
  const cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  const srcs = {
    mod: read(MODULE_FILE),
    menu: read(MENU_FILE),
    texts: read(TEXTS_FILE),
    repair: read(REPAIR_FILE),
  };
  const r = await run(srcs, cases);
  if (r.length) {
    console.error(`${TAG} FAIL\n - ${r.join('\n - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK (${cases.length} кейсов)`);
}
