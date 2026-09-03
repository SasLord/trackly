#!/usr/bin/env node
// [check-report-type-parity] Постоянный гейт против регрессии «reportType экрана
// разошёлся с reportType экспорта» (Phase 40, gap closure UAT-40 test 13, D-25).
//
// Почему он существует: `ReportsPage.svelte` держит ДВЕ строки, которые обязаны
// совпадать на каждый шаг:
//   INV-1 — проп `reportType`, переданный живой таблице (`<ReportTable ...
//           reportType={...} />`), обязан читать нормализованную функцию
//           `reportTypeKey()`, а не сырой ключ вкладки `activeReport`.
//           Для домена «Перемещения» `activeReport` ВСЕГДА равен `'all'`
//           (общий ключ вкладки со «Заявками»), а `reportTypeKey()` для этого
//           же домена возвращает `'movements'`.
//   INV-2 — строковый литерал, с которым `ReportTable.showDeletedBadge`
//           сравнивает `reportType` (сейчас `'movements'`), обязан быть одним
//           из return-значений `reportTypeKey()` в `ReportsPage.svelte`.
//           Иначе бейдж «Удалено» рендерится в CSV/PDF-экспорте (который уже
//           шёл через `reportTypeKey()`), но никогда — в живой таблице
//           приложения: ровно тот дефект, который закрывает этот план.
//
// Это уже ТРЕТЬЯ площадка одной и той же известной коллизии ключа `'all'`
// между доменами «Заявки» и «Перемещения» (после `currentColumns()` и
// `currentCmd()`, см. комментарии GAP-R1 в ReportsPage.svelte) — гейт ловит
// класс дефекта, а не единичный случай.
//
// Гейт СТРУКТУРНЫЙ (по образцу check-place-path-short.mjs): читает исходники
// и парсит регулярками/скобочным балансом, НЕ рендерит компонент и не выполняет
// код. Не гарантирует правильность рендера бейджа — только то, что оба места
// используют один и тот же нормализованный источник.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-report-type-parity.mjs              # проверить репозиторий
//   node scripts/check-report-type-parity.mjs --src=<dir>  # проверить копию (самотест)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-report-type-parity]';

const REPORTS_PAGE = 'src/features/reports/ReportsPage.svelte';
const REPORT_TABLE = 'src/features/reports/ReportTable.svelte';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/** Тело функции по имени (учитывает вложенные `{}`, как в check-place-path-short.mjs). */
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

/**
 * Значение атрибута `reportType={...}` внутри открывающего тега `<ReportTable ...>`
 * (с учётом вложенных `{}`, как `titleExpression` в check-place-path-short.mjs).
 */
function reportTypePropExpression(src) {
  const tagMatch = src.match(/<ReportTable[\s>]/);
  if (!tagMatch) return null;
  const tagStart = tagMatch.index;

  // Конец открывающего тега <ReportTable ...> с учётом вложенности { }.
  let depth = 0;
  let tagEnd = -1;
  for (let i = tagStart; i < src.length; i++) {
    const c = src[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) {
      tagEnd = i;
      break;
    }
  }
  if (tagEnd < 0) return null;

  const tagSrc = src.slice(tagStart, tagEnd + 1);
  const propMatch = tagSrc.match(/reportType\s*=\s*\{/);
  if (!propMatch) return null;

  const open = propMatch.index + propMatch[0].length - 1;
  let pdepth = 0;
  for (let i = open; i < tagSrc.length; i++) {
    if (tagSrc[i] === '{') pdepth++;
    else if (tagSrc[i] === '}') {
      pdepth--;
      if (pdepth === 0) return tagSrc.slice(open + 1, i);
    }
  }
  return null;
}

/**
 * Строковый литерал, с которым `reportType` сравнивается внутри тела функции
 * `showDeletedBadge` (например, `reportType === 'movements'`).
 */
function reportTypeComparisonLiteral(body) {
  const m = body.match(/reportType\s*===\s*'([^']+)'/);
  return m ? m[1] : null;
}

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

