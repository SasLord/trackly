#!/usr/bin/env node
// [check-place-path-short] Постоянный гейт против регрессий колонки «Место»
// (Phase 39.1, PLC-08, D-21).
//
// Почему он существует: контракт колонки состоит из ДВУХ половин, и сломать
// можно каждую по отдельности, ничего не уронив при сборке —
//   в ТЕКСТЕ ячейки  — сокращённый путь (`place_path_short`, приходит с бэкенда);
//   в АТРИБУТЕ title — полный путь (`full_path`), чтобы полное расположение
//                      было в одном наведении курсора.
// Одна половина уже отваливалась: сгруппированный список устройств
// (`DeviceGroupRow`) молча продолжал показывать полный путь и расходился с
// плоским видом — WR-04 из 39.1-REVIEW.md, уехало в сборку, найдено ревьюером,
// починено руками в коммите 8fa995e5. Ни один автотест этого не держал:
// `pnpm build` доказывает, что компонент компилируется, но не то, КАКОЕ поле он
// читает, а Rust-тесты кончаются на границе DTO.
//
// Гейт СТРУКТУРНЫЙ (как check-print-isolation.mjs): он читает исходники и
// проверяет наличие инвариантов, а НЕ правильность рендера — вёрстка и hover
// проверяются руками (Manual-Only в 39.1-VALIDATION.md). Его единственная
// задача — громко падать, когда выстраданный фикс молча убрали при рефакторинге.
//
// Проверяемые инварианты:
//   INV-1 (WR-04) — в компоненте есть ячейка `<td>`, чей ТЕКСТ читает
//                   `place_path_short`;
//   INV-2 (D-21)  — у той же ячейки в `title=` стоит выражение с полным путём
//                   (`full_path`);
//   INV-3 (D-21)  — текст той ячейки НЕ рендерит полный путь напрямую
//                   (иначе сокращение декоративно).
//   INV-4 (WR-04) — ReportTable: `formatCellDisplay` читает `place_path_short`,
//                   а `formatCellTitle` — нет (title обязан остаться полным).
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-place-path-short.mjs              # проверить репозиторий
//   node scripts/check-place-path-short.mjs --src=<dir>  # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-place-path-short]';

const SHORT = 'place_path_short';
const FULL = 'full_path';

/** Компоненты со «строчным» контрактом ячейки (INV-1..INV-3). */
const ROW_COMPONENTS = [
  'src/features/devices/DeviceListRow.svelte',
  'src/features/devices/DeviceGroupRow.svelte',
  'src/features/cartridges/CartridgeListRow.svelte',
  'src/features/places/PlaceContents.svelte',
];

/** Компонент с функциональным контрактом (INV-4) — отчёты строят ячейку хелперами. */
const REPORT_TABLE = 'src/features/reports/ReportTable.svelte';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/**
 * Затирает пробелами (сохраняя длину и переводы строк) всё, что не является
 * разметкой: HTML-комментарии, блоки <script> и <style>. Без этого объяснение
 * контракта в комментарии или чтение `full_path` во вспомогательной функции
 * удовлетворяли бы проверку при мёртвой разметке — ровно та ошибка, которую
 * гейт обязан ловить.
 */
function markupOnly(src) {
  const out = src.split('');
  const blankRange = (from, to) => {
    for (let i = from; i < to && i < src.length; i++) if (src[i] !== '\n') out[i] = ' ';
  };

  for (const re of [
    /<!--[\s\S]*?-->/g,
    /<script[\s\S]*?<\/script>/gi,
    /<style[\s\S]*?<\/style>/gi,
  ]) {
    for (const m of src.matchAll(re)) blankRange(m.index, m.index + m[0].length);
  }
  return out.join('');
}

/**
 * Индекс `>`, закрывающего открывающий тег, начинающийся на `startIdx`.
 * Учитывает вложенность `{}`: `onclick={() => openView(row)}` содержит `>`,
 * который тегом не является.
 */
