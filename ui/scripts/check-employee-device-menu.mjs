#!/usr/bin/env node
// [check-employee-device-menu] Гейт меню строки устройства сотрудника (фаза 41.6,
// план 08; D-09, D-10, D-11).
//
// Слой 1 (golden, исполняемый): чистый модуль src/features/employee/employeeDeviceMenu.ts
//   транспилируется пакетом typescript и гоняется по fixtures/employee-device-menu/cases.json.
//   Каждый кейс подобран так, чтобы реальная ошибка (охрана принтера, подсказка, порядок
//   заявок, порядок секций, охрана участия, фильтр пустого номера) меняла результат.
// Слой 2 (структура, комментарии вырезаны):
//   [M1] в модуле нет runtime-импортов (допустим только `import type`);
//   [M2] в src/features/employee/** нет импорта DeviceRowMenu (staff-only, D-09);
//   [M3] в фикстуре не меньше MIN_CASES golden-кейсов.
// --selftest: мутанты РЕАЛЬНОГО модуля (якорь встречается ровно один раз) и фикстурные
//   мутанты структурных правил обязаны краснеть.
//
// Usage: node scripts/check-employee-device-menu.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';
import { walk as walkRoots } from './_roots.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-employee-device-menu]';
const MODULE_FILE = path.join(UI_ROOT, 'src/features/employee/employeeDeviceMenu.ts');
const FIXTURE = path.join(__dirname, 'fixtures/employee-device-menu/cases.json');
const MIN_CASES = 10;

function loadTypescript() {
  const require = createRequire(import.meta.url);
  try {
    return require('typescript');
  } catch {
    console.error(`${TAG} FAIL - не разрешается пакет typescript; запусти pnpm install в ui/.`);
    process.exit(1);
  }
}

async function loadModuleFromTs(tsSource, label) {
  const ts = loadTypescript();
  const js = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: label,
    reportDiagnostics: false,
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

function canonical(v) {
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v)
      .filter((k) => v[k] !== undefined)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v);
}

function loadCases() {
  let cases;
  try {
    cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  } catch (e) {
    console.error(`${TAG} FAIL - фикстура не читается: ${e.message}`);
    process.exit(1);
  }
  return cases;
}

function runGolden(mod, cases) {
  const v = [];
  if (typeof mod.buildEmployeeMenu !== 'function') {
    return ['[G] модуль не экспортирует buildEmployeeMenu'];
  }
  for (const c of cases) {
    const before = canonical(c.input);
    let got;
    try {
      got = mod.buildEmployeeMenu(JSON.parse(before));
    } catch (e) {
      v.push(`[G] ${c.name}: исключение ${e.message}`);
      continue;
    }
    if (!Array.isArray(got) || got.some((s) => !Array.isArray(s) || s.length === 0)) {
      v.push(`[G] ${c.name}: пустая секция или не массив секций: ${canonical(got)}`);
      continue;
    }
    if (canonical(got) !== canonical(c.expected)) {
      v.push(`[G] ${c.name}: ожидалось ${canonical(c.expected)}, получено ${canonical(got)}`);
    }
    // Функция не должна мутировать вход (sort на оригинале переставил бы заявки строки).
    const probe = JSON.parse(before);
    mod.buildEmployeeMenu(probe);
    if (canonical(probe) !== before) v.push(`[G] ${c.name}: функция мутировала вход`);
  }
  return v;
}

