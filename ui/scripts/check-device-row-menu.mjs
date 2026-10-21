#!/usr/bin/env node
// [check-device-row-menu] Гейт меню строки устройства (фаза 41.1, план 16;
// GRD-01, GRD-02, GRD-05; D-22, D-24, D-25, D-26, D-38).
//
// Два слоя.
//
// 1. ИСПОЛНЯЮЩИЙ golden-гейт чистого модуля `src/features/devices/deviceRowMenu.ts`.
//    Модуль транспилируется пакетом `typescript` и гоняется по фикстуре
//    `scripts/fixtures/device-row-menu/cases.json`. Каждый кейс подобран так,
//    чтобы реальная ошибка (порядок секций, лишний пункт, забытый статус,
//    потерянная часть селектора, всегда-истинная охрана) меняла результат.
//    Правила видимости зеркалят отказы сервера (план 11), но общей фикстуры с
//    Rust у модуля НЕТ: сервер остаётся границей безопасности, UI только
//    заранее прячет пункты.
//
// 2. СТРУКТУРНЫЕ правила на исходниках `src/` (комментарии вырезаются):
//    [A] в DeviceRowMenu.svelte ровно один `<hr` с role="separator"; он лежит
//        внутри `{#each}` по секциям из buildMenuSections и НЕ внутри вложенного
//        цикла по пунктам;
//    [B] во всём `src/` нет литералов встроенных подписей быстрых действий
//        («Сформировать группу», «Замещение группой», «На разбор»): они живут
//        только на сервере, в UI подпись приходит из типа группы (D-22);
//    [C] в deviceRowMenu.ts есть `quick_action_enabled` и нет этих подписей;
//    [G] DeviceRowMenu.svelte зовёт `quickActionsForDevice(` и НЕ зовёт
//        `quickActionsFromTypes(` напрямую (иначе правило «создающие группу
//        действия только у type_id = 1» (D-38) обходится);
//    [F] охрана событий строки (D-25, D-26). DeviceRowMenu стоит внутри <td>, а
//        Modal не портальный: правый клик в поле модалки, двойной клик по
//        инвентарному номеру в просмотре и пробел в поле названия иначе
//        перехватывает строка. Для каждого файла строки с привязками
//        oncontextmenu= / ondblclick= / onkeydown=:
//          F1 (счёт): isRowOverlayEvent( не меньше, чем привязок contextmenu +
//             dblclick; при onkeydown= есть хотя бы один isOwnKeyEvent(;
//          F2 (охрана В обработчике): значение привязки разрешается в тело
//             функции того же файла (или инлайн-стрелки); тело начинается с
//             `if (`, первым операндом условия стоит охрана (для contextmenu
//             и dblclick: isRowOverlayEvent( БЕЗ отрицания; для keydown:
//             !isOwnKeyEvent(), а потребитель условия — return. Не
//             разрешилось — нарушение, а не SKIP;
//          F3 (порядок): охрана раньше preventDefault, stopPropagation,
//             getSelection, removeAllRanges, openAt(, openView(,
//             openFromElement(, toggle.
//        Счёт F1 порядка и места не видит: охрана ПОСЛЕ preventDefault или в
//        другой функции файла проходит F1 зелёной — это именно та дыра, ради
//        которой есть F2 и F3.
//        Набор проверяемых файлов выводится обходом: ROW_FILES (ожидаемые
//        строки) плюс КАЖДЫЙ .svelte, содержащий `<DeviceRowMenu`. Новый файл,
//        подключивший меню, попадает под правило сам. Файл без привязок
//        печатает явный SKIP; обязательный (MANDATORY_ROW_FILES) без привязок
//        или без самого файла — нарушение.
//
//    [F4] охрана click на ячейках строки (план 41.1-26, WR-02). `onclick` НЕ входит
//        в общий BINDING_RE (он совпал бы с Button, RowChevron и ссылками), поэтому
//        привязки `onclick={...}` ищутся только на открывающих тегах `<td` (конец
//        тега считается с учётом `{...}`, чтобы `>` в выражении не обрезал его).
//        Значение разрешается в тело функции файла и проверяется той же
//        checkGuardedBody: первым оператором `if (isRowOverlayEvent(...)) return;`
//        БЕЗ отрицания, до любого `onclick(`, preventDefault и т.д. Нарушения
//        охраны: [F4] (нет, не та полярность, не первым оператором, не
//        разрешилось); действие раньше охраны даёт ещё и [F3]. Перестановка
//        `onclick(); if (isRowOverlayEvent(...)) return;` проходит счёт F1, но
//        краснеет [F4]+[F3]. Файлы из REQUIRED_TD_CLICK_ROWS обязаны иметь хотя
//        бы одну такую привязку (снятая привязка не должна давать зелёного по
//        пустоте, иначе [F0]). Счёт F1 учитывает td-click: isRowOverlayEvent(
//        не меньше, чем привязок contextmenu + dblclick + td-click.
//
//    [D] вторая реализация меню не возвращается (план 41.1-17, D-21): файла
//        src/features/devices/DeviceContextMenu.svelte нет; идентификатор
//        DeviceContextMenu не встречается в src вне комментариев; классы
//        `ctx-menu-` допустимы только в src/features/cartridges/.
//
// Усиление относительно текста плана: полярность охраны проверяется (overlay
// без `!`, own-key только с `!`): охрана с обратной полярностью выходит на
// СВОИХ событиях строки и убивает меню.
//
// Self-test (`--selftest`): мутанты РЕАЛЬНОГО исходника deviceRowMenu.ts (якорь
// каждого встречается ровно один раз, иначе replace молча отключил бы соседний
// фрагмент) и мутанты структурных правил на фикстурах в памяти (у каждой
// «до» зелёная, «после» красная с указанным кодом). Только node:fs/node:path/
// node:url/node:module.
//
// Usage:
//   node scripts/check-device-row-menu.mjs               # проверить репозиторий
//   node scripts/check-device-row-menu.mjs --src=<root>  # КОРЕНЬ копии с каталогом src/ внутри
//   node scripts/check-device-row-menu.mjs --impl=<path> # копия deviceRowMenu.ts для golden-слоя
//   node scripts/check-device-row-menu.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';
import { stripComments } from './_strip.mjs';
export { stripComments };

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-row-menu]';

const MODULE_FILE = 'src/features/devices/deviceRowMenu.ts';
const MENU_FILE = 'src/features/devices/DeviceRowMenu.svelte';
const FIXTURE = path.join(__dirname, 'fixtures/device-row-menu/cases.json');
const MIN_CASES = 28;
// Сколько .svelte/.ts обязан увидеть структурный проход: меньше — гейт слеп.
const MIN_SCANNED_FILES = 50;
const SELFTEST_MIN_SCANNED_FILES = 1;

// Ожидаемые файлы строк. Файл без привязок или отсутствующий — явный SKIP.
const ROW_FILES = [
  'src/features/devices/DeviceListRow.svelte',
  'src/features/devices/DeviceGroupRow.svelte',
  'src/features/printers/PrinterListRow.svelte',
];
// Обязательные: отсутствие файла или привязок — нарушение, SKIP недопустим.
// План 17 внёс DeviceListRow и DeviceGroupRow, план 19 (Task 1) — PrinterListRow.
const MANDATORY_ROW_FILES = [
  'src/features/devices/DeviceListRow.svelte',
  'src/features/devices/DeviceGroupRow.svelte',
  'src/features/printers/PrinterListRow.svelte',
];
// Привязки, которые обязательная строка ОБЯЗАНА иметь (закрывает слепое пятно
// плана 41.1-18: снятие `ondblclick=` не краснило ни F1 (считает охраны, а не
// привязки), ни F0 (привязки есть, пока осталась хоть одна), и двойной клик молча
// переставал открывать просмотр). Запись обязательна для КАЖДОГО файла из
// MANDATORY_ROW_FILES: отсутствие записи — нарушение [F0] (самоинвалидация).
// У DeviceGroupRow контекстного меню и клавиатурной привязки нет по построению
// (плана 17: у свёртки нет сущности для меню, D-26).
const REQUIRED_BINDINGS = {
  'src/features/devices/DeviceListRow.svelte': ['oncontextmenu', 'ondblclick', 'onkeydown'],
  'src/features/devices/DeviceGroupRow.svelte': ['ondblclick'],
  'src/features/printers/PrinterListRow.svelte': ['oncontextmenu', 'ondblclick', 'onkeydown'],
};

