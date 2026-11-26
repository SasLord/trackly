// Общие корни сканирования и обходчик для гейтов (фаза 42, план 01, LNK-01).
//
// Имя файла не `check-*`, поэтому check-lint-selftest-wiring его гейтом не считает.
// После переезда фронтенда в `packages/*/src` и `link-ui/src` гейты, привязанные
// к подкаталогам `ui/src`, не должны терять переехавшие файлы: для не-ui корней
// обход всегда полный.
//
// Если задана переменная окружения GATE_COVERAGE_REPORT, walk() дописывает в файл
// одну JSON-строку {"gate": <имя скрипта>, "count": <число файлов>}.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const UI_ROOT = path.resolve(__dirname, '..');
export const REPO = path.resolve(UI_ROOT, '..');

const DEFAULT_EXTS = ['.svelte', '.ts', '.js', '.mjs', '.scss', '.css'];
const SKIP_DIRS = new Set(['node_modules', 'dist']);

/** Существующие корни сканирования; отсутствие ui/src — ошибка, не пустой результат. */
export function scanRoots() {
  const uiSrc = path.join(UI_ROOT, 'src');
  if (!fs.existsSync(uiSrc)) {
    throw new Error(`[_roots] не найден обязательный корень сканирования: ${uiSrc}`);
  }
  const roots = [uiSrc];
  const pkgs = path.join(REPO, 'packages');
  if (fs.existsSync(pkgs)) {
    for (const name of fs.readdirSync(pkgs).sort()) {
      const src = path.join(pkgs, name, 'src');
      if (fs.existsSync(src) && fs.statSync(src).isDirectory()) roots.push(src);
    }
  }
  const linkUi = path.join(REPO, 'link-ui', 'src');
  if (fs.existsSync(linkUi)) roots.push(linkUi);
  return roots;
}

function collect(dir, exts, out) {
  if (!fs.existsSync(dir)) return;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, ent.name);
    if (ent.isDirectory()) {
      if (SKIP_DIRS.has(ent.name)) continue;
      collect(full, exts, out);
    } else if (ent.isFile()) {
      if (full === path.join(UI_ROOT, 'src', 'bindings.ts')) continue;
      if (exts.includes(path.extname(ent.name))) out.push(full);
    }
  }
}

/** Отсортированный список абсолютных путей файлов. */
export function walk({ exts = DEFAULT_EXTS, subdirs = null, roots = null } = {}) {
  const list = roots ?? scanRoots();
  const uiSrc = path.join(UI_ROOT, 'src');
  const out = [];
  for (const root of list) {
    if (subdirs && path.resolve(root) === uiSrc) {
      for (const sub of subdirs) collect(path.join(root, sub), exts, out);
    } else {
      collect(root, exts, out);
    }
  }
  out.sort();
  const report = process.env.GATE_COVERAGE_REPORT;
  if (report) {
    const gate = path.basename(process.argv[1] ?? 'unknown');
    fs.appendFileSync(report, JSON.stringify({ gate, count: out.length }) + '\n');
  }
  return out;
}
