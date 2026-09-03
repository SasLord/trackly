#!/usr/bin/env node
// [check-print-idempotency] Постоянный гейт против регрессии
// UAT-40 lan-print-duplicate-first-page (Phase 40-27, HST-04).
//
// Почему он существует: печать/экспорт PDF отчёта «Перемещения» из
// LAN-браузера добавляла третий (дублированный) лист, потому что
// `printViaTopLevel()` рендерила Paged.js в долгоживущий разделяемый
// контейнер `#act-print-root` БЕЗ очистки перед рендером — единственная
// очистка висела на событии `afterprint`, а `Chunker.setup()` (pagedjs
// 0.4.3) всегда дописывает новый `.pagedjs_pages` через `appendChild`.
// `handlePrint()` не был защищён от повторного входа: кнопка «Печать»
// оставалась активной всё время пагинации, и повторный клик складывал
// вторую копию документа поверх первой. Фикс (см. `.planning/debug/
// lan-print-duplicate-first-page.md` и 40-27-PLAN.md) поднял состояние в
// component-scope и добавил очистку в НАЧАЛЕ вызова + re-entrancy guard —
// оба легко теряются при будущем рефакторинге, если их не держит гейт.
//
// Гейт СТРУКТУРНЫЙ (как check-print-isolation.mjs/check-place-path-short.mjs):
// читает исходник и проверяет наличие инвариантов текстовым/AST-лёгким
// разбором, а НЕ то, что печать реально не дублирует лист — это проверяется
// руками (см. <verification> в 40-27-PLAN.md).
//
// Проверяемые инварианты:
//   INV-1 — тело printViaTopLevel содержит `printRoot.innerHTML = ''` ДО
//           первого вызова `await previewer.preview(` — очистка происходит
//           до пагинации, а не только в cleanup()/afterprint.
//   INV-2 — вызов `registerHandlers(RepeatTableHeadHandler)` находится
//           внутри `if`-блока, завязанного на `repeatTableHeadHandlerRegistered`
//           — не выполняется безусловно на каждый клик.
//   INV-3 — тело handlePrint содержит идентификатор `printing` в условии
//           раннего return — re-entrancy guard не удалён при рефакторинге.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-print-idempotency.mjs              # проверить репозиторий
//   node scripts/check-print-idempotency.mjs --src=<dir>  # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-print-idempotency]';

const TARGET = 'src/features/acts/PdfPreviewModal.svelte';

/**
 * Тело функции (включая async) по имени — по образцу `functionBody()` в
 * check-place-path-short.mjs, но допускает необязательное `async` перед
 * `function`.
 */
function functionBody(src, fnName) {
  const m = src.match(new RegExp(`(?:async\\s+)?function\\s+${fnName}\\s*\\(`));
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

function checkInv1(src, violations) {
  const body = functionBody(src, 'printViaTopLevel');
  if (body === null) {
    violations.push({
      inv: 'INV-1',
      message: 'не найдена функция printViaTopLevel',
      hint: 'Функцию переименовали/удалили — очистка printRoot больше не проверяется гейтом. Обнови гейт вместе с рефакторингом, осознанно.',
    });
    return;
  }

  const previewIdx = body.indexOf('await previewer.preview(');
  if (previewIdx < 0) {
    violations.push({
      inv: 'INV-1',
      message: 'printViaTopLevel не содержит вызов await previewer.preview(',
      hint: 'Пагинация Paged.js больше не вызывается напрямую — гейт не может определить границу «до пагинации». Обнови гейт осознанно.',
    });
    return;
  }

  const beforePreview = body.slice(0, previewIdx);
  if (
    !beforePreview.includes("printRoot.innerHTML = ''") &&
    !beforePreview.includes('printRoot.innerHTML=""')
  ) {
    violations.push({
      inv: 'INV-1',
      message: 'printRoot.innerHTML не очищается ДО await previewer.preview(',
      hint: 'Это ровно регресс UAT-40 lan-print-duplicate-first-page: без очистки printRoot В НАЧАЛЕ вызова pagedjs дописывает новый .pagedjs_pages поверх результата предыдущего прогона (Chunker.setup() всегда appendChild), и печать/сохранение PDF даёт лишний (дублированный первый) лист.',
    });
  }
}

function checkInv2(src, violations) {
  const idx = src.indexOf('registerHandlers(RepeatTableHeadHandler)');
  if (idx < 0) {
    violations.push({
      inv: 'INV-2',
      message: 'не найден вызов registerHandlers(RepeatTableHeadHandler)',
      hint: 'Регистрацию обработчика переименовали/удалили. Обнови гейт вместе с рефакторингом, осознанно.',
    });
    return;
  }

  const before = src.slice(0, idx);
  const lines = before.split('\n');
  // Последняя НЕПУСТАЯ строка перед вызовом.
  let precedingLine = '';
  for (let i = lines.length - 1; i >= 0; i--) {
    if (lines[i].trim() !== '') {
      precedingLine = lines[i];
      break;
    }
  }

  if (
    !precedingLine.includes('if') ||
    !precedingLine.includes('repeatTableHeadHandlerRegistered')
  ) {
    violations.push({
      inv: 'INV-2',
      message:
        'registerHandlers(RepeatTableHeadHandler) вызывается без предшествующей проверки `if (...repeatTableHeadHandlerRegistered...)`',
      hint: 'Регистрация обработчика снова выполняется безусловно на каждый клик «Печать» — pagedjs копит дублирующиеся регистрации класса в своём глобальном реестре за время жизни компонента.',
    });
  }
}

function checkInv3(src, violations) {
  const body = functionBody(src, 'handlePrint');
  if (body === null) {
    violations.push({
      inv: 'INV-3',
      message: 'не найдена функция handlePrint',
      hint: 'Функцию переименовали/удалили — re-entrancy guard больше не проверяется гейтом. Обнови гейт вместе с рефакторингом, осознанно.',
    });
    return;
  }

  const returnMatch = body.match(/if\s*\(([^)]*)\)\s*return;/);
  if (!returnMatch || !returnMatch[1].includes('printing')) {
    violations.push({
      inv: 'INV-3',
      message: 'ранний return в handlePrint не содержит проверку `printing`',
      hint: 'Re-entrancy guard удалён: повторный клик «Печать» во время идущей пагинации снова может запустить второй параллельный прогон Paged.js и сложить дубликат страницы поверх первой.',
    });
  }
}

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-print-idempotency.mjs [--src=<ui-dir>]`);
    process.exit(0);
  }

  const full = path.join(SRC_ROOT, TARGET);
  let source;
  try {
    source = fs.readFileSync(full, 'utf8');
  } catch {
    console.error(
      `${TAG} FAIL — не удалось прочитать ${TARGET}. Компонент переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
    );
    process.exit(1);
  }

  const violations = [];
  checkInv1(source, violations);
  checkInv2(source, violations);
  checkInv3(source, violations);

  for (const v of violations) {
    console.error(`${TAG} ${TARGET} — ${v.inv}: ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений инвариантов идемпотентности печати в ${TARGET}. ` +
        'Этот класс дефекта уже уезжал в LAN-печать (UAT-40 lan-print-duplicate-first-page) — не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