function structural(moduleSrc, employeeFiles, caseCount) {
  const v = [];
  const mod = stripComments(moduleSrc);
  if (/^\s*import\s+(?!type\b)/m.test(mod) || /\bimport\s*\(|\brequire\s*\(/.test(mod)) {
    v.push('[M1] в employeeDeviceMenu.ts есть runtime-импорт (допустим только import type)');
  }
  for (const f of employeeFiles) {
    const src = stripComments(f.src);
    if (/\bDeviceRowMenu\b/.test(src)) {
      v.push(`[M2] ${f.name}: упоминание DeviceRowMenu (staff-only меню, D-09)`);
    }
  }
  if (caseCount < MIN_CASES) {
    v.push(`[M3] в фикстуре ${caseCount} кейсов, нужно не меньше ${MIN_CASES}`);
  }
  return v;
}

function readEmployeeFiles() {
  const out = [];
  for (const p of walkRoots({ exts: ['.svelte', '.ts'], subdirs: ['features/employee'] })) {
    out.push({ name: path.relative(UI_ROOT, p), src: fs.readFileSync(p, 'utf8') });
  }
  return out;
}

async function runAll(moduleSrc, employeeFiles, cases) {
  const mod = await loadModuleFromTs(moduleSrc, 'employeeDeviceMenu.ts');
  return [...runGolden(mod, cases), ...structural(moduleSrc, employeeFiles, cases.length)];
}

function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL - якорь мутанта «${label}» встречается ${n} раз, нужен ровно 1 (неуникальный якорь снимает гейт у соседней функции)`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

async function selftest() {
  const cases = loadCases();
  const moduleSrc = fs.readFileSync(MODULE_FILE, 'utf8');
  const files = readEmployeeFiles();
  const clean = await runAll(moduleSrc, files, cases);
  if (clean.length) {
    console.error(`${TAG} SELFTEST FAIL - чистый исходник красный:\n${clean.join('\n')}`);
    process.exit(1);
  }
  const m = (from, to, label) => [label, mutate(moduleSrc, from, to, label)];
  const moduleMutants = [
    m('if (row.isPrinter) second.push(', 'if (true) second.push(', 'снята охрана isPrinter'),
    m('if (row.inRepair) repair.hint = JOIN_REPAIR_HINT;', '', 'снято условие подсказки'),
    m(
      "export const JOIN_REPAIR_HINT = 'Уже в ремонте — вы присоединитесь к заявке';",
      "export const JOIN_REPAIR_HINT = 'Уже в ремонте по заявке №1 — вы присоединитесь';",
      'номер чужой заявки в подсказке',
    ),
    m(
      '[...row.myOpenRequests].sort((a, b) => a.id - b.id)',
      '[...row.myOpenRequests]',
      'снят sort по id',
    ),
    m(
      'return [first, second, third].filter',
      'return [second, first, third].filter',
      'перестановка секций',
    ),
    m('if (row.myRepairRequestId === null) {', 'if (true) {', 'убрана охрана участия в ремонте'),
    m(
      "if ((row.inventoryNumber ?? '').trim() !== '') {",
      'if (true) {',
      'убран фильтр пустого инв. номера',
    ),
    m('.filter((s) => s.length > 0)', '', 'убран фильтр пустых секций'),
    m('[...row.myOpenRequests].sort(', 'row.myOpenRequests.sort(', 'sort мутирует вход'),
  ];
  let bad = 0;
  for (const [name, src] of moduleMutants) {
    const v = await runAll(src, files, cases);
    if (v.length) console.log(`${TAG} ПОЙМАН: ${name}`);
    else {
      console.error(`${TAG} SELFTEST FAIL - мутант «${name}» прошёл зелёным`);
      bad++;
    }
  }
  const structMutants = [
    [
      'runtime-импорт в модуле',
      structural("import { x } from './y';\n" + moduleSrc, files, cases.length),
    ],
    [
      'DeviceRowMenu в employee-коде',
      structural(
        moduleSrc,
        [
          ...files,
          {
            name: 'fake.svelte',
            src: "import DeviceRowMenu from '../devices/DeviceRowMenu.svelte';",
          },
        ],
        cases.length,
      ),
    ],
    ['урезанная фикстура', structural(moduleSrc, files, MIN_CASES - 1)],
  ];
  for (const [name, v] of structMutants) {
    if (v.length) console.log(`${TAG} ПОЙМАН: ${name}`);
    else {
      console.error(`${TAG} SELFTEST FAIL - мутант «${name}» прошёл зелёным`);
      bad++;
    }
  }
  if (bad) process.exit(1);
  console.log(`${TAG} SELFTEST PASS`);
}

async function main() {
  if (process.argv.includes('--selftest')) return selftest();
  const v = await runAll(fs.readFileSync(MODULE_FILE, 'utf8'), readEmployeeFiles(), loadCases());
  if (v.length) {
    console.error(`${TAG} FAIL\n${v.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}

main();
