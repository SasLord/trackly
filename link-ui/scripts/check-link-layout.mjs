#!/usr/bin/env node
// [check-link-layout] Структурный гейт каркаса Link (фаза 42, план 16; LNK-10, D-14, D-15, D-20).
//
// Зачем: раскладка окна (логотип, вкладка «Чат», сайдбар, область содержимого, заглушка,
// служебная строка), конфигурация окна и права webview не должны тихо деградировать.
//
// Правила (комментарии вырезаются):
//   [L0] ни одного .svelte в link-ui/src — гейт вакуумный, нарушение;
//   [L1] App.svelte без LinkLogo / Tabs с «Чат» / <aside / <main / заглушки / StatusLine;
//   [L2] tauri.conf.json: размеры 1000x700, минимум 720x500, visible:false, имя и бинарник;
//   [L3] capabilities/main.json: ровно core:default, window-state:default и
//        core:window:allow-set-background-color (фон окна под тему, quick 261126-rbg);
//   [L5] App.svelte не использует screenFor / <AccountMenu / <OfflineBanner (план 44-13);
//   [L6] в link-ui/src есть «Доверять» (кнопки принятия сертификата нет нигде, D-07);
//   [L4] в link-ui/src есть {@html, fetch(, XMLHttpRequest, WebSocket (сети в webview нет, ADR 0002).
// Отсутствие файла — нарушение, не SKIP.
//
// Чего гейт не видит: визуальный результат и поведение WebView2 (проверяется вручную).
//
// Usage:
//   node scripts/check-link-layout.mjs [--root=<копия репозитория>]
//   node scripts/check-link-layout.mjs --selftest

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const REPO = path.resolve(path.dirname(__filename), '..', '..');

/** Вырезает HTML-, блочные и строчные комментарии (строки с `://` не трогаем). */
export function stripComments(text) {
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/[^\n]*/g, '$1');
}

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}

function read(root, rel, violations, code) {
  const p = path.join(root, rel);
  if (!fs.existsSync(p)) {
    violations.push(`[${code}] нет файла ${rel}`);
    return null;
  }
  return fs.readFileSync(p, 'utf8');
}

/** Возвращает { violations, checks }. */
export function check(root) {
  const violations = [];
  let checks = 0;
  const need = (ok, msg) => {
    checks++;
    if (!ok) violations.push(msg);
  };

  const srcDir = path.join(root, 'link-ui', 'src');
  const files = walk(srcDir);
  const svelte = files.filter((f) => f.endsWith('.svelte'));
  need(svelte.length > 0, '[L0] в link-ui/src нет ни одного .svelte');

  const app = read(root, 'link-ui/src/App.svelte', violations, 'L1');
  if (app !== null) {
    const a = stripComments(app);
    need(/<LinkLogo[\s/>]/.test(a), '[L1] App.svelte: нет LinkLogo');
    need(
      /<Tabs[\s\S]*?\/>/.test(a) && /label:\s*'Чат'/.test(a),
      '[L1] App.svelte: нет вкладки «Чат»',
    );
    need(/<aside[\s>]/.test(a), '[L1] App.svelte: нет <aside');
    need(/<main[\s>]/.test(a), '[L1] App.svelte: нет <main');
    need(a.includes('Здесь скоро будет чат'), '[L1] App.svelte: нет заглушки чата');
    need(/<StatusLine[\s/>]/.test(a), '[L1] App.svelte: нет StatusLine');
    need(/\bscreenFor\s*\(/.test(a), '[L5] App.svelte: экран выбирается не через screenFor');
    need(/<AccountMenu[\s/>]/.test(a), '[L5] App.svelte: нет AccountMenu');
    need(/<OfflineBanner[\s/>]/.test(a), '[L5] App.svelte: нет OfflineBanner');
  }

  const confText = read(root, 'crates/link-app/tauri.conf.json', violations, 'L2');
  if (confText !== null) {
    try {
      const conf = JSON.parse(confText);
      const w = conf.app?.windows?.[0] ?? {};
      need(w.width === 1000 && w.height === 700, '[L2] размер окна не 1000x700');
      need(w.minWidth === 720 && w.minHeight === 500, '[L2] минимум окна не 720x500');
      need(w.visible === false, '[L2] окно должно создаваться visible:false (window-state)');
      // фон окна ставит только стор темы (не на macOS): backgroundColor в конфиге
      // перекрашивает шапку окна macOS при системном цвете заголовка
      need(
        w.backgroundColor === undefined,
        '[L2] backgroundColor в конфиге окна ломает шапку на macOS (фон ставит стор темы)',
      );
      need(conf.productName === 'Trackly Link', '[L2] productName не «Trackly Link»');
      need(conf.mainBinaryName === 'trackly-link', '[L2] mainBinaryName не trackly-link');
    } catch {
      violations.push('[L2] tauri.conf.json не разобрался как JSON');
    }
  }

  const capText = read(root, 'crates/link-app/capabilities/main.json', violations, 'L3');
  if (capText !== null) {
    try {
      const perms = JSON.parse(capText).permissions ?? [];
      const ok =
        perms.length === 3 &&
        perms.includes('core:default') &&
        perms.includes('window-state:default') &&
        perms.includes('core:window:allow-set-background-color');
      need(
        ok,
        `[L3] capabilities: ожидалось ровно core:default + window-state:default + core:window:allow-set-background-color, есть ${JSON.stringify(perms)}`,
      );
    } catch {
      violations.push('[L3] capabilities/main.json не разобрался как JSON');
    }
  }

  for (const f of svelte) {
    checks++;
    if (stripComments(fs.readFileSync(f, 'utf8')).includes('Доверять'))
      violations.push(`[L6] ${path.relative(root, f)}: «Доверять» (принятие сертификата запрещено)`);
  }

  for (const f of files.filter((x) => /\.(svelte|ts)$/.test(x))) {
    const t = stripComments(fs.readFileSync(f, 'utf8'));
    const rel = path.relative(root, f);
    for (const re of [/\{@html/, /\bfetch\s*\(/, /XMLHttpRequest/, /\bWebSocket\b/]) {
      checks++;
      if (re.test(t)) violations.push(`[L4] ${rel}: запрещённая конструкция ${re.source}`);
    }
  }
  return { violations, checks };
}

function copyTree(root) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'link-layout-'));
  for (const rel of [
    'link-ui/src',
    'crates/link-app/tauri.conf.json',
    'crates/link-app/capabilities/main.json',
  ]) {
    const from = path.join(root, rel);
    const to = path.join(tmp, rel);
    fs.mkdirSync(path.dirname(to), { recursive: true });
    fs.cpSync(from, to, { recursive: true });
  }
  return tmp;
}