// Файлы, у которых обязана быть хотя бы одна привязка `onclick` на <td> (правило
// [F4]). Остальные строки (DeviceListRow, DeviceGroupRow) таких привязок не
// имеют и проходят правило без записи здесь.
const REQUIRED_TD_CLICK_ROWS = ['src/features/printers/PrinterListRow.svelte'];

// [D] Вторая реализация меню удалена в плане 41.1-17 и не должна вернуться.
const LEGACY_MENU_FILE = 'src/features/devices/DeviceContextMenu.svelte';
const LEGACY_MENU_IDENT = /\bDeviceContextMenu\b/;
const LEGACY_CLASS = /\bctx-menu-/;
// Общие .ctx-menu-* стили живут только у картриджных меню.
const LEGACY_CLASS_ALLOWED_DIR = 'src/features/cartridges/';

const BUILTIN_LABELS = ['Сформировать группу', 'Замещение группой', 'На разбор'];
const OVERLAY_TOKENS = ['.modal-backdrop', '[role="dialog"]', '[role="menu"]'];
const FORBIDDEN_BEFORE_GUARD = new RegExp(
  [
    'preventDefault',
    'stopPropagation',
    'getSelection',
    'removeAllRanges',
    '\\bopenAt\\s*\\(',
    '\\bopenView\\s*\\(',
    '\\bopenFromElement\\s*\\(',
    // Тело обработчика, а не атрибут `onclick=` разметки: вызов пропса строки.
    '\\bonclick\\s*\\(',
    'toggle',
  ].join('|'),
);

// ---------------------------------------------------------------------------
// Слой 1: исполняемый golden-гейт
// ---------------------------------------------------------------------------

function loadTypescript() {
  const require = createRequire(import.meta.url);
  try {
    return require('typescript');
  } catch {
    console.error(
      `${TAG} FAIL — не разрешается пакет \`typescript\` (devDependency ui/package.json); запусти \`pnpm install\` в ui/.`,
    );
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
  // data:-URL: модуль самодостаточен (ни одного import, кроме типов).
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

function loadFixture() {
  let cases;
  try {
    cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  } catch (e) {
    console.error(
      `${TAG} FAIL — фикстура ${path.relative(UI_ROOT, FIXTURE)} не читается: ${e.message}`,
    );
    process.exit(1);
  }
  if (!Array.isArray(cases) || cases.length < MIN_CASES) {
    console.error(
      `${TAG} FAIL — в фикстуре ${Array.isArray(cases) ? cases.length : 'не массив'} кейсов, ожидалось не меньше ${MIN_CASES}. Её урезали вместо починки функции?`,
    );
    process.exit(1);
  }
  return cases;
}

function canonical(v) {
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v);
}

/** Фейковая цель события по дескриптору из фикстуры. */
function makeTarget(desc) {
  switch (desc.kind) {
    case 'null':
      return null;
    case 'undefined':
      return undefined;
    case 'no-closest':
      return {};
    case 'closest-miss':
      return { closest: () => null };
    case 'closest-hit-token':
      // Попадание только если селектор содержит токен: часть селектора, убранная
      // из реализации, перестаёт срабатывать.
      return { closest: (sel) => (String(sel).includes(desc.token) ? {} : null) };
    default:
      throw new Error(`неизвестный дескриптор цели «${desc.kind}»`);
  }
}

const FUNCTIONS = [
  'buildMenuSections',
  'isGroupActionAllowed',
  'quickActionsFromTypes',
  'quickActionsForDevice',
  'isRowOverlayEvent',
  'isOwnKeyEvent',
];

/** Возвращает список текстов провалов (пустой — всё в порядке). */
function runCases(mod, cases) {
  const failures = [];
  for (const fn of FUNCTIONS) {
    if (typeof mod[fn] !== 'function') failures.push(`модуль не экспортирует функцию ${fn}`);
  }
  if (typeof mod.ROW_OVERLAY_SELECTOR !== 'string') {
    failures.push('модуль не экспортирует ROW_OVERLAY_SELECTOR');
  } else {
    for (const tok of OVERLAY_TOKENS) {
      if (!mod.ROW_OVERLAY_SELECTOR.includes(tok)) {
        failures.push(`ROW_OVERLAY_SELECTOR не содержит часть ${tok}`);
      }
    }
  }
  if (mod.ANCHOR_DEVICE_TYPE_ID !== 1) {
    failures.push(`ANCHOR_DEVICE_TYPE_ID равен ${mod.ANCHOR_DEVICE_TYPE_ID}, ожидалось 1`);
  }
  if (failures.length) return failures;

  const covered = new Set();
  const hitTokens = new Set();
  for (const c of cases) {
    covered.add(c.fn);
    try {
      let got;
      if (c.fn === 'buildMenuSections') {
        const before = canonical(c.input);
        got = mod.buildMenuSections(JSON.parse(before));
        if (!Array.isArray(got) || got.some((s) => !Array.isArray(s) || s.length === 0)) {
          failures.push(`${c.name}: пустая секция или не массив секций: ${canonical(got)}`);
          continue;
        }
      } else if (c.fn === 'isGroupActionAllowed') {
        got = mod.isGroupActionAllowed(c.input);
      } else if (c.fn === 'quickActionsFromTypes') {
        const types = JSON.parse(JSON.stringify(c.input));
        got = mod.quickActionsFromTypes(types);
        if (canonical(types) !== canonical(c.input)) {
          failures.push(`${c.name}: функция мутировала входной список типов`);
        }
      } else if (c.fn === 'quickActionsForDevice') {
        got = mod.quickActionsForDevice(
          JSON.parse(JSON.stringify(c.input.types)),
          c.input.deviceTypeId,
        );
      } else if (c.fn === 'isRowOverlayEvent') {
        got = mod.isRowOverlayEvent(makeTarget(c.input.target));
        if (c.input.target.kind === 'closest-hit-token' && c.expected === true) {
          hitTokens.add(c.input.target.token);
        }
      } else if (c.fn === 'isOwnKeyEvent') {
        const objs = { A: { id: 'A' }, B: { id: 'B' } };
        const pick = (k) => (k === null ? null : objs[k]);
        got = mod.isOwnKeyEvent(pick(c.input.target), pick(c.input.currentTarget));
      } else {
        failures.push(`${c.name}: неизвестная функция «${c.fn}» в фикстуре`);
        continue;
      }
      if (canonical(got) !== canonical(c.expected)) {
        failures.push(`${c.name}: получено ${canonical(got)}, ожидалось ${canonical(c.expected)}`);
      }
    } catch (e) {
      failures.push(`${c.name}: исключение ${e.message}`);
    }
  }
  for (const fn of FUNCTIONS) {
    if (!covered.has(fn)) failures.push(`в фикстуре нет ни одного кейса для ${fn}`);
  }
  for (const tok of OVERLAY_TOKENS) {
    if (!hitTokens.has(tok)) {
      failures.push(`в фикстуре нет положительного кейса isRowOverlayEvent для части ${tok}`);
    }
  }
  return failures;
}

// ---------------------------------------------------------------------------
// Слой 2: разбор исходников
// ---------------------------------------------------------------------------

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
}

/** Делит .svelte на script и markup (script/style в markup затираются). */
function splitSvelte(src) {
  let script = '';
  const markup = src
    .replace(/<script\b[^>]*>([\s\S]*?)<\/script>/g, (m, body) => {
      script += `${body}\n`;
      return blank(m);
    })
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/g, blank);
  return { script, markup };
}

/** Индекс закрывающей скобки для открывающей в text[openIdx]; -1 — не нашлось. */
function readBalanced(text, openIdx, open, close) {
  let depth = 0;
  let quote = null;
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === '\\') i++;
      else if (c === quote) quote = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      quote = c;
      continue;
    }
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function lineOf(text, idx) {
  return text.slice(0, idx).split('\n').length;
}

// --- [F]: привязки, разрешение обработчика, охрана ----------------------------

