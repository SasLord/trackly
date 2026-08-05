#!/usr/bin/env node
// [check-print-isolation] Постоянный гейт против регрессий LAN-печати в
// `PdfPreviewModal.svelte` → `printViaTopLevel()`.
//
// Почему он существует: LAN-печать верстается ПРЯМО В DOM самого приложения
// (в отличие от desktop-пути, который пишет отдельный temp .html без единого
// стиля приложения). Из-за этого каскад приложения протекает в печатный
// вывод, и один и тот же класс дефекта регрессировал ТРИ раза подряд
// (быстрофиксы 260805-ifj, 260805-har, 260805-jwf), плюс отдельно —
// 260805-gdz-02. Ни один автотест эти правки не удерживал.
//
// Гейт СТРУКТУРНЫЙ: он читает исходник и проверяет, что инварианты (а не
// конкретные байты) на месте. Он НЕ доказывает, что печать рендерится
// правильно — рендер проверяется только руками (см. Manual-Only в
// 33-VALIDATION.md). Его единственная задача — громко падать, когда
// выстраданное исправление молча удалили при рефакторинге.
//
// Проверяемые инварианты:
//   INV-1a (260805-ifj + 260805-jwf) — сброс line-height/letter-spacing/
//           word-spacing объявлен БЕЗУСЛОВНО и на #act-print-root;
//   INV-1b (260805-jwf)              — этот сброс НЕ достаёт до body/html
//           приложения (иначе течёт обратно в UI — это и был дефект A);
//   INV-1c (260805-jwf)              — CSS шаблона не интерполируется второй
//           раз в общий документ приложения;
//   INV-1d (260805-har)              — в @media print фон бумаги принудительно
//           белый (и на body/html, и на .pagedjs_page);
//   INV-2  (260805-jwf)              — polisher захвачен и .destroy() вызван в
//           обработчике afterprint (иначе стили Paged.js остаются в head);
//   INV-3a (260805-gdz-02)           — на #act-print-root нигде не вешается
//           display: none (Paged.js меряет геометрию элемента);
//   INV-3b/c (260805-gdz-02)         — он спрятан офскрином и возвращается
//           в поток внутри @media print.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-print-isolation.mjs                 # проверить репозиторий
//   node scripts/check-print-isolation.mjs <path.svelte>   # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const DEFAULT_TARGET = path.join(UI_ROOT, 'src/features/acts/PdfPreviewModal.svelte');
const TAG = '[check-print-isolation]';

// ---------------------------------------------------------------------------
// Source scanning helpers
// ---------------------------------------------------------------------------

/**
 * Возвращает копию исходника, пригодную для структурного поиска:
 *   - содержимое комментариев затирается пробелами (иначе фраза
 *     `polisher.destroy()` из большого комментария-объяснения удовлетворила бы
 *     проверку INV-2 при удалённом реальном вызове);
 *   - содержимое template-литералов затирается (там живёт CSS с фигурными
 *     скобками, ломающими сопоставление блоков кода);
 *   - в обычных строковых литералах затираются только фигурные скобки
 *     (в файле есть строка с инлайн-скриптом, содержащая `{`/`}`).
 * Длина и позиции символов сохраняются один в один, поэтому индексы этой
 * копии применимы к оригиналу.
 */
function blankNonCode(src) {
  const out = src.split('');
  const n = src.length;
  let i = 0;
  const blank = (idx) => {
    if (src[idx] !== '\n') out[idx] = ' ';
  };
  while (i < n) {
    const c = src[i];
    const next = src[i + 1];
    if (c === '/' && next === '/') {
      while (i < n && src[i] !== '\n') blank(i++);
    } else if (c === '/' && next === '*') {
      blank(i++);
      blank(i++);
      while (i < n && !(src[i] === '*' && src[i + 1] === '/')) blank(i++);
      if (i < n) {
        blank(i++);
        blank(i++);
      }
    } else if (c === '`') {
      i++;
      while (i < n && src[i] !== '`') {
        if (src[i] === '\\') {
          blank(i++);
          if (i < n) blank(i++);
          continue;
        }
        blank(i++);
      }
      i++;
    } else if (c === "'" || c === '"') {
      const quote = c;
      i++;
      while (i < n && src[i] !== quote && src[i] !== '\n') {
        if (src[i] === '\\') {
          i += 2;
          continue;
        }
        if (src[i] === '{' || src[i] === '}') blank(i);
        i++;
      }
      i++;
    } else {
      i++;
    }
  }
  return out.join('');
}

