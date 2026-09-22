#!/usr/bin/env node
// [check-act-number-no-regex-split] Постоянный гейт против regex-склейки
// номера акта (D-05, Фаза 40.3-03, находка N-1 аудита v1.4).
//
// Почему он существует: ReturnModal.svelte показывал тост после оформления
// возврата, склеивая ret.number_raw (сырое значение) с суффиксом, вычисленным
// regex-ом `ret.number.replace(/^\d+/, '')` — код предполагал, что номер акта
// ВСЕГДА начинается с цифр. С Фазы 40.2 (NUM-14) номер акта — свободный текст
// по шаблону («2026/09-1»), поэтому такой код искажает отображаемый номер
// («№2026/09-1/09-1в» вместо корректного «№2026/09-1в»). Сервер уже отдаёт
// готовое ActDto.number — искажение чисто клиентское. Forward risk F7 аудита
// предупреждает, что это класс бага, не единичный инцидент: любой будущий
// клиентский код, реконструирующий номер акта из «сырого» значения +
// regex-суффикс, повторит его (актуально для Фазы 41 WKS-05/06 актов АРМ).
//
// Гейт СТРУКТУРНЫЙ (по образцу check-movements-type-filter.mjs): сканирует
// ui/src/**/*.svelte и ui/src/**/*.ts (кроме bindings.ts — генерируемый файл,
// и node_modules) на regex-литерал внутри вызова `.replace(...)`, источник
// которого начинается с `^\d` (например `/^\d+/`, `/^\d/`) — это И ЕСТЬ
// семантический маркер «код предполагает, что значение начинается с цифр»
// (D-05: «убрать любой UI-код, считающий номер акта начинающимся с цифр»).
// Паттерн узкий и самодостаточный: не требует понимания, что именно
// форматируется, не рендерит компоненты.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-act-number-no-regex-split.mjs              # проверить репозиторий
//   node scripts/check-act-number-no-regex-split.mjs --src=<dir>  # проверить копию (самотест)
//     <dir> — копия каталога ui/ (ожидается <dir>/src/**).

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-number-no-regex-split]';

// Относительные (от корня ui/) POSIX-пути файлов, исключённых из сканирования.
const EXCLUDED_FILES = new Set(['src/bindings.ts']);
const EXCLUDED_DIRS = new Set(['node_modules']);
const EXTENSIONS = new Set(['.svelte', '.ts']);

// `.replace(` (с возможным пробелом) → regex-литерал, источник которого
// начинается с `^\d` (например `/^\d+/`, `/^\d/`, `/^\d{2}/`).
const REPLACE_DIGIT_START_RE = /\.replace\(\s*\/\^\\d/g;

/** Рекурсивно собрать файлы нужных расширений под `root/dir` (POSIX-пути от root). */
function collectFiles(root, dir, out) {
  const abs = path.join(root, dir);
  let entries;
  try {
    entries = fs.readdirSync(abs, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const rel = dir ? `${dir}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      if (EXCLUDED_DIRS.has(entry.name)) continue;
      collectFiles(root, rel, out);
      continue;
    }
    if (!EXTENSIONS.has(path.extname(entry.name))) continue;
    if (EXCLUDED_FILES.has(rel)) continue;
    out.push(rel);
  }
  return out;
}

/** Номер строки (1-based) по индексу символа в тексте. */
function lineAt(src, index) {
  let line = 1;
  for (let i = 0; i < index; i++) {
    if (src.charCodeAt(i) === 10) line++;
  }
  return line;
}

function checkFile(root, relPath, violations) {
  const src = fs.readFileSync(path.join(root, relPath), 'utf8');

  REPLACE_DIGIT_START_RE.lastIndex = 0;
  let m;
  while ((m = REPLACE_DIGIT_START_RE.exec(src)) !== null) {
    const line = lineAt(src, m.index);
    const snippet = src.slice(m.index, Math.min(src.length, m.index + 80)).split('\n')[0];
    violations.push(
      `${relPath}:${line}: regex-литерал, начинающийся с ^\\d, внутри .replace(...) — ` +
        `код предполагает, что значение (например номер акта) начинается с цифр (D-05). ` +
        `Найдено: ${snippet}`,
    );
  }
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-act-number-no-regex-split.mjs [--src=<ui-dir>]`,
    );
    process.exit(0);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const srcDir = path.join(SRC_ROOT, 'src');
  if (!fs.existsSync(srcDir)) {
    console.error(`${TAG} FAIL — не найден каталог src (${srcDir}).`);
    process.exit(1);
  }

  const files = collectFiles(SRC_ROOT, 'src', []);
  const violations = [];
  for (const rel of files) {
    checkFile(SRC_ROOT, rel, violations);
  }

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} regex-склеек номера акта.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
