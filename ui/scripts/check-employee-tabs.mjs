#!/usr/bin/env node
// [check-employee-tabs] Гейт вкладок раздела в шапке сотрудника (фаза 41.6, план 05, D-01/D-03).
//
// Слой 1 (golden, исполняемый): sidebarActivePattern из sidebar-config.ts на кейсах
//   fixtures/employee-tabs/cases.json (шаблоны /requests и /devices, хвост querystring).
// Слой 2 (структура EmployeeLayout.svelte, комментарии вырезаны):
//   [T1] один <nav aria-label="Разделы">, внутри href="#/requests" раньше href="#/devices";
//   [T2] path у каждого use:active - вызов sidebarActivePattern(, не литерал;
//   [T3] UnreadBadge с requestsWithUnread внутри ссылки «Заявки», не «Устройства»;
//   [T4] нет импорта Sidebar.svelte и SIDEBAR_ITEMS;
//   [T5] правило активной вкладки в style записано через :global(.
// --selftest: мутанты реального EmployeeLayout.svelte и до-фиксовый шаблон обязаны краснеть.
//
// Usage: node scripts/check-employee-tabs.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-employee-tabs]';
const MODULE_FILE = path.join(UI_ROOT, 'src/features/layout/sidebar-config.ts');
const LAYOUT_FILE = path.join(UI_ROOT, 'src/features/layout/EmployeeLayout.svelte');
const FIXTURE = path.join(__dirname, 'fixtures/employee-tabs/cases.json');
const MIN_CASES = 8;
const MIN_TABS = 2;

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

function structural(layoutSrc) {
  const v = [];
  const src = stripComments(layoutSrc);
  const navs = [...src.matchAll(/<nav\b[^>]*aria-label="Разделы"[^>]*>/g)];
  if (navs.length !== 1) {
    v.push(`[T1] <nav aria-label="Разделы"> найдено ${navs.length}, нужен ровно 1`);
    return v;
  }
  const navStart = navs[0].index;
  const navEnd = src.indexOf('</nav>', navStart);
  if (navEnd < 0) {
    v.push('[T1] у nav нет закрывающего тега');
    return v;
  }
  const nav = src.slice(navStart, navEnd);
  const iReq = nav.indexOf('href="#/requests"');
  const iDev = nav.indexOf('href="#/devices"');
  if (iReq < 0 || iDev < 0) v.push('[T1] в nav нет ссылки #/requests и/или #/devices');
  else if (iReq > iDev)
    v.push('[T1] порядок вкладок нарушен: «Заявки» должна идти перед «Устройства»');
  const anchors = [...nav.matchAll(/<a\b/g)].length;
  if (anchors < MIN_TABS) v.push(`[T1] вкладок ${anchors}, нужно не меньше ${MIN_TABS}`);

  const paths = [...src.matchAll(/use:active=\{\{\s*path:\s*([^,}]+)/g)].map((m) => m[1].trim());
  if (paths.length < MIN_TABS) v.push(`[T2] use:active найдено ${paths.length}`);
  for (const p of paths) {
    if (!/^sidebarActivePattern\(\s*'\/[a-z]+'\s*\)$/.test(p))
      v.push(`[T2] path в use:active = «${p}», нужен sidebarActivePattern('/маршрут')`);
  }

  if (iReq >= 0 && iDev >= 0) {
    const badge = nav.search(/<UnreadBadge\b[^>]*requestsWithUnread/);
    if (badge < 0) v.push('[T3] в nav нет UnreadBadge с requestsWithUnread');
    else if (!(badge > iReq && badge < iDev))
      v.push('[T3] счётчик не внутри ссылки «Заявки» (между href #/requests и #/devices)');
    if (iDev >= 0 && (nav.slice(iDev).match(/requestsWithUnread/g) || []).length)
      v.push('[T3] requestsWithUnread попал во вторую вкладку');
  }

  if (/import\s+[^;]*Sidebar\.svelte|SIDEBAR_ITEMS/.test(src))
    v.push('[T4] EmployeeLayout тянет Sidebar.svelte/SIDEBAR_ITEMS (D-01)');

  if (!/:global\(\s*\.tab\.is-active\s*\)\s*\{/.test(src))
    v.push('[T5] правило активной вкладки не через :global(.tab.is-active) - будет вырезано');
  return v;
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

async function runAll(moduleSrc, layoutSrc, cases) {
  const mod = await loadModuleFromTs(moduleSrc, 'sidebar-config.ts');
  return [...runGolden(mod, cases), ...structural(layoutSrc)];
}

async function selftest() {
  const cases = loadCases();
  const moduleSrc = fs.readFileSync(MODULE_FILE, 'utf8');
  const layoutSrc = fs.readFileSync(LAYOUT_FILE, 'utf8');
  const clean = await runAll(moduleSrc, layoutSrc, cases);
  if (clean.length) {
    console.error(`${TAG} SELFTEST FAIL - чистый исходник красный:\n${clean.join('\n')}`);
    process.exit(1);
  }
  const R = 'href="#/requests"';
  const D = 'href="#/devices"';
  const badgeLine =
    '<UnreadBadge count={unreadStore.requestsWithUnread} label="Заявок с непрочитанным" />';
  const swapped = mutate(
    mutate(layoutSrc, R, '@@TMP@@', 'swap1').replace('@@TMP@@', () => '@@X@@'),
    D,
    R,
    'swap2',
  ).replace('@@X@@', () => D);
  const mutants = [
    ['перестановка вкладок (порядок)', swapped],
    [
      'счётчик во второй вкладке',
      mutate(
        mutate(layoutSrc, badgeLine, '', 'badge-out'),
        '<span>Устройства</span>',
        `<span>Устройства</span>\n          ${badgeLine}`,
        'badge-in',
      ),
    ],
    [
      'path - литерал',
      mutate(layoutSrc, "path: sidebarActivePattern('/requests')", "path: '/requests'", 'path'),
    ],
    [
      'импорт Sidebar',
      mutate(
        layoutSrc,
        "import active from 'svelte-spa-router/active';",
        "import active from 'svelte-spa-router/active';\n  import Sidebar from './Sidebar.svelte';",
        'sidebar',
      ),
    ],
    ['нет :global', mutate(layoutSrc, ':global(.tab.is-active)', '.tab.is-active', 'global')],
    [
      'нет aria-label у nav',
      mutate(layoutSrc, 'aria-label="Разделы"', 'aria-label="Меню"', 'aria-label'),
    ],
  ];
  let bad = 0;
  for (const [name, src] of mutants) {
    const v = await runAll(moduleSrc, src, cases);
    if (v.length) console.log(`${TAG} ПОЙМАН: ${name}`);
    else {
      console.error(`${TAG} SELFTEST FAIL - мутант «${name}» прошёл зелёным`);
      bad++;
    }
  }
  const oldMod = {
    sidebarActivePattern: (r) =>
      new RegExp('^' + r.replace(/\/+$/, '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '/?$', 'i'),
  };
  if (runGolden(oldMod, cases).length)
    console.log(`${TAG} ПОЙМАН: до-фиксовый шаблон без querystring`);
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
    fs.readFileSync(LAYOUT_FILE, 'utf8'),
    loadCases(),
  );
  if (v.length) {
    console.error(`${TAG} FAIL\n${v.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}

main();