const BINDING_RE = /\b(oncontextmenu|ondblclick|onkeydown)\s*=\s*\{/g;

function extractBindings(markup) {
  const out = [];
  BINDING_RE.lastIndex = 0;
  let m;
  while ((m = BINDING_RE.exec(markup)) !== null) {
    const open = m.index + m[0].length - 1;
    const close = readBalanced(markup, open, '{', '}');
    if (close === -1) {
      out.push({ attr: m[1], expr: null, line: lineOf(markup, m.index) });
      continue;
    }
    out.push({
      attr: m[1],
      expr: markup.slice(open + 1, close).trim(),
      line: lineOf(markup, m.index),
    });
  }
  return out;
}

/** Индекс `>`, закрывающего открывающий тег, начиная с `<`; `{...}` и кавычки пропускаются. */
function findTagEnd(text, start) {
  let i = start;
  while (i < text.length) {
    const c = text[i];
    if (c === '{') {
      const e = readBalanced(text, i, '{', '}');
      if (e === -1) return -1;
      i = e + 1;
      continue;
    }
    if (c === '"' || c === "'") {
      const e = text.indexOf(c, i + 1);
      if (e === -1) return -1;
      i = e + 1;
      continue;
    }
    if (c === '>') return i;
    i++;
  }
  return -1;
}

/** [F4]: привязки `onclick={...}` только на открывающих тегах <td. */
function extractTdClickBindings(markup) {
  const out = [];
  const tdRe = /<td\b/g;
  let m;
  while ((m = tdRe.exec(markup)) !== null) {
    const end = findTagEnd(markup, m.index);
    const tag = markup.slice(m.index, end === -1 ? markup.length : end);
    const attr = /\bonclick\s*=\s*\{/.exec(tag);
    if (!attr) continue;
    const open = m.index + attr.index + attr[0].length - 1;
    const close = readBalanced(markup, open, '{', '}');
    out.push({
      attr: 'onclick',
      expr: close === -1 ? null : markup.slice(open + 1, close).trim(),
      line: lineOf(markup, m.index + attr.index),
    });
  }
  return out;
}

/**
 * Сколько РАЗНЫХ обработчиков у td-click привязок: одна функция на несколько ячеек
 * (handleClick у трёх <td> строки принтера) охраняется одной охраной, поэтому для
 * счёта F1 она считается один раз; инлайн-выражения считаются каждое отдельно.
 */
function tdClickHandlerCount(tdClicks) {
  const named = new Set();
  let inline = 0;
  for (const b of tdClicks) {
    if (b.expr !== null && /^[A-Za-z_$][\w$]*$/.test(b.expr)) named.add(b.expr);
    else inline++;
  }
  return named.size + inline;
}

/** Тело функции/стрелки, начиная с позиции сразу после списка параметров. */
function bodyAfterParams(text, afterParams, kind) {
  let i = afterParams;
  if (kind === 'function') {
    const brace = text.indexOf('{', i);
    if (brace === -1) return null;
    const end = readBalanced(text, brace, '{', '}');
    return end === -1 ? null : text.slice(brace + 1, end);
  }
  const arrow = text.indexOf('=>', i);
  if (arrow === -1) return null;
  i = arrow + 2;
  while (i < text.length && /\s/.test(text[i])) i++;
  if (text[i] === '{') {
    const end = readBalanced(text, i, '{', '}');
    return end === -1 ? null : text.slice(i + 1, end);
  }
  // Выражение вместо блока: до конца оператора.
  const semi = text.indexOf(';', i);
  return text.slice(i, semi === -1 ? text.length : semi);
}

/** Тело обработчика по значению привязки; null — не разрешилось. */
function resolveHandlerBody(expr, script) {
  if (expr === null) return null;
  if (/^[A-Za-z_$][\w$]*$/.test(expr)) {
    const fnDecl = new RegExp(`function\\s+${expr.replace(/\$/g, '\\$')}\\s*\\(`).exec(script);
    if (fnDecl) {
      const open = fnDecl.index + fnDecl[0].length - 1;
      const close = readBalanced(script, open, '(', ')');
      return close === -1 ? null : bodyAfterParams(script, close + 1, 'function');
    }
    const constDecl = new RegExp(
      `(?:const|let|var)\\s+${expr.replace(/\$/g, '\\$')}\\b[^=;]*=\\s*(?:async\\s+)?(\\()`,
    ).exec(script);
    if (constDecl) {
      const open = constDecl.index + constDecl[0].length - 1;
      const close = readBalanced(script, open, '(', ')');
      return close === -1 ? null : bodyAfterParams(script, close + 1, 'arrow');
    }
    return null;
  }
  const inline = /^(?:async\s*)?\(/.exec(expr);
  if (inline) {
    const open = inline[0].length - 1;
    const close = readBalanced(expr, open, '(', ')');
    if (close === -1) return null;
    if (!/^\s*(?::[^=]+)?=>/.test(expr.slice(close + 1))) return null;
    return bodyAfterParams(expr, close + 1, 'arrow');
  }
  if (/^(?:async\s+)?[A-Za-z_$][\w$]*\s*=>/.test(expr)) return bodyAfterParams(expr, 0, 'arrow');
  return null;
}

const GUARD_BY_ATTR = {
  onclick: { name: 'isRowOverlayEvent', negated: false },
  oncontextmenu: { name: 'isRowOverlayEvent', negated: false },
  ondblclick: { name: 'isRowOverlayEvent', negated: false },
  onkeydown: { name: 'isOwnKeyEvent', negated: true },
};

/** Проверка F2/F3 для одного тела. Возвращает [{code, msg}]. */
function checkGuardedBody(body, attr) {
  const { name, negated } = GUARD_BY_ATTR[attr];
  const out = [];
  const t = body.trimStart();
  const ifm = /^if\s*\(/.exec(t);
  if (!ifm) {
    out.push({
      code: 'F2',
      msg: `тело ${attr} не начинается с \`if (\`: охрана ${name}( обязана быть первым оператором`,
    });
  } else {
    const open = ifm[0].length - 1;
    const close = readBalanced(t, open, '(', ')');
    const cond = close === -1 ? '' : t.slice(open + 1, close).trim();
    const hasNeg = cond.startsWith('!');
    const bare = cond.replace(/^!+\s*/, '');
    if (!new RegExp(`^${name}\\s*\\(`).test(bare)) {
      out.push({
        code: 'F2',
        msg: `первым операндом условия первого if в ${attr} должна стоять ${name}(`,
      });
    } else if (hasNeg !== negated) {
      out.push({
        code: 'F2',
        msg: `охрана ${name}( в ${attr} с неверной полярностью: ${negated ? 'нужно `!' + name + '(`' : 'без `!`'}`,
      });
    }
    const rest = close === -1 ? '' : t.slice(close + 1).trimStart();
    if (!/^(?:\{\s*)?return\b/.test(rest)) {
      out.push({ code: 'F2', msg: `охрана в ${attr} должна выходить через return` });
    }
  }
  const guardIdx = body.search(new RegExp(`\\b${name}\\s*\\(`));
  const forbidden = body.search(FORBIDDEN_BEFORE_GUARD);
  if (forbidden !== -1 && (guardIdx === -1 || guardIdx > forbidden)) {
    out.push({ code: 'F3', msg: `в теле ${attr} действие стоит раньше охраны ${name}(` });
  }
  return out;
}

/** Правило [F] для одного файла. mandatory — отсутствие привязок нарушение. */
export function checkRowFile(rel, src, mandatory, required = null, requiredTdClick = false) {
  const violations = [];
  const notes = [];
  if (src === null) {
    if (mandatory)
      violations.push({ code: 'F0', file: rel, msg: 'обязательный файл строки не найден' });
    else notes.push(`SKIP ${rel}: файла нет`);
    return { violations, notes };
  }
  const clean = stripComments(src);
  const { script, markup } = splitSvelte(clean);
  const bindings = extractBindings(markup);
  // [F4]: охрана click на <td>, с учётом порядка (checkGuardedBody).
  const tdClicks = extractTdClickBindings(markup);
  if (requiredTdClick && tdClicks.length === 0) {
    violations.push({
      code: 'F0',
      file: rel,
      msg: 'обязательная привязка onclick= на <td> снята (клик по ячейке перестал выбирать строку, а охрана в теле осталась бы зелёной по пустоте)',
    });
  }
  for (const b of tdClicks) {
    const body = resolveHandlerBody(b.expr, script);
    if (body === null) {
      violations.push({
        code: 'F4',
        file: rel,
        msg: `строка ${b.line}: значение onclick={${b.expr ?? '?'}} на <td не разрешается в тело функции файла`,
      });
      continue;
    }
    for (const v of checkGuardedBody(body, 'onclick')) {
      violations.push({
        code: v.code === 'F2' ? 'F4' : v.code,
        file: rel,
        msg: `строка ${b.line}: ${v.msg}`,
      });
    }
  }
  if (bindings.length === 0) {
    if (mandatory)
      violations.push({
        code: 'F0',
        file: rel,
        msg: 'обязательный файл не содержит привязок oncontextmenu=/ondblclick=/onkeydown=',
      });
    else
      notes.push(
        `SKIP ${rel}: привязок oncontextmenu=/ondblclick=/onkeydown= нет (строка ещё не подключена)`,
      );
    return { violations, notes };
  }
  if (mandatory) {
    if (required === null) {
      violations.push({
        code: 'F0',
        file: rel,
        msg: 'обязательный файл не внесён в REQUIRED_BINDINGS: не задано, какие привязки обязан иметь',
      });
    } else {
      for (const attr of required) {
        if (!bindings.some((b) => b.attr === attr)) {
          violations.push({
            code: 'F0',
            file: rel,
            msg: `обязательная привязка ${attr}= снята (охрана в теле обработчика осталась бы, а событие перестало бы доходить до него)`,
          });
        }
      }
    }
  }
  const mouse =
    bindings.filter((b) => b.attr !== 'onkeydown').length + tdClickHandlerCount(tdClicks);
  const hasKey = bindings.some((b) => b.attr === 'onkeydown');
  const overlayCount = (clean.match(/\bisRowOverlayEvent\s*\(/g) ?? []).length;
  const ownCount = (clean.match(/\bisOwnKeyEvent\s*\(/g) ?? []).length;
  if (overlayCount < mouse) {
    violations.push({
      code: 'F1',
      file: rel,
      msg: `isRowOverlayEvent( встречается ${overlayCount} раз, привязок oncontextmenu=/ondblclick=/onclick на <td ${mouse}`,
    });
  }
  if (hasKey && ownCount < 1) {
    violations.push({
      code: 'F1',
      file: rel,
      msg: 'есть onkeydown=, но нет ни одного isOwnKeyEvent(',
    });
  }
  for (const b of bindings) {
    const body = resolveHandlerBody(b.expr, script);
    if (body === null) {
      violations.push({
        code: 'F2',
        file: rel,
        msg: `строка ${b.line}: значение ${b.attr}={${b.expr ?? '?'}} не разрешается в тело функции файла`,
      });
      continue;
    }
    for (const v of checkGuardedBody(body, b.attr)) {
      violations.push({ code: v.code, file: rel, msg: `строка ${b.line}: ${v.msg}` });
    }
  }
  return { violations, notes };
}

// --- [A], [B], [C], [G] ------------------------------------------------------

/** Позиции открытий/закрытий {#each} в markup с глубиной. */
function eachBlockSpan(markup, startIdx) {
  const re = /\{#each\b|\{\/each\}/g;
  re.lastIndex = startIdx + 1;
  let depth = 1;
  let m;
  while ((m = re.exec(markup)) !== null) {
    if (m[0] === '{#each' || m[0].startsWith('{#each')) depth++;
    else depth--;
    if (depth === 0) return m.index;
  }
  return -1;
}

export function checkMenuFile(src) {
  const violations = [];
  const clean = stripComments(src);
  const { script, markup } = splitSvelte(clean);
  // [G]
  if (!/\bquickActionsForDevice\s*\(/.test(clean)) {
    violations.push({
      code: 'G',
      msg: 'DeviceRowMenu.svelte не зовёт quickActionsForDevice(: правило type_id = 1 (D-38) не применяется',
    });
  }
  if (/\bquickActionsFromTypes\s*\(/.test(clean)) {
    violations.push({
      code: 'G',
      msg: 'DeviceRowMenu.svelte зовёт quickActionsFromTypes( напрямую: правило type_id = 1 (D-38) обходится',
    });
  }
  // [A]
  const hrs = [...markup.matchAll(/<hr\b[^>]*>/g)];
  if (hrs.length !== 1) {
    violations.push({
      code: 'A',
      msg: `в разметке ${hrs.length} элементов <hr, нужен ровно один (разделитель — граница секций)`,
    });
  } else if (!/role\s*=\s*"separator"/.test(hrs[0][0])) {
    violations.push({ code: 'A', msg: '<hr без role="separator"' });
  }
  const decl =
    /(?:const|let)\s+(\w+)\s*=\s*\$derived(?:\.by)?\([\s\S]{0,200}?\bbuildMenuSections\s*\(/.exec(
      script,
    );
  if (!decl) {
    violations.push({
      code: 'A',
      msg: 'секции не выводятся из buildMenuSections: нет `const <имя> = $derived(…buildMenuSections(…))`',
    });
  } else if (hrs.length === 1) {
    const name = decl[1];
    const eachRe = new RegExp(`\\{#each\\s+${name}\\b`, 'g');
    let inside = false;
    let m;
    while ((m = eachRe.exec(markup)) !== null) {
      const end = eachBlockSpan(markup, m.index);
      const hrIdx = hrs[0].index;
      if (end === -1 || hrIdx < m.index || hrIdx > end) continue;
      // Глубина вложенных {#each} между началом блока и <hr обязана быть нулевой.
      const nested = /\{#each\b|\{\/each\}/g;
      nested.lastIndex = m.index + 1;
      let depth = 0;
      let n;
      while ((n = nested.exec(markup)) !== null && n.index < hrIdx) {
        depth += n[0].startsWith('{#each') ? 1 : -1;
      }
      if (depth === 0) inside = true;
    }
    if (!inside) {
      violations.push({
        code: 'A',
        msg: `<hr не лежит внутри {#each ${name} …} по секциям (или лежит во вложенном цикле по пунктам)`,
      });
    }
  }
  return violations;
}

export function checkModuleFile(src) {
  const violations = [];
  const clean = stripComments(src);
  if (!clean.includes('quick_action_enabled')) {
    violations.push({
      code: 'C',
      msg: 'deviceRowMenu.ts не читает quick_action_enabled: быстрые действия больше не порождаются типами',
    });
  }
  for (const label of BUILTIN_LABELS) {
    if (clean.includes(label))
      violations.push({
        code: 'C',
        msg: `в deviceRowMenu.ts литерал встроенной подписи «${label}»`,
      });
  }
  return violations;
}

export function checkLiterals(files) {
  const violations = [];
  for (const [rel, src] of files) {
    const clean = stripComments(src);
    for (const label of BUILTIN_LABELS) {
      let at = clean.indexOf(label);
      while (at !== -1) {
        violations.push({
          code: 'B',
          file: rel,
          msg: `строка ${lineOf(clean, at)}: литерал встроенной подписи «${label}» (подпись берётся из quick_action_label типа, D-22)`,
        });
        at = clean.indexOf(label, at + label.length);
      }
    }
  }
  return violations;
}

/** [D] Возврат второй реализации меню строки (DeviceContextMenu / .ctx-menu-*). */
export function checkLegacyMenu(files) {
  const violations = [];
  if (files.has(LEGACY_MENU_FILE)) {
    violations.push({
      code: 'D',
      file: LEGACY_MENU_FILE,
      msg: 'файл удалён в плане 41.1-17 (D-21): единственная реализация меню строки — ActionMenu через DeviceRowMenu',
    });
  }
  for (const [rel, src] of files) {
    const clean = stripComments(src);
    const ident = LEGACY_MENU_IDENT.exec(clean);
    if (ident) {
      violations.push({
        code: 'D',
        file: rel,
        msg: `строка ${lineOf(clean, ident.index)}: упоминание DeviceContextMenu в коде (импорт или использование удалённого меню)`,
      });
    }
    if (!rel.startsWith(LEGACY_CLASS_ALLOWED_DIR)) {
      const cls = LEGACY_CLASS.exec(clean);
      if (cls) {
        violations.push({
          code: 'D',
          file: rel,
          msg: `строка ${lineOf(clean, cls.index)}: класс ctx-menu-* допустим только в ${LEGACY_CLASS_ALLOWED_DIR} (вторая реализация меню)`,
        });
      }
    }
  }
  return violations;
}

/** Все структурные правила по карте «относительный путь -> исходник». */
export function scanStructural(files, opts = {}) {
  const mandatory = opts.mandatory ?? MANDATORY_ROW_FILES;
  const requiredBindings = opts.requiredBindings ?? REQUIRED_BINDINGS;
  const rowFiles = opts.rowFiles ?? ROW_FILES;
  const requiredTdRows = opts.requiredTdClickRows ?? REQUIRED_TD_CLICK_ROWS;
  const minScanned = opts.minScanned ?? MIN_SCANNED_FILES;
  const violations = [];
  const notes = [];
  const add = (v) => violations.push(v);

  if (files.size < minScanned) {
    add({
      code: 'E',
      file: 'src',
      msg: `просмотрено ${files.size} файлов, ожидается не меньше ${minScanned}: гейт слеп (изменилась структура каталогов?)`,
    });
  }
  for (const m of mandatory) {
    if (!rowFiles.includes(m))
      add({
        code: 'F0',
        file: m,
        msg: 'MANDATORY_ROW_FILES содержит файл, которого нет в ROW_FILES',
      });
  }

  for (const m of requiredTdRows) {
    if (!mandatory.includes(m))
      add({
        code: 'F0',
        file: m,
        msg: 'REQUIRED_TD_CLICK_ROWS содержит файл, которого нет в MANDATORY_ROW_FILES',
      });
  }

  const menuSrc = files.get(MENU_FILE);
  if (menuSrc === undefined) {
    notes.push(`SKIP [A]/[G] ${MENU_FILE}: файла нет`);
  } else {
    for (const v of checkMenuFile(menuSrc)) add({ ...v, file: MENU_FILE });
  }
  const moduleSrc = files.get(MODULE_FILE);
  if (moduleSrc === undefined) add({ code: 'C', file: MODULE_FILE, msg: 'файл модуля не найден' });
  else for (const v of checkModuleFile(moduleSrc)) add({ ...v, file: MODULE_FILE });
  for (const v of checkLiterals(files)) add(v);
  for (const v of checkLegacyMenu(files)) add(v);

  // [F]: ROW_FILES + каждый .svelte с <DeviceRowMenu (обход, не ручной список).
  const rowSet = new Set(rowFiles);
  for (const [rel, src] of files) {
    if (
      rel.endsWith('.svelte') &&
      rel !== MENU_FILE &&
      /<DeviceRowMenu\b/.test(stripComments(src))
    ) {
      rowSet.add(rel);
    }
  }
  for (const rel of [...rowSet].sort()) {
    const isMandatory = mandatory.includes(rel);
    const src = files.has(rel) ? files.get(rel) : null;
    // Охрана нужна, пока в ячейках строки может оказаться модалка меню. Файл из
    // ROW_FILES, который меню ещё не подключил, проверяется только когда он
    // обязательный; иначе — явный SKIP (его привязки не связаны с DeviceRowMenu).
    if (!isMandatory && src !== null && !/<DeviceRowMenu\b/.test(stripComments(src))) {
      notes.push(`SKIP ${rel}: <DeviceRowMenu не подключён, охрана событий строки не требуется`);
      continue;
    }
    const r = checkRowFile(
      rel,
      src,
      isMandatory,
      requiredBindings[rel] ?? null,
      requiredTdRows.includes(rel),
    );
    for (const v of r.violations) add(v);
    for (const n of r.notes) notes.push(n);
  }
  return { violations, notes };
}

function walkSrc(srcDir, out = []) {
  for (const e of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const p = path.join(srcDir, e.name);
    if (e.isDirectory()) {
      if (e.name === 'node_modules' || e.name === 'dist') continue;
      walkSrc(p, out);
    } else if (e.name.endsWith('.svelte') || e.name.endsWith('.ts')) out.push(p);
  }
  return out;
}

function readTree(root) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) return null;
  const files = new Map();
  for (const abs of walkSrc(srcDir)) {
    files.set(path.relative(root, abs).split(path.sep).join('/'), fs.readFileSync(abs, 'utf8'));
  }
  return files;
}

function formatViolation(v) {
  return `[${v.code}] ${v.file ? `${v.file}: ` : ''}${v.msg}`;
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

/** Замена с проверкой, что якорь встречается ровно один раз. */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

const SOURCE_MUTANTS = [
  { name: 'не фильтровать пустые секции', from: '.filter((s) => s.length > 0)', to: '' },
  {
    name: 'порядок: «Удалить» раньше групповой секции',
    from: '[poll, main, group, del]',
    to: '[poll, main, del, group]',
  },
  {
    name: 'порядок: опрос после просмотра',
    from: '[poll, main, group, del]',
    to: '[main, poll, group, del]',
  },
  {
    name: 'групповые пункты без groupActionsVisible',
    from: 'input.groupActionsVisible',
    to: 'true',
  },
  {
    name: 'не учитывать isAnchor',
    from: 'input.isMember || input.isAnchor || input.isNetworkPrinter',
    to: 'input.isMember || input.isNetworkPrinter',
  },
  {
    name: 'не учитывать isMember',
    from: 'input.isMember || input.isAnchor || input.isNetworkPrinter',
    to: 'input.isAnchor || input.isNetworkPrinter',
  },
  {
    name: 'не учитывать isNetworkPrinter',
    from: 'input.isMember || input.isAnchor || input.isNetworkPrinter',
    to: 'input.isMember || input.isAnchor',
  },
  { name: 'не блокировать на_списание', from: "['списано', 'на_списание']", to: "['списано']" },
  { name: 'не блокировать списано', from: "['списано', 'на_списание']", to: "['на_списание']" },
  {
    name: 'не фильтровать выключенные быстрые действия',
    from: 't.quick_action_enabled === true',
    to: 'true',
  },
  { name: 'не обрезать подпись', from: '.trim()', to: '' },
  { name: 'не отбрасывать пустую подпись', from: "x.label !== ''", to: 'true' },
  { name: 'убрать сортировку по sort_order', from: 'a.t.sort_order - b.t.sort_order || ', to: '' },
  { name: 'убрать tie-break по id', from: ' || a.t.id - b.t.id', to: '' },
  {
    name: 'убрать проверку deviceTypeId',
    from: 'deviceTypeId !== ANCHOR_DEVICE_TYPE_ID',
    to: 'false',
  },
  {
    name: 'ANCHOR_DEVICE_TYPE_ID = 2',
    from: 'export const ANCHOR_DEVICE_TYPE_ID = 1;',
    to: 'export const ANCHOR_DEVICE_TYPE_ID = 2;',
  },
  { name: 'селектор без [role="dialog"]', from: '[role="dialog"], ', to: '' },
  { name: 'селектор без .modal-backdrop', from: '.modal-backdrop, ', to: '' },
  { name: 'селектор без [role="menu"]', from: ', [role="menu"]', to: '' },
  {
    name: 'isRowOverlayEvent без защиты от null',
    from: "if (typeof target !== 'object' || target === null) return false;",
    to: '',
  },
  {
    name: 'isRowOverlayEvent без проверки closest',
    from: "if (typeof closest !== 'function') return false;",
    to: '',
  },
  {
    name: 'isRowOverlayEvent всегда true при наличии closest',
    from: 'closest.call(target, ROW_OVERLAY_SELECTOR) != null',
    to: 'true',
  },
  {
    name: 'isOwnKeyEvent всегда true',
    from: 'return target === currentTarget;',
    to: 'return true;',
  },
  {
    name: 'isOwnKeyEvent без защиты от пустой цели',
    from: 'if (target === null || target === undefined) return false;',
    to: '',
  },
];

const CLEAN_MENU = `<script lang="ts">
  // раньше здесь было quickActionsFromTypes(types) — удалено
  import { buildMenuSections, quickActionsForDevice } from './deviceRowMenu';
  const sections = $derived(
    buildMenuSections({
      hasPoll: false,
      hasPrint: true,
      groupActionsVisible: true,
      quickActions: quickActionsForDevice(types, device.type_id),
    }),
  );
</script>
<!-- <hr role="separator" /> в комментарии не считается -->
<ActionMenu>
  {#each sections as section, si (si)}
    {#if si > 0}
      <hr role="separator" />
    {/if}
    {#each section as item (item.kind + item.label)}
      <button role="menuitem">{item.label}</button>
    {/each}
  {/each}
</ActionMenu>
`;

const CLEAN_MODULE = `export function x(t) {
  return t.filter((q) => q.quick_action_enabled === true);
}
`;

const CLEAN_ROW = `<script lang="ts">
  import { isRowOverlayEvent, isOwnKeyEvent } from './deviceRowMenu';
  function handleContextMenu(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    e.preventDefault();
    menuRef?.openAt(e.clientX, e.clientY);
  }
  const handleDblClick = (e: MouseEvent) => {
    if (isRowOverlayEvent(e.target) || (e.target as HTMLElement).closest('button, a')) return;
    window.getSelection()?.removeAllRanges();
    menuRef?.openView();
  };
  function handleClick(e: MouseEvent) {
    if (isRowOverlayEvent(e.target)) return;
    onclick();
  }
  function handleCellKeydown(e: KeyboardEvent) {
    if (!isOwnKeyEvent(e.target, e.currentTarget)) return;
    if (e.key === 'ContextMenu') {
      e.preventDefault();
      menuRef?.openFromElement(e.currentTarget as HTMLElement);
    }
  }
</script>
<TableRow oncontextmenu={handleContextMenu} ondblclick={handleDblClick}>
  <td onkeydown={handleCellKeydown}><DeviceRowMenu bind:this={menuRef} {device} /></td>
  <td class="cell" onclick={handleClick}>{label}</td>
  <td class="cell" title={a > b ? 'x' : 'y'}>без onclick: > внутри выражения не обрезает тег</td>
</TableRow>
`;

const ROW_PATH = 'src/features/devices/DeviceListRow.svelte';

function fixtureMap(extra = {}) {
  const m = new Map([
    [MENU_FILE, CLEAN_MENU],
    [MODULE_FILE, CLEAN_MODULE],
    [ROW_PATH, CLEAN_ROW],
  ]);
  for (const [k, v] of Object.entries(extra)) {
    if (v === null) m.delete(k);
    else m.set(k, v);
  }
  return m;
}

const SELFTEST_OPTS = {
  minScanned: SELFTEST_MIN_SCANNED_FILES,
  mandatory: [],
  rowFiles: [],
  requiredTdClickRows: [],
};

/** Структурные мутанты: у каждого «до» зелёная, «после» красная с кодами expectCodes. */
function structuralMutants() {
  const row = (from, to, label) => mutate(CLEAN_ROW, from, to, label);
  const menu = (from, to, label) => mutate(CLEAN_MENU, from, to, label);
  return [
    {
      name: '[A] второй <hr у DeviceRowMenu',
      files: fixtureMap({
        [MENU_FILE]: menu('</ActionMenu>', '<hr role="separator" /></ActionMenu>', 'A2'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] <hr без role="separator"',
      files: fixtureMap({
        [MENU_FILE]: menu('<hr role="separator" />\n    {/if}', '<hr />\n    {/if}', 'Arole'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] <hr вынесен за цикл по секциям',
      files: fixtureMap({
        [MENU_FILE]: menu(
          '    {#if si > 0}\n      <hr role="separator" />\n    {/if}\n',
          '',
          'Aout',
        ).replace('</ActionMenu>', '<hr role="separator" />\n</ActionMenu>'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] <hr во вложенном цикле по пунктам',
      files: fixtureMap({
        [MENU_FILE]: menu(
          '    {#if si > 0}\n      <hr role="separator" />\n    {/if}\n',
          '',
          'Ain',
        ).replace('<button role="menuitem">', '<hr role="separator" /><button role="menuitem">'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] секции не из buildMenuSections',
      files: fixtureMap({ [MENU_FILE]: menu('buildMenuSections({', 'computeSections({', 'Asrc') }),
      expect: ['A'],
    },
    {
      name: '[G] прямой вызов quickActionsFromTypes(',
      files: fixtureMap({
        [MENU_FILE]: menu(
          'quickActions: quickActionsForDevice(types, device.type_id)',
          'quickActions: quickActionsFromTypes(types)',
          'G',
        ),
      }),
      expect: ['G'],
    },
    {
      // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
      // Блочно-первая вычистка стирала бы прямой вызов между `templates/*.html` и
      // ближайшим `*/` ниже. quickActionsForDevice( остаётся на месте, чтобы красным
      // был именно прямой вызов, а не отсутствие обязательного.
      name: '[G] `/*` в строчном комментарии не прячет прямой вызов quickActionsFromTypes(',
      files: fixtureMap({
        [MENU_FILE]: menu(
          'quickActions: quickActionsForDevice(types, device.type_id)',
          'quickActions: quickActionsForDevice(types, device.type_id),\n' +
            '      // шаблоны лежат в templates/*.html\n' +
            '      extra: quickActionsFromTypes(types)\n' +
            '      /* настоящий блочный комментарий */',
          'G-comment-trap',
        ),
      }),
      expect: ['G'],
    },
    {
      name: '[B] литерал встроенной подписи в другом файле',
      files: fixtureMap({ 'src/features/foo/Foo.svelte': '<button>Замещение группой</button>' }),
      expect: ['B'],
    },
    {
      name: '[C] модуль не читает quick_action_enabled',
      files: fixtureMap({ [MODULE_FILE]: 'export function x() { return []; }\n' }),
      expect: ['C'],
    },
    {
      name: '[C] в модуле литерал подписи «На разбор»',
      files: fixtureMap({ [MODULE_FILE]: `${CLEAN_MODULE}export const L = 'На разбор';\n` }),
      expect: ['C', 'B'],
    },
    {
      name: '[F1] oncontextmenu= без isRowOverlayEvent(',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          '    e.preventDefault();',
          'F1a',
        ),
      }),
      expect: ['F1'],
    },
    {
      name: '[F2/F3] охрана ПОСЛЕ preventDefault, счёт прежний',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          '    e.preventDefault();\n    if (isRowOverlayEvent(e.target)) return;',
          'F2/F3',
        ),
      }),
      expect: ['F2', 'F3'],
      absent: ['F1'],
    },
    {
      name: '[F2] охрана не первым оператором (перед ней const)',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          '    const x = e.clientX;\n    if (isRowOverlayEvent(e.target)) return;\n    menuRef?.touch(x);',
          'F2first',
        ),
      }),
      expect: ['F2'],
      absent: ['F1'],
    },
    {
      name: '[F2] isRowOverlayEvent( есть в другой функции файла, в обработчике нет, счёт сходится',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          '    e.preventDefault();',
          'F2other',
        ).replace(
          '</script>',
          'function other(e: MouseEvent) {\n    if (isRowOverlayEvent(e.target)) return;\n  }\n</script>',
        ),
      }),
      expect: ['F2'],
      absent: ['F1'],
    },
    {
      name: '[F1] onkeydown= без isOwnKeyEvent(',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (!isOwnKeyEvent(e.target, e.currentTarget)) return;',
          'if (e.defaultPrevented) return;',
          'F1key',
        ),
      }),
      expect: ['F1'],
    },
    {
      name: '[F2] значение привязки не разрешается в тело функции',
      files: fixtureMap({
        [ROW_PATH]: row(
          'oncontextmenu={handleContextMenu}',
          'oncontextmenu={handlerFromElsewhere}',
          'F2unres',
        ),
      }),
      expect: ['F2'],
    },
    {
      name: '[F2] охрана с обратной полярностью у contextmenu',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          'if (!isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          'F2pol',
        ),
      }),
      expect: ['F2'],
    },
    {
      name: '[F2] охрана keydown без отрицания',
      files: fixtureMap({
        [ROW_PATH]: row('if (!isOwnKeyEvent(', 'if (isOwnKeyEvent(', 'F2polkey'),
      }),
      expect: ['F2'],
    },
    {
      name: '[F2] охрана вторым операндом условия',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          'if (e.defaultPrevented || isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          'F2second',
        ),
      }),
      expect: ['F2'],
    },
    {
      name: '[F2] инлайн-стрелка без охраны в теле',
      files: fixtureMap({
        [ROW_PATH]: row(
          'ondblclick={handleDblClick}',
          'ondblclick={(e) => menuRef?.openView()}',
          'F2inline',
        ),
      }),
      expect: ['F2'],
    },
    {
      name: '[F2] охрана без выхода return',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (isRowOverlayEvent(e.target)) return;\n    e.preventDefault();',
          'if (isRowOverlayEvent(e.target)) log(e);\n    e.preventDefault();',
          'F2ret',
        ),
      }),
      expect: ['F2'],
    },
    {
      name: '[D] файл DeviceContextMenu.svelte вернулся',
      files: fixtureMap({ [LEGACY_MENU_FILE]: '<div></div>\n' }),
      expect: ['D'],
    },
    {
      name: '[D] строка вернула импорт DeviceContextMenu',
      files: fixtureMap({
        [ROW_PATH]: row(
          "  import { isRowOverlayEvent, isOwnKeyEvent } from './deviceRowMenu';",
          "  import DeviceContextMenu from './DeviceContextMenu.svelte';\n  import { isRowOverlayEvent, isOwnKeyEvent } from './deviceRowMenu';",
          'Dimport',
        ),
      }),
      expect: ['D'],
      absent: ['F1', 'F2', 'F3'],
    },
    {
      name: '[D] класс ctx-menu-item в DeviceRowMenu',
      files: fixtureMap({
        [MENU_FILE]: menu(
          '<button role="menuitem">',
          '<button role="menuitem" class="ctx-menu-item">',
          'Dclass',
        ),
      }),
      expect: ['D'],
    },
    {
      name: '[D] класс ctx-menu-item в картриджах допустим (не мутант, контроль)',
      files: fixtureMap({
        'src/features/cartridges/CartridgeContextMenu.svelte':
          '<button class="ctx-menu-item">x</button>\n',
      }),
      expect: [],
      absent: ['D'],
    },
    {
      name: '[F4] охрана click удалена (счёт прежний за счёт соседней функции)',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    onclick();',
          '    onclick();',
          'F4del',
        ).replace(
          '</script>',
          'function other(e: MouseEvent) {\n    if (isRowOverlayEvent(e.target)) return;\n  }\n</script>',
        ),
      }),
      expect: ['F4', 'F3'],
      absent: ['F1'],
    },
    {
      name: '[F4] охрана click с обратной полярностью',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (isRowOverlayEvent(e.target)) return;\n    onclick();',
          'if (!isRowOverlayEvent(e.target)) return;\n    onclick();',
          'F4pol',
        ),
      }),
      expect: ['F4'],
      absent: ['F1', 'F3'],
    },
    {
      name: '[F4] МУТАНТ-ПЕРЕСТАНОВКА: onclick() раньше охраны, счёт прежний',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    onclick();',
          '    onclick();\n    if (isRowOverlayEvent(e.target)) return;',
          'F4swap',
        ),
      }),
      expect: ['F4', 'F3'],
      absent: ['F1'],
    },
    {
      name: '[F4] охрана click не первым оператором (перед ней const)',
      files: fixtureMap({
        [ROW_PATH]: row(
          '    if (isRowOverlayEvent(e.target)) return;\n    onclick();',
          '    const x = e.detail;\n    if (isRowOverlayEvent(e.target)) return;\n    log(x);',
          'F4first',
        ),
      }),
      expect: ['F4'],
      absent: ['F1', 'F3'],
    },
    {
      name: '[F4] охрана click без выхода return',
      files: fixtureMap({
        [ROW_PATH]: row(
          'if (isRowOverlayEvent(e.target)) return;\n    onclick();',
          'if (isRowOverlayEvent(e.target)) log(e);\n    onclick();',
          'F4ret',
        ),
      }),
      expect: ['F4'],
    },
    {
      name: '[F4] вторая ячейка с неохраняемым обработчиком, счёт сходится',
      files: fixtureMap({
        [ROW_PATH]: row(
          '<td class="cell" title={a > b',
          '<td class="cell" onclick={handleOther}>y</td>\n  <td class="cell" title={a > b',
          'F4second',
        ).replace(
          '</script>',
          'function handleOther(e: MouseEvent) {\n    onclick();\n  }\n  function spare(e: MouseEvent) {\n    if (isRowOverlayEvent(e.target)) return;\n  }\n</script>',
        ),
      }),
      expect: ['F4'],
      absent: ['F1'],
    },
    {
      name: '[F4] инлайн-стрелка без охраны вместо handleClick',
      files: fixtureMap({
        [ROW_PATH]: row('onclick={handleClick}', 'onclick={() => onclick()}', 'F4inline'),
      }),
      expect: ['F4'],
    },
    {
      name: '[F4] обработчик не разрешается в тело функции файла',
      files: fixtureMap({
        [ROW_PATH]: row('onclick={handleClick}', 'onclick={handlerFromElsewhere}', 'F4unres'),
      }),
      expect: ['F4'],
    },
    {
      name: '[F] новый файл, подключивший меню, ловится обходом (нет в ROW_FILES)',
      files: fixtureMap({
        'src/features/foo/NewRow.svelte':
          '<script lang="ts">\n  function h(e: MouseEvent) {\n    e.preventDefault();\n  }\n</script>\n<tr oncontextmenu={h}><DeviceRowMenu {device} /></tr>\n',
      }),
      expect: ['F1', 'F2'],
    },
  ];
}