/** Индекс парной закрывающей скобки для открывающей на позиции `openIdx`. */
function matchBrace(text, openIdx) {
  let depth = 0;
  for (let i = openIdx; i < text.length; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/** Тело функции по её имени: {start, end} — индексы внутри исходника. */
function findFunctionBody(blanked, fnName) {
  const sig = new RegExp(`function\\s+${fnName}\\s*\\(`);
  const m = blanked.match(sig);
  if (!m) return null;
  const openIdx = blanked.indexOf('{', m.index + m[0].length);
  if (openIdx < 0) return null;
  const closeIdx = matchBrace(blanked, openIdx);
  if (closeIdx < 0) return null;
  return { start: openIdx + 1, end: closeIdx };
}

/** Тело стрелочной/обычной функции, присвоенной идентификатору `name`. */
function findNamedCallbackBody(blanked, name) {
  const decl = new RegExp(`\\b(?:const|let|var|function)\\s+${name}\\b`);
  const m = blanked.match(decl);
  if (!m) return null;
  const openIdx = blanked.indexOf('{', m.index);
  if (openIdx < 0) return null;
  const closeIdx = matchBrace(blanked, openIdx);
  if (closeIdx < 0) return null;
  return { start: openIdx + 1, end: closeIdx };
}

// ---------------------------------------------------------------------------
// CSS parsing helpers (плоский разбор внедряемой печатной таблицы стилей)
// ---------------------------------------------------------------------------

function parseDeclarations(body) {
  const decls = [];
  for (const chunk of body.split(';')) {
    const part = chunk.trim();
    if (!part || part.includes('{')) continue;
    const sep = part.indexOf(':');
    if (sep < 0) continue;
    const prop = part.slice(0, sep).trim().toLowerCase();
    const rawValue = part
      .slice(sep + 1)
      .trim()
      .toLowerCase();
    if (!prop) continue;
    decls.push({
      prop,
      value: rawValue.replace('!important', '').trim(),
      important: rawValue.includes('!important'),
    });
  }
  return decls;
}

/** Плоский список правил: {selector, context: [@-правила], decls}. */
function parseCssRules(css, context = []) {
  const rules = [];
  let i = 0;
  let start = 0;
  while (i < css.length) {
    const ch = css[i];
    if (ch === '{') {
      const prelude = css.slice(start, i).trim();
      const end = matchBrace(css, i);
      if (end < 0) break;
      const body = css.slice(i + 1, end);
      if (prelude.startsWith('@')) {
        rules.push(...parseCssRules(body, [...context, prelude.toLowerCase()]));
      } else if (prelude) {
        rules.push({ selector: prelude, context, decls: parseDeclarations(body) });
      }
      i = end + 1;
      start = i;
    } else if (ch === '}') {
      i++;
      start = i;
    } else {
      i++;
    }
  }
  return rules;
}

const selectorParts = (selector) =>
  selector
    .split(',')
    .map((s) => s.replace(/\s+/g, ' ').trim().toLowerCase())
    .filter(Boolean);

const stripNot = (part) => part.replace(/:not\([^)]*\)/g, ' ');

const isUnconditional = (rule) => rule.context.length === 0;
const isPrintOnly = (rule) => rule.context.some((c) => /@media[^{]*\bprint\b/.test(c));

const decl = (rule, prop) => rule.decls.find((d) => d.prop === prop) ?? null;

const WHITE_VALUES = new Set(['#fff', '#ffffff', 'white', 'rgb(255,255,255)', 'rgb(255 255 255)']);
const isWhite = (value) => WHITE_VALUES.has(value.replace(/\s*,\s*/g, ',').trim());

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

const RESET_PROPS = ['line-height', 'letter-spacing', 'word-spacing'];

function checkPrintIsolation(source, label) {
  const violations = [];
  const fail = (inv, fixId, message, hint) => violations.push({ inv, fixId, message, hint, label });

  const blanked = blankNonCode(source);

  const idMatch = source.match(/const\s+PRINT_ROOT_ID\s*=\s*['"]([^'"]+)['"]/);
  if (!idMatch) {
    fail(
      'INV-0',
      '260805-gdz-02',
      'не найдена константа PRINT_ROOT_ID — структура printViaTopLevel изменилась настолько, ' +
        'что гейт больше ничего не проверяет',
      'Обнови этот гейт вместе с рефакторингом, не удаляй его.',
    );
    return violations;
  }
  const printRootId = idMatch[1];
  const rootSel = `#${printRootId}`;

  const fn = findFunctionBody(blanked, 'printViaTopLevel');
  if (!fn) {
    fail(
      'INV-0',
      '260805-jwf',
      'функция printViaTopLevel() не найдена — путь LAN-печати переименован или удалён',
      'Обнови этот гейт вместе с рефакторингом, не удаляй его.',
    );
    return violations;
  }
  const fnBlanked = blanked.slice(fn.start, fn.end);
  const fnSource = source.slice(fn.start, fn.end);

  // --- внедряемая таблица стилей -------------------------------------------
  const assign = fnBlanked.match(/\.textContent\s*=\s*/);
  let cssRaw = null;
  if (assign) {
    const tickStart = fnSource.indexOf('`', assign.index + assign[0].length);
    if (tickStart >= 0) {
      const tickEnd = fnSource.indexOf('`', tickStart + 1);
      if (tickEnd > tickStart) cssRaw = fnSource.slice(tickStart + 1, tickEnd);
    }
  }

  let rules = [];
  if (cssRaw === null) {
    fail(
      'INV-1a',
      '260805-ifj / 260805-har / 260805-jwf',
      'в printViaTopLevel() не найден внедряемый печатный стиль (присваивание textContent ' +
        'шаблонной строкой) — вместе с ним пропали ВСЕ изоляционные правила каскада',
      'Печать в LAN-режиме рендерится в DOM самого приложения; без этого блока в бумагу ' +
        'протекают line-height, фон темы и вёрстка приложения.',
    );
  } else {
    const cssResolved = cssRaw
      .replace(/\$\{\s*PRINT_ROOT_ID\s*\}/g, printRootId)
      .replace(/\/\*[\s\S]*?\*\//g, ' ');
    rules = parseCssRules(cssResolved);

    const rootRules = rules.filter((r) =>
      selectorParts(r.selector).some((p) => stripNot(p).includes(rootSel)),
    );
    const unconditionalRoot = rootRules.filter(isUnconditional);
    const printRoot = rootRules.filter(isPrintOnly);

    // --- INV-1a: безусловный сброс наследуемой типографики на #act-print-root
    const missingReset = RESET_PROPS.filter(
      (prop) => !unconditionalRoot.some((r) => decl(r, prop)?.value === 'normal'),
    );
    if (missingReset.length > 0) {
      const onlyInPrint = missingReset.filter((prop) =>
        rules.some((r) => isPrintOnly(r) && decl(r, prop)?.value === 'normal'),
      );
      if (onlyInPrint.length > 0) {
        fail(
          'INV-1a',
          '260805-jwf',
          `сброс ${onlyInPrint.join('/')} объявлен только внутри @media print, а не безусловно`,
          'Paged.js меряет и разбивает DOM НА ЭКРАНЕ, до window.print(): print-only сброс даёт ' +
            'пагинацию по одной типографике и печать по другой (дефект B, регресс 260805-ifj). ' +
            `Правило должно быть на ${rootSel} вне любого @media.`,
        );
      } else {
        fail(
          'INV-1a',
          '260805-ifj + 260805-jwf',
          `на ${rootSel} нет безусловного сброса: ${missingReset.join(', ')} (ожидается ` +
            'значение normal)',
          `Без него ${rootSel} наследует body { line-height: 1.5 } из global.scss, и LAN-печать ` +
            'расходится с desktop-печатью (там стилей приложения просто нет).',
        );
      }
    }

    // --- INV-1b: сброс не должен доставать до самого приложения
    for (const rule of rules) {
      const reachesApp = selectorParts(rule.selector).some(
        (p) => /(^|[\s>+~])(html|body)\b/.test(stripNot(p)) && !stripNot(p).includes(rootSel),
      );
      if (!reachesApp) continue;
      const leaked = RESET_PROPS.filter((prop) => decl(rule, prop));
      if (leaked.length > 0) {
        fail(
          'INV-1b',
          '260805-jwf',
          `правило «${rule.selector}» задаёт ${leaked.join('/')} на body/html самого приложения`,
          `Стиль внедряется в общий документ приложения — сброс обязан быть заскоупен на ` +
            `${rootSel}, иначе он меняет экранную типографику UI (дефект A).`,
        );
      }
    }

    // --- INV-1c: CSS шаблона не дублируется в документ приложения
    const cssVarMatch = fnBlanked.match(/\.preview\s*\([^)]*?:\s*(\w+)\s*\}/);
    const cssVar = cssVarMatch ? cssVarMatch[1] : null;
    if (cssVar && new RegExp(`\\$\\{\\s*${cssVar}\\s*\\}`).test(cssRaw)) {
      fail(
        'INV-1c',
        '260805-jwf',
        `CSS шаблона (\`${cssVar}\`) снова интерполируется во внедряемый стиль приложения`,
        'previewer.preview() уже применяет ровно эту таблицу стилей через аргумент stylesheets. ' +
          'Дубль попадает в общий документ приложения без скоупа и подменяет шрифт UI до ' +
          'перезагрузки (дефект A).',
      );
    }

    // --- INV-1d: бумага всегда белая, независимо от темы
    const whiteBg = (rule) => {
      const d = decl(rule, 'background') ?? decl(rule, 'background-color');
      return d != null && d.important && isWhite(d.value);
    };
    const hasBodyWhite = rules.some(
      (r) =>
        isPrintOnly(r) &&
        selectorParts(r.selector).some((p) => /^(html|body)$/.test(p)) &&
        whiteBg(r),
    );
    if (!hasBodyWhite) {
      fail(
        'INV-1d',
        '260805-har',
        'в @media print нет правила html/body { background: #fff !important }',
        'global.scss задаёт body { background: var(--tr-bg) }: без нейтрализации на бумагу ' +
          'уходит серый (светлая тема) или почти чёрный (тёмная) фон приложения.',
      );
    }
    const hasSheetWhite = rules.some(
      (r) =>
        isPrintOnly(r) &&
        selectorParts(r.selector).some((p) => p.includes('.pagedjs_page')) &&
        whiteBg(r),
    );
    if (!hasSheetWhite) {
      fail(
        'INV-1d',
        '260805-har',
        'в @media print нет правила .pagedjs_page { background: #fff !important }',
        'Лист Paged.js обязан быть белой бумагой в обеих темах (D-08), как и в экранном превью.',
      );
    }

    // --- INV-3a: никакого display:none на печатном корне
    for (const rule of rootRules) {
      const d = decl(rule, 'display');
      if (d && d.value === 'none') {
        const where = isUnconditional(rule)
          ? 'безусловно'
          : `в контексте ${rule.context.join(' ')}`;
        fail(
          'INV-3a',
          '260805-gdz-02',
          `на ${rootSel} снова навешен display: none (${where}, правило «${rule.selector}»)`,
          'display: none обнуляет getBoundingClientRect для всего поддерева, а previewer.preview() ' +
            'меряет реальную геометрию, чтобы разбить содержимое на страницы — пагинация ломается. ' +
            'Прятать только офскрином (position: absolute; left: -100000px).',
        );
      }
    }

    // --- INV-3b/c: офскрин + возврат в поток при печати
    const offscreen = unconditionalRoot.find(
      (r) => decl(r, 'position')?.value === 'absolute' && /^-\d/.test(decl(r, 'left')?.value ?? ''),
    );
    if (!offscreen) {
      fail(
        'INV-3b',
        '260805-gdz-02',
        `${rootSel} больше не спрятан офскрином (нет безусловного position: absolute + ` +
          'отрицательного left)',
        'Контейнер должен оставаться вне вьюпорта, но с реальным layout-боксом — Paged.js ' +
          'паginирует его измерением DOM.',
      );
    } else {
      const resetsInPrint = printRoot.some(
        (r) => decl(r, 'position')?.value === 'static' && decl(r, 'left') != null,
      );
      if (!resetsInPrint) {
        fail(
          'INV-3c',
          '260805-gdz-02',
          `в @media print нет возврата ${rootSel} в поток (position: static + left)`,
          'Иначе офскрин-смещение уезжает в печатный вывод и страница печатается пустой.',
        );
      }
    }
  }

  // --- INV-2: polisher.destroy() в обработчике afterprint ------------------
  const polisherMatch = fnBlanked.match(/(\w+)\s*=\s*\w+\s*\.\s*polisher\b/);
  if (!polisherMatch) {
    fail(
      'INV-2',
      '260805-jwf',
      'printViaTopLevel() больше не захватывает previewer.polisher',
      'Polisher.insert() добавляет <style data-pagedjs-inserted-styles> в head общего документа ' +
        'приложения при каждом preview(); destroy() — единственный путь их убрать.',
    );
  } else {
    const polisherVar = polisherMatch[1];
    const destroyRe = new RegExp(`\\b${polisherVar}\\s*\\??\\.\\s*destroy\\s*\\(`);
    // Подписка ищется без предположений о форме обработчика; имя (если он
    // вынесен в переменную) — отдельной, необязательной группой, иначе
    // инлайн-стрелка давала бы ложное срабатывание.
    const hasAfterprint = /addEventListener\(\s*['"]afterprint['"]/.test(fnBlanked);
    const afterprint = fnBlanked.match(
      /addEventListener\(\s*['"]afterprint['"]\s*,\s*([A-Za-z_$][\w$]*)\s*[,)]/,
    );
    if (!hasAfterprint) {
      fail(
        'INV-2',
        '260805-jwf',
        'в printViaTopLevel() нет подписки на событие afterprint',
        'Очистка (включая polisher.destroy()) выполняется именно по afterprint; без подписки ' +
          'стили Paged.js остаются в документе приложения навсегда.',
      );
    } else {
      const handlerBody = afterprint ? findNamedCallbackBody(fnBlanked, afterprint[1]) : null;
      const scope = handlerBody ? fnBlanked.slice(handlerBody.start, handlerBody.end) : fnBlanked; // инлайн-обработчик: проверяем грубее, но без ложных срабатываний
      if (!destroyRe.test(scope)) {
        fail(
          'INV-2',
          '260805-jwf',
          `${polisherVar}.destroy() не вызывается в обработчике afterprint` +
            (handlerBody ? '' : ' (обработчик встроен — проверена вся функция)'),
          'Без destroy() вставленные Paged.js стили шаблона переживают цикл печати и остаются ' +
            'в документе приложения до перезагрузки страницы.',
        );
      }
    }
  }

  return violations;
}

// ---------------------------------------------------------------------------

function main() {
  const arg = process.argv[2];
  const target = arg ? path.resolve(process.cwd(), arg) : DEFAULT_TARGET;

  let source;
  try {
    source = fs.readFileSync(target, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${target}`);
    process.exit(1);
  }

  const label = path.relative(UI_ROOT, target);
  const violations = checkPrintIsolation(source, label);

  for (const v of violations) {
    console.error(`${TAG} ${label} — ${v.inv} (регресс быстрофикса ${v.fixId}): ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений изоляции каскада на пути LAN-печати. ` +
        'Этот класс дефекта уже регрессировал трижды — не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