function selftest() {
  const mutants = [
    ['L1', 'link-ui/src/App.svelte', "label: 'Чат'", "label: 'Беседы'"],
    ['L1', 'link-ui/src/App.svelte', '<aside', '<div'],
    ['L1', 'link-ui/src/App.svelte', 'Здесь скоро будет чат', 'Пусто'],
    ['L1', 'link-ui/src/App.svelte', '<StatusLine />', ''],
    ['L5', 'link-ui/src/App.svelte', 'screenFor(st)', 'String(st)'],
    ['L5', 'link-ui/src/App.svelte', '<AccountMenu', '<div'],
    ['L5', 'link-ui/src/App.svelte', '<OfflineBanner />', ''],
    ['L6', 'link-ui/src/App.svelte', 'Беседы появятся здесь', 'Доверять'],
    ['L2', 'crates/link-app/tauri.conf.json', '"minWidth": 720', '"minWidth": 400'],
    ['L2', 'crates/link-app/tauri.conf.json', '"visible": false', '"visible": true'],
    [
      'L2',
      'crates/link-app/tauri.conf.json',
      '"visible": false',
      '"backgroundColor": "#eef1f6", "visible": false',
    ],
    ['L3', 'crates/link-app/capabilities/main.json', '"window-state:default"', '"shell:default"'],
    [
      'L3',
      'crates/link-app/capabilities/main.json',
      ', "core:window:allow-set-background-color"',
      '',
    ],
    [
      'L3',
      'crates/link-app/capabilities/main.json',
      '"core:default",',
      '"core:default","fs:default",',
    ],
    [
      'L4',
      'link-ui/src/App.svelte',
      '<p class="sidebar-stub">',
      '<p class="sidebar-stub">{@html x}',
    ],
  ];
  const base = check(REPO);
  if (base.violations.length) {
    console.error(
      '[check-link-layout] selftest: чистое дерево уже красное:\n' + base.violations.join('\n'),
    );
    process.exit(1);
  }
  let bad = 0;
  for (const [code, rel, from, to] of mutants) {
    const tmp = copyTree(REPO);
    const p = path.join(tmp, rel);
    const src = fs.readFileSync(p, 'utf8');
    const n = src.split(from).length - 1;
    if (n !== 1) {
      console.error(`  ANCHOR ${rel}: «${from}» встречается ${n} раз (нужно ровно 1)`);
      bad++;
      continue;
    }
    fs.writeFileSync(
      p,
      src.replace(from, () => to),
    );
    const { violations } = check(tmp);
    const hit = violations.some((v) => v.startsWith(`[${code}]`));
    console.log(`  ${hit ? 'ok  ' : 'FAIL'} мутант [${code}] ${rel}: «${from}»`);
    if (!hit) bad++;
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  // [L0]: пустой src
  const tmp0 = copyTree(REPO);
  for (const f of walk(path.join(tmp0, 'link-ui', 'src'))) if (f.endsWith('.svelte')) fs.rmSync(f);
  const l0 = check(tmp0).violations.some((v) => v.startsWith('[L0]'));
  console.log(`  ${l0 ? 'ok  ' : 'FAIL'} мутант [L0] без .svelte`);
  if (!l0) bad++;
  fs.rmSync(tmp0, { recursive: true, force: true });
  if (bad) {
    console.error(`[check-link-layout] selftest: провалов ${bad}`);
    process.exit(1);
  }
  console.log('[check-link-layout] selftest: все мутанты пойманы');
}

if (process.argv[1] === __filename) {
  if (process.argv.includes('--selftest')) selftest();
  else {
    const arg = process.argv.find((a) => a.startsWith('--root='));
    const { violations, checks } = check(arg ? path.resolve(arg.slice(7)) : REPO);
    if (violations.length) {
      console.error('[check-link-layout] FAIL\n' + violations.join('\n'));
      process.exit(1);
    }
    console.log(`[check-link-layout] PASS (проверено признаков: ${checks})`);
  }
}