function checkInv1(reportsPageSrc, violations) {
  const expr = reportTypePropExpression(reportsPageSrc);

  if (expr === null) {
    violations.push(
      `INV-1 (регресс D-25): не найден проп reportType={...} на <ReportTable ...> в ${REPORTS_PAGE}. ` +
        'Компонент переименован/переструктурирован — обнови гейт осознанно, а не удаляй проверку.',
    );
    return;
  }

  const trimmed = expr.trim();
  if (trimmed === 'activeReport') {
    violations.push(
      `INV-1 (регресс D-25): <ReportTable reportType={activeReport} /> в ${REPORTS_PAGE} — ` +
        "это ключ ВКЛАДКИ (для домена «Перемещения» всегда 'all'), а не нормализованный ключ " +
        'домена. Живая таблица снова разойдётся с CSV/PDF-экспортом (который уже использует ' +
        'reportTypeKey()) — бейдж «Удалено» перестанет рендериться в приложении.',
    );
    return;
  }

  if (!trimmed.includes('reportTypeKey()')) {
    violations.push(
      `INV-1 (регресс D-25): <ReportTable reportType={${trimmed}} /> в ${REPORTS_PAGE} не читает ` +
        'reportTypeKey() — единственную нормализующую функцию, уже используемую в путях ' +
        'exportCsv/exportPdf. Живая таблица и экспорт должны получать одно и то же значение.',
    );
  }
}

function checkInv2(reportsPageSrc, reportTableSrc, violations) {
  const badgeBody = functionBody(reportTableSrc, 'showDeletedBadge');
  if (badgeBody === null) {
    violations.push(
      `INV-2 (регресс D-25): функция showDeletedBadge не найдена в ${REPORT_TABLE}. ` +
        'Переименована/удалена — обнови гейт осознанно, а не удаляй проверку.',
    );
    return;
  }

  const literal = reportTypeComparisonLiteral(badgeBody);
  if (literal === null) {
    violations.push(
      `INV-2 (регресс D-25): showDeletedBadge в ${REPORT_TABLE} не сравнивает reportType со ` +
        "строковым литералом вида reportType === '<значение>'. Логика бейджа переписана — " +
        'обнови гейт осознанно.',
    );
    return;
  }

  const keyBody = functionBody(reportsPageSrc, 'reportTypeKey');
  if (keyBody === null) {
    violations.push(
      `INV-2 (регресс D-25): функция reportTypeKey не найдена в ${REPORTS_PAGE}. ` +
        'Переименована/удалена — обнови гейт осознанно, а не удаляй проверку.',
    );
    return;
  }

  if (!keyBody.includes(`'${literal}'`)) {
    violations.push(
      `INV-2 (регресс D-25): showDeletedBadge в ${REPORT_TABLE} сравнивает reportType с ` +
        `'${literal}', но тело reportTypeKey() в ${REPORTS_PAGE} не содержит return-значения ` +
        `'${literal}'. Бейдж «Удалено» рендерится в CSV/PDF-экспорте, но никогда — в живой ` +
        'таблице приложения (несоответствие экрана и экспорта).',
    );
  }
}

// ---------------------------------------------------------------------------

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-report-type-parity.mjs [--src=<ui-dir>]`);
    process.exit(0);
  }

  const reportsPagePath = path.join(SRC_ROOT, REPORTS_PAGE);
  const reportTablePath = path.join(SRC_ROOT, REPORT_TABLE);

  let reportsPageSrc;
  let reportTableSrc;
  try {
    reportsPageSrc = fs.readFileSync(reportsPagePath, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${REPORTS_PAGE}.`);
    process.exit(1);
  }
  try {
    reportTableSrc = fs.readFileSync(reportTablePath, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${REPORT_TABLE}.`);
    process.exit(1);
  }

  const violations = [];
  checkInv1(reportsPageSrc, violations);
  checkInv2(reportsPageSrc, reportTableSrc, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений паритета reportType между экраном и экспортом.`,
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