function endOfOpeningTag(text, startIdx) {
  let depth = 0;
  for (let i = startIdx; i < text.length; i++) {
    const c = text[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return i;
  }
  return -1;
}

/** Все `<td>`-ячейки как {attrs, body, line}. */
function collectCells(markup, src) {
  const cells = [];
  const lineOf = (idx) => src.slice(0, idx).split('\n').length;

  for (const m of markup.matchAll(/<td[\s>]/g)) {
    const start = m.index;
    const openEnd = endOfOpeningTag(markup, start);
    if (openEnd < 0) continue;
    const closeIdx = markup.indexOf('</td', openEnd);
    if (closeIdx < 0) continue;
    cells.push({
      attrs: markup.slice(start, openEnd + 1),
      body: markup.slice(openEnd + 1, closeIdx),
      line: lineOf(start),
    });
  }
  return cells;
}

/** Значение атрибута `title={...}` из открывающего тега (с учётом вложенных скобок). */
function titleExpression(attrs) {
  const m = attrs.match(/title\s*=\s*\{/);
  if (!m) return null;
  const open = m.index + m[0].length - 1;
  let depth = 0;
  for (let i = open; i < attrs.length; i++) {
    if (attrs[i] === '{') depth++;
    else if (attrs[i] === '}') {
      depth--;
      if (depth === 0) return attrs.slice(open + 1, i);
    }
  }
  return null;
}

/** Тело функции по имени — для INV-4. */
function functionBody(src, fnName) {
  const m = src.match(new RegExp(`function\\s+${fnName}\\s*\\(`));
  if (!m) return null;
  const open = src.indexOf('{', m.index + m[0].length);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(open + 1, i);
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

function checkRowComponent(src, label) {
  const violations = [];
  const fail = (inv, fixId, message, hint) => violations.push({ inv, fixId, message, hint, label });

  const markup = markupOnly(src);
  const cells = collectCells(markup, src);
  const placeCells = cells.filter((c) => c.body.includes(SHORT));

  if (placeCells.length === 0) {
    fail(
      'INV-1',
      'WR-04',
      `ни одна ячейка <td> не рендерит ${SHORT}`,
      `Колонка «Место» снова показывает полный путь — ровно тот дефект, из-за которого ` +
        `сгруппированный и плоский виды списка устройств разошлись между собой. ` +
        `Текст ячейки обязан читать ${SHORT}, приходящий с бэкенда.`,
    );
    return violations;
  }

  for (const cell of placeCells) {
    const title = titleExpression(cell.attrs);
    if (title === null) {
      fail(
        'INV-2',
        'D-21',
        `ячейка с ${SHORT} (строка ${cell.line}) не имеет атрибута title={...}`,
        'Сокращённый путь без полного в title — единственный способ увидеть полное ' +
          'расположение исчезает.',
      );
    } else if (!title.includes(FULL)) {
      fail(
        'INV-2',
        'D-21',
        `ячейка с ${SHORT} (строка ${cell.line}) имеет title={${title.trim()}} без ${FULL}`,
        `В title обязан стоять ПОЛНЫЙ путь (${FULL}) — он и есть «полное расположение ` +
          'в одном наведении курсора».',
      );
    }

    if (cell.body.includes(FULL)) {
      fail(
        'INV-3',
        'D-21',
        `ячейка с ${SHORT} (строка ${cell.line}) рендерит в тексте ещё и ${FULL}`,
        'Полный путь принадлежит title, а не тексту ячейки — иначе сокращение декоративно.',
      );
    }
  }

  return violations;
}

function checkReportTable(src, label) {
  const violations = [];
  const fail = (inv, fixId, message, hint) => violations.push({ inv, fixId, message, hint, label });

  const display = functionBody(src, 'formatCellDisplay');
  const title = functionBody(src, 'formatCellTitle');

  if (display === null || title === null) {
    fail(
      'INV-4',
      'WR-04',
      `не найдены функции formatCellDisplay/formatCellTitle (${display === null ? 'display' : 'title'})`,
      'Их переименовали или удалили — асимметрия «текст сокращён, title полный» больше ' +
        'никем не удерживается. Обнови гейт вместе с рефакторингом, осознанно.',
    );
    return violations;
  }

  if (!display.includes(SHORT)) {
    fail(
      'INV-4',
      'WR-04',
      `formatCellDisplay не читает ${SHORT}`,
      'Колонка «Место» в отчётах снова печатает полный путь.',
    );
  }

  if (title.includes(SHORT)) {
    fail(
      'INV-4',
      'D-21',
      `formatCellTitle читает ${SHORT}`,
      'В title обязан идти ПОЛНЫЙ путь; если и он сокращён — полного пути в отчёте ' +
        'не увидеть нигде.',
    );
  }

  return violations;
}

// ---------------------------------------------------------------------------

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-place-path-short.mjs [--src=<ui-dir>]`);
    process.exit(0);
  }

  const violations = [];
  let checked = 0;

  for (const rel of [...ROW_COMPONENTS, REPORT_TABLE]) {
    const full = path.join(SRC_ROOT, rel);
    let source;
    try {
      source = fs.readFileSync(full, 'utf8');
    } catch {
      console.error(
        `${TAG} FAIL — не удалось прочитать ${rel}. Компонент переехал или удалён: ` +
          'обнови список в гейте осознанно, а не удаляй проверку.',
      );
      process.exit(1);
    }
    checked++;
    violations.push(
      ...(rel === REPORT_TABLE ? checkReportTable(source, rel) : checkRowComponent(source, rel)),
    );
  }

  for (const v of violations) {
    console.error(`${TAG} ${v.label} — ${v.inv} (регресс ${v.fixId}): ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений контракта колонки «Место» в ${checked} компонентах. ` +
        'Этот класс дефекта уже уезжал в сборку (WR-04) — не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений (${checked} компонентов)`);
  process.exit(0);
}

main();