async function selftest(cases) {
  let tsSource;
  try {
    tsSource = fs.readFileSync(path.join(UI_ROOT, MODULE_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${MODULE_FILE}`);
    process.exit(1);
  }
  // 1. Эталон проходит фикстуру.
  const goodFailures = runCases(await loadModuleFromTs(tsSource, 'good.ts'), cases);
  if (goodFailures.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальный исходник не прошёл фикстуру:\n  - ${goodFailures.join('\n  - ')}`,
    );
    process.exit(1);
  }
  // 2. Мутанты исходника ловятся.
  for (const m of SOURCE_MUTANTS) {
    const mutated = mutate(tsSource, m.from, m.to, m.name);
    const failures = runCases(await loadModuleFromTs(mutated, 'mutant.ts'), cases);
    if (!failures.length) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» прошёл фикстуру: гейт его не ловит`);
      process.exit(1);
    }
  }
  // 3. Структурные правила: чистая карта зелёная, мутанты красные нужным кодом.
  const clean = scanStructural(fixtureMap(), SELFTEST_OPTS);
  if (clean.violations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — чистая фикстура структурных правил красная:\n  - ${clean.violations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  const mutants = structuralMutants();
  for (const m of mutants) {
    const r = scanStructural(m.files, SELFTEST_OPTS);
    const codes = new Set(r.violations.map((v) => v.code));
    for (const code of m.expect) {
      if (!codes.has(code)) {
        console.error(
          `${TAG} SELFTEST FAIL — «${m.name}»: ожидалось нарушение [${code}], получено [${[...codes].join(',') || 'ничего'}]`,
        );
        process.exit(1);
      }
    }
    for (const code of m.absent ?? []) {
      if (codes.has(code)) {
        console.error(
          `${TAG} SELFTEST FAIL — «${m.name}»: [${code}] не должно срабатывать (мутант обязан проходить счёт и падать на порядке)`,
        );
        process.exit(1);
      }
    }
  }
  // 4. Допустимые формы охраны остаются зелёными (охрана с дополнительным операндом).
  // CLEAN_ROW уже содержит handleDblClick с `|| closest(...)` — он прошёл выше.

  // 5. Обязательный файл: нет файла / нет привязок — нарушение, необязательный — явный SKIP.
  const ALL_BINDINGS = ['oncontextmenu', 'ondblclick', 'onkeydown'];
  const mandatoryOpts = {
    ...SELFTEST_OPTS,
    mandatory: [ROW_PATH],
    rowFiles: [ROW_PATH],
    requiredBindings: { [ROW_PATH]: ALL_BINDINGS },
  };
  const cleanMandatory = scanStructural(fixtureMap(), mandatoryOpts);
  if (cleanMandatory.violations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — чистая обязательная строка красная:\n  - ${cleanMandatory.violations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  // 5a. Слепое пятно плана 41.1-18: снятая привязка при живой охране. Без
  // REQUIRED_BINDINGS счёт F1 остаётся зелёным (охран не меньше привязок), F0 не
  // срабатывает (привязки есть). Мутант обязан краснеть именно [F0] и ни одним
  // из F1/F2/F3 — иначе доказано не то.
  const unbind = [
    ['ondblclick', ' ondblclick={handleDblClick}', 'ondblclick'],
    ['oncontextmenu', ' oncontextmenu={handleContextMenu}', 'oncontextmenu'],
    ['onkeydown', ' onkeydown={handleCellKeydown}', 'onkeydown'],
  ];
  for (const [attr, from, label] of unbind) {
    const mutated = mutate(CLEAN_ROW, from, '', `снята привязка ${label}`);
    const r = scanStructural(fixtureMap({ [ROW_PATH]: mutated }), mandatoryOpts);
    const codes = new Set(r.violations.map((v) => v.code));
    if (!codes.has('F0') || !r.violations.some((v) => v.msg.includes(`${attr}=`))) {
      console.error(
        `${TAG} SELFTEST FAIL — снятая привязка ${attr}= у обязательной строки не краснит [F0]`,
      );
      process.exit(1);
    }
    if (codes.has('F1') || codes.has('F2') || codes.has('F3')) {
      console.error(
        `${TAG} SELFTEST FAIL — мутант «снята ${attr}=» должен краснеть ТОЛЬКО [F0] (охрана-то на месте); иначе слепое пятно не доказано`,
      );
      process.exit(1);
    }
    // Тот же мутант на необязательной строке слеп по построению: это и есть дыра.
    const optionalOpts = { ...SELFTEST_OPTS, rowFiles: [ROW_PATH] };
    if (scanStructural(fixtureMap({ [ROW_PATH]: mutated }), optionalOpts).violations.length) {
      console.error(
        `${TAG} SELFTEST FAIL — контроль: без REQUIRED_BINDINGS мутант «снята ${attr}=» не должен краснеть (иначе F1 уже ловил его и правило лишнее)`,
      );
      process.exit(1);
    }
  }
  // 5a2. [F4]/[F0]: снятая привязка onclick на <td> у файла из REQUIRED_TD_CLICK_ROWS
  // краснит ТОЛЬКО [F0] (охрана-то на месте); без записи в REQUIRED_TD_CLICK_ROWS
  // тот же мутант слеп по построению.
  const tdOpts = { ...mandatoryOpts, requiredTdClickRows: [ROW_PATH] };
  const cleanTd = scanStructural(fixtureMap(), tdOpts);
  if (cleanTd.violations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — чистая строка с обязательным td-click красная:\n  - ${cleanTd.violations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  const noTdBind = mutate(CLEAN_ROW, ' onclick={handleClick}', '', 'снята привязка onclick на td');
  const noTdBindRes = scanStructural(fixtureMap({ [ROW_PATH]: noTdBind }), tdOpts);
  const noTdCodes = new Set(noTdBindRes.violations.map((v) => v.code));
  if (!noTdCodes.has('F0') || !noTdBindRes.violations.some((v) => v.msg.includes('onclick='))) {
    console.error(`${TAG} SELFTEST FAIL — снятая привязка onclick= на <td не краснит [F0]`);
    process.exit(1);
  }
  if (noTdCodes.has('F1') || noTdCodes.has('F2') || noTdCodes.has('F3') || noTdCodes.has('F4')) {
    console.error(
      `${TAG} SELFTEST FAIL — мутант «снята onclick= на td» должен краснеть ТОЛЬКО [F0]`,
    );
    process.exit(1);
  }
  if (scanStructural(fixtureMap({ [ROW_PATH]: noTdBind }), mandatoryOpts).violations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — контроль: без REQUIRED_TD_CLICK_ROWS снятая onclick= не должна краснеть`,
    );
    process.exit(1);
  }
  const badTdConfig = scanStructural(fixtureMap(), {
    ...SELFTEST_OPTS,
    requiredTdClickRows: ['src/x.svelte'],
  });
  if (!badTdConfig.violations.some((v) => v.code === 'F0')) {
    console.error(
      `${TAG} SELFTEST FAIL — REQUIRED_TD_CLICK_ROWS вне MANDATORY_ROW_FILES не краснит`,
    );
    process.exit(1);
  }
  for (const f of REQUIRED_TD_CLICK_ROWS) {
    if (!MANDATORY_ROW_FILES.includes(f)) {
      console.error(`${TAG} SELFTEST FAIL — ${f} из REQUIRED_TD_CLICK_ROWS не обязательный файл`);
      process.exit(1);
    }
  }
  // 5b. Самоинвалидация: обязательный файл без записи в REQUIRED_BINDINGS краснит [F0].
  const noEntry = scanStructural(fixtureMap(), { ...mandatoryOpts, requiredBindings: {} });
  if (!noEntry.violations.some((v) => v.code === 'F0' && v.msg.includes('REQUIRED_BINDINGS'))) {
    console.error(
      `${TAG} SELFTEST FAIL — обязательный файл без записи REQUIRED_BINDINGS не краснит`,
    );
    process.exit(1);
  }
  // 5c. Реальная конфигурация согласована: у каждого обязательного файла есть запись.
  for (const f of MANDATORY_ROW_FILES) {
    if (!Array.isArray(REQUIRED_BINDINGS[f]) || REQUIRED_BINDINGS[f].length === 0) {
      console.error(`${TAG} SELFTEST FAIL — для обязательного ${f} нет записи в REQUIRED_BINDINGS`);
      process.exit(1);
    }
  }
  const noFile = scanStructural(fixtureMap({ [ROW_PATH]: null }), mandatoryOpts);
  if (!noFile.violations.some((v) => v.code === 'F0')) {
    console.error(`${TAG} SELFTEST FAIL — отсутствие обязательного файла строки не краснит`);
    process.exit(1);
  }
  const noBind = scanStructural(fixtureMap({ [ROW_PATH]: '<tr><td>x</td></tr>\n' }), mandatoryOpts);
  if (!noBind.violations.some((v) => v.code === 'F0')) {
    console.error(`${TAG} SELFTEST FAIL — обязательный файл без привязок не краснит`);
    process.exit(1);
  }
  const optional = scanStructural(fixtureMap({ [ROW_PATH]: '<tr><td>x</td></tr>\n' }), {
    ...SELFTEST_OPTS,
    rowFiles: [ROW_PATH],
  });
  if (
    optional.violations.length ||
    !optional.notes.some((n) => n.startsWith('SKIP') && n.includes(ROW_PATH))
  ) {
    console.error(
      `${TAG} SELFTEST FAIL — файл без привязок обязан давать явный SKIP, а не молчать и не краснеть`,
    );
    process.exit(1);
  }
  const unconnectedBad =
    '<tr oncontextmenu={h}><td>x</td></tr>\n<script>function h(e){e.preventDefault();}</script>\n';
  const rowOpts = { ...SELFTEST_OPTS, rowFiles: [ROW_PATH] };
  const skipUnconnected = scanStructural(fixtureMap({ [ROW_PATH]: unconnectedBad }), rowOpts);
  if (
    skipUnconnected.violations.length ||
    !skipUnconnected.notes.some((n) => n.startsWith('SKIP') && n.includes('не подключён'))
  ) {
    console.error(
      `${TAG} SELFTEST FAIL — строка без DeviceRowMenu обязана давать явный SKIP, а не краснеть`,
    );
    process.exit(1);
  }
  const failMandatoryUnconnected = scanStructural(
    fixtureMap({ [ROW_PATH]: unconnectedBad }),
    mandatoryOpts,
  );
  if (!failMandatoryUnconnected.violations.some((v) => v.code === 'F1')) {
    console.error(
      `${TAG} SELFTEST FAIL — обязательная строка без охраны обязана краснеть даже без <DeviceRowMenu`,
    );
    process.exit(1);
  }
  const connectedNoBindings = scanStructural(
    fixtureMap({ [ROW_PATH]: '<tr><td><DeviceRowMenu {device} /></td></tr>\n' }),
    rowOpts,
  );
  if (
    connectedNoBindings.violations.length ||
    !connectedNoBindings.notes.some((n) => n.includes('привязок'))
  ) {
    console.error(
      `${TAG} SELFTEST FAIL — подключённая строка без привязок обязана давать явный SKIP про привязки`,
    );
    process.exit(1);
  }
  const badConfig = scanStructural(fixtureMap(), {
    ...SELFTEST_OPTS,
    mandatory: ['src/x.svelte'],
    rowFiles: [],
  });
  if (!badConfig.violations.some((v) => v.code === 'F0')) {
    console.error(`${TAG} SELFTEST FAIL — MANDATORY_ROW_FILES вне ROW_FILES не краснит`);
    process.exit(1);
  }
  // 6. Защита от вакуумности: слишком мало файлов — [E].
  const vac = scanStructural(fixtureMap(), { ...SELFTEST_OPTS, minScanned: 1000 });
  if (!vac.violations.some((v) => v.code === 'E')) {
    console.error(`${TAG} SELFTEST FAIL — малое число просмотренных файлов не краснит [E]`);
    process.exit(1);
  }
  console.log(
    `${TAG} selftest OK — эталон проходит ${cases.length} кейсов, ${SOURCE_MUTANTS.length} мутантов исходника и ${mutants.length} структурных мутантов ловятся; файл строки без привязок даёт явный SKIP`,
  );
}

// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { impl: path.join(UI_ROOT, MODULE_FILE), root: UI_ROOT, selftest: false };
  for (const a of argv) {
    if (a === '--selftest') args.selftest = true;
    else if (a.startsWith('--impl=')) args.impl = path.resolve(process.cwd(), a.slice(7));
    else if (a.startsWith('--src=')) {
      args.root = path.resolve(process.cwd(), a.slice(6));
      args.impl = path.join(args.root, MODULE_FILE);
    }
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cases = loadFixture();
  if (args.selftest) return selftest(cases);

  let tsSource;
  try {
    tsSource = fs.readFileSync(args.impl, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${args.impl}`);
    process.exit(1);
  }
  const failures = runCases(await loadModuleFromTs(tsSource, path.basename(args.impl)), cases);
  const files = readTree(args.root);
  if (files === null) {
    console.error(`${TAG} FAIL — в ${args.root} нет каталога src/ (--src принимает КОРЕНЬ копии)`);
    process.exit(1);
  }
  const { violations, notes } = scanStructural(files);
  if (failures.length || violations.length) {
    console.error(`${TAG} FAIL —`);
    for (const f of failures) console.error(`  - [golden] ${f}`);
    for (const v of violations) console.error(`  - ${formatViolation(v)}`);
    process.exit(1);
  }
  for (const n of notes) console.log(`${TAG} ${n}`);
  console.log(
    `${TAG} OK — ${cases.length} кейсов фикстуры, структурные правила по ${files.size} файлам`,
  );
}

main();
