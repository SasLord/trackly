#!/usr/bin/env node
// [check-sidebar-active] Гейт подсветки пункта сайдбара (фаза 41.9, план 10, G-2).
//
// Причина G-2 (установлена воспроизведением): `use:active` svelte-spa-router
// сверяет шаблон пути с `location + '?' + querystring`, а шаблон строкового
// пути (regexparam) querystring не допускает. Любой переход `#/<раздел>?id=N`
// снимал подсветку пункта.
//
// Слой 1: golden по fixtures/sidebar-active/cases.json на транспилированном
//   sidebar-config.ts (функция sidebarActivePattern).
// Слой 2 (структура, комментарии вырезаны):
//   [S]  в Sidebar.svelte значение path в use:active - вызов sidebarActivePattern(,
//        а не entry.route и не литерал;
//   [S2] каждый маршрут из реального SIDEBAR_ITEMS совпадает с `route + '?id=1'`
//        и с `route`.
// --selftest: мутанты реального sidebar-config.ts и Sidebar.svelte, а также
// до-фиксовый шаблон (без хвоста querystring) - все обязаны краснеть.
//
// Usage: node scripts/check-sidebar-active.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-sidebar-active]';
const MODULE_FILE = path.join(UI_ROOT, 'src/features/layout/sidebar-config.ts');
const SIDEBAR_FILE = path.join(UI_ROOT, 'src/features/layout/Sidebar.svelte');
const FIXTURE = path.join(__dirname, 'fixtures/sidebar-active/cases.json');
const MIN_CASES = 12;

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

function runGolden(mod, cases) {
  const violations = [];
  for (const c of cases) {
    let got;
    try {
      got = mod[c.fn](c.input.route).test(c.input.location);
    } catch (e) {
      violations.push(`[G] ${c.name}: исключение ${e.message}`);
      continue;
    }
    if (got !== c.expected) {
      violations.push(`[G] ${c.name}: ожидалось ${c.expected}, получено ${got}`);
    }
  }
  return violations;
}

function structural(moduleSrc, mod, sidebarSrc) {
  const violations = [];
  const sb = stripComments(sidebarSrc);
  const m = sb.match(/use:active=\{\{\s*path:\s*([^,}]+)/);
  if (!m) violations.push('[S] в Sidebar.svelte не найден use:active с path');
  else if (!/^sidebarActivePattern\(\s*entry\.route\s*\)\s*$/.test(m[1].trim())) {
    violations.push(
      `[S] path в use:active = «${m[1].trim()}», нужен sidebarActivePattern(entry.route)`,
    );
  }
  const routes = [
    ...stripComments(moduleSrc).matchAll(/kind:\s*'item',\s*route:\s*'([^']+)'/g),
  ].map((x) => x[1]);
  if (routes.length < 10)
    violations.push(`[S2] найдено маршрутов ${routes.length}, ожидалось не меньше 10`);
  for (const r of routes) {
    const p = mod.sidebarActivePattern(r);
    if (!p.test(r)) violations.push(`[S2] ${r}: шаблон не совпадает с самим маршрутом`);
    if (!p.test(`${r === '/' ? '' : r}?id=1`) && r !== '/')
      violations.push(`[S2] ${r}: шаблон не допускает querystring`);
    if (r === '/' && !p.test('/?id=1')) violations.push('[S2] /: шаблон не допускает querystring');
  }
  return violations;
}

function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL - якорь мутанта «${label}» встречается ${n} раз, нужен ровно 1`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

function loadCases() {
  const cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  if (cases.length < MIN_CASES) {
    console.error(`${TAG} FAIL - в фикстуре ${cases.length} кейсов, нужно не меньше ${MIN_CASES}`);
    process.exit(1);
  }
  return cases;
}

async function runAll(moduleSrc, sidebarSrc, cases) {
  const mod = await loadModuleFromTs(moduleSrc, 'sidebar-config.ts');
  return [...runGolden(mod, cases), ...structural(moduleSrc, mod, sidebarSrc)];
}

async function selftest() {
  const cases = loadCases();
  const moduleSrc = fs.readFileSync(MODULE_FILE, 'utf8');
  const sidebarSrc = fs.readFileSync(SIDEBAR_FILE, 'utf8');
  const clean = await runAll(moduleSrc, sidebarSrc, cases);
  if (clean.length) {
    console.error(`${TAG} SELFTEST FAIL - чистый исходник красный:\n${clean.join('\n')}`);
    process.exit(1);
  }
  const mutants = [
    ['module', 'нет хвоста querystring', mutate(moduleSrc, "'/?(?:\\\\?.*)?$'", "'/?$'", 'tail')],
    [
      'module',
      'нет экранирования',
      mutate(moduleSrc, ".replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&')", '', 'escape'),
    ],
    [
      'module',
      'нет необязательного слеша',
      mutate(moduleSrc, "'/?(?:\\\\?.*)?$'", "'(?:\\\\?.*)?$'", 'slash'),
    ],
    ['module', 'нет флага i', mutate(moduleSrc, ", 'i')", ", '')", 'flag')],
    [
      'sidebar',
      'path: entry.route',
      mutate(sidebarSrc, 'path: sidebarActivePattern(entry.route)', 'path: entry.route', 'sidebar'),
    ],
  ];
  let bad = 0;
  for (const [kind, name, src] of mutants) {
    const v =
      kind === 'module'
        ? await runAll(src, sidebarSrc, cases)
        : await runAll(moduleSrc, src, cases);
    if (v.length) console.log(`${TAG} ПОЙМАН: ${name}`);
    else {
      console.error(`${TAG} SELFTEST FAIL - мутант «${name}» прошёл зелёным`);
      bad++;
    }
  }
  // До-фиксовый шаблон (точный путь без хвоста) - вакуумная проверка golden.
  const oldMod = {
    sidebarActivePattern: (r) =>
      new RegExp('^' + r.replace(/\/+$/, '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '/?$', 'i'),
  };
  if (runGolden(oldMod, cases).length) console.log(`${TAG} ПОЙМАН: до-фиксовый шаблон`);
  else {
    console.error(`${TAG} SELFTEST FAIL - до-фиксовый шаблон прошёл golden зелёным`);
    bad++;
  }
  if (bad) process.exit(1);
  console.log(`${TAG} SELFTEST PASS`);
}

async function main() {
  if (process.argv.includes('--selftest')) return selftest();
  const v = await runAll(
    fs.readFileSync(MODULE_FILE, 'utf8'),
    fs.readFileSync(SIDEBAR_FILE, 'utf8'),
    loadCases(),
  );
  if (v.length) {
    console.error(`${TAG} FAIL\n${v.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}

main();
