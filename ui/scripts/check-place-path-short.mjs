#!/usr/bin/env node
// [check-place-path-short] Постоянный гейт против регрессий колонки «Место»
// (Phase 39.1, PLC-08, D-21; Phase 41.12, INT-07, PLC-07).
//
// Почему он существует: контракт колонки состоит из ДВУХ половин, и сломать
// можно каждую по отдельности, ничего не уронив при сборке —
//   в ТЕКСТЕ ячейки  — сокращённый путь (`place_path_short`, приходит с бэкенда);
//   в АТРИБУТЕ title — полный путь (`full_path` / `place_path`), чтобы полное
//                      расположение было в одном наведении курсора.
// Одна половина уже отваливалась: сгруппированный список устройств
// (`DeviceGroupRow`) молча продолжал показывать полный путь и расходился с
// плоским видом — WR-04 из 39.1-REVIEW.md, уехало в сборку, найдено ревьюером,
// починено руками в коммите 8fa995e5. Ни один автотест этого не держал:
// `pnpm build` доказывает, что компонент компилируется, но не то, КАКОЕ поле он
// читает, а Rust-тесты кончаются на границе DTO.
//
// ПОЧЕМУ СКАН, А НЕ СПИСОК (INT-07). Первая версия гейта держала фиксированный
// перечень `ROW_COMPONENTS` (4 файла + ReportTable). Пять потребителей пути
// (GroupContentsTable, GroupAddDevicesModal, MovementTimeline, DeviceRowMenu и
// др.) были написаны верно, но ничем не удерживались — ровно тот класс пропуска,
// ради которого гейт создан, а новый компонент с сырым полным путём гейт не
// увидел бы вовсе. Теперь потребители ПЕРЕЧИСЛЯЮТСЯ обходом `src/` (все `.svelte`
// и `.ts`, кроме сгенерированных `bindings*.ts` и `*.d.ts` — их содержимое
// задаёт бэкенд-схема, а не потребитель), а освобождения закодированы таблицей
// `VERDICTS` с причиной.
//
// Гейт СТРУКТУРНЫЙ (как check-print-isolation.mjs): он читает исходники и
// проверяет наличие инвариантов, а НЕ правильность рендера — вёрстка и hover
// проверяются руками. Его единственная задача — громко падать, когда
// выстраданный фикс молча убрали при рефакторинге либо появился новый
// потребитель пути, о котором гейт не знает.
//
// «Семейство пути»: SHORT_RE — `place_path_short`, `from_/to_place_path_short`;
// FULL_RE — `full_path`, `place_path`, `from_/to_place_path`.
//
// Правила (код — в сообщениях):
//   P1 полнота      — файл под `src/`, где (без комментариев) есть идентификатор
//                     семейства, обязан попасть под вердикт (файл или каталог с `/`);
//   P2 контракт вида — вид вердикта задаёт, что именно держать:
//       ROW_CELL       `<td>` с SHORT в теле: title={...} с любым именем FULL,
//                      в теле нет FULL (прежние INV-1..INV-3);
//       LINK_BUTTON    `<button>`/`<a>` с SHORT в теле, не меньше `minSites`
//                      штук, у каждой title={...} с FULL (откат на FULL в тексте
//                      допустим: `*_short ?? *_full`);
//       REPORT_TABLE   `formatCellDisplay` читает SHORT, `formatCellTitle` — нет
//                      (прежний INV-4);
//       PASSTHROUGH    в `<script>` одновременно ключи `full_path:` и
//                      `place_path_short:` (объект, который отдаётся дальше);
//       SLOT_TEXT      сокращённый путь читается в `<script>` и уходит в слот
//                      выпадающего списка (Dropdown не даёт повесить title): в
//                      файле нет полного пути, а в разметке нет и SHORT;
//   P3 устаревание  — вердикт без файла; FULL_BY_DESIGN / DATA_ONLY / PASSTHROUGH
//                     / DEMO без идентификатора семейства; SHORT в разметке
//                     FULL_BY_DESIGN / DATA_ONLY (вердикт неверен); дубль пути;
//   P4 качество     — `reason` короче 20 символов; путь не файл `.svelte`/`.ts` и
//                     без `/` на конце; неизвестный `kind`; потребительский вид на
//                     каталоге или `.ts`;
//   P5 антивакуум   — потребителей SHORT не меньше MIN_SHORT_CONSUMERS и
//                     вердиктов не меньше MIN_VERDICTS (иначе «0 нарушений»
//                     могло бы значить «скан ничего не увидел»).
//
// ОГРАНИЧЕНИЕ ПОДХОДА. Присваивание в `<script>` с последующим показом
// переменной (`displayText` в PlacePicker) невидимо разметочному скану. Такие
// файлы удерживаются ТОЛЬКО вердиктом и проверкой «идентификатор семейства
// присутствует» (P3), а не проверкой того, что именно показано.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-place-path-short.mjs              # проверить репозиторий
//   node scripts/check-place-path-short.mjs --src=<dir>  # проверить копию (корень ui)
//   node scripts/check-place-path-short.mjs --report     # инвентарь семейства, exit 0
//   node scripts/check-place-path-short.mjs --selftest   # мутанты по реальным файлам

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-place-path-short]';

const SRC_DIR = 'src';
const SHORT_RE = /\b(?:from_|to_)?place_path_short\b/;
const FULL_RE = /\b(?:from_|to_)?(?:full_path|place_path)\b/;
const SHORT_RE_G = new RegExp(SHORT_RE.source, 'g');
const FULL_RE_G = new RegExp(FULL_RE.source, 'g');
const MIN_REASON = 20;

/** Пороги антивакуума: равны фактическим числам первого прогона по репозиторию. */
const MIN_SHORT_CONSUMERS = 9;
const MIN_VERDICTS = 23;

/** Виды вердикта, требующие `reason` (освобождения). */
const EXEMPT_KINDS = new Set(['PASSTHROUGH', 'FULL_BY_DESIGN', 'DATA_ONLY', 'DEMO', 'SLOT_TEXT']);
/** Виды-потребители SHORT: считаются в MIN_SHORT_CONSUMERS и применимы только к `.svelte`-файлу. */
const CONSUMER_KINDS = new Set(['ROW_CELL', 'LINK_BUTTON', 'REPORT_TABLE', 'PASSTHROUGH']);
const ALL_KINDS = new Set([...EXEMPT_KINDS, ...CONSUMER_KINDS]);

/**
 * Таблица вердиктов: КАЖДЫЙ файл под `src/`, где встречается идентификатор
 * семейства пути, обязан попасть сюда (P1). `path` — файл либо каталог с `/` на
 * конце (иначе `features/requests` освободил бы `features/requests_old`).
 */
const VERDICTS = [
  // --- ROW_CELL: колонка «Место» в таблице; текст — short, title — полный путь ---
  { path: 'src/features/devices/DeviceListRow.svelte', kind: 'ROW_CELL' },
  { path: 'src/features/devices/DeviceGroupRow.svelte', kind: 'ROW_CELL' },
  { path: 'src/features/cartridges/CartridgeListRow.svelte', kind: 'ROW_CELL' },
  { path: 'src/features/places/PlaceContents.svelte', kind: 'ROW_CELL' },
  // title читает `place_path` (поле DTO группового устройства), а не `full_path`
  { path: 'src/features/groups/GroupContentsTable.svelte', kind: 'ROW_CELL' },
  { path: 'src/features/groups/GroupAddDevicesModal.svelte', kind: 'ROW_CELL' },

  // --- LINK_BUTTON: кнопки «откуда» и «куда» в ленте перемещений ---
  { path: 'src/lib/components/MovementTimeline.svelte', kind: 'LINK_BUTTON', minSites: 2 },

  // --- REPORT_TABLE: ячейки строятся хелперами formatCellDisplay / formatCellTitle ---
  { path: 'src/features/reports/ReportTable.svelte', kind: 'REPORT_TABLE' },

  // --- PASSTHROUGH: объект с обоими путями уходит дальше, сам файл путь не печатает ---
  {
    path: 'src/features/devices/DeviceRowMenu.svelte',
    kind: 'PASSTHROUGH',
    reason:
      'собирает PlaceContentDto для карточки просмотра: оба пути (full_path и place_path_short) передаются потребителю, текст не печатается',
  },

  // --- FULL_BY_DESIGN: полный путь показан намеренно ---
  {
    path: 'src/features/acts/ActDetail.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'карточка акта: поле «Расположение» показывает полный путь, это не колонка списка',
  },
  {
    path: 'src/features/acts/ReturnModal.svelte',
    kind: 'FULL_BY_DESIGN',
    reason:
      'подписи складских мест для выбора места возврата: это сами места (places_get), а не колонка «Место» устройства',
  },
  {
    path: 'src/features/cartridges/CartridgeDetail.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'карточка картриджа: поле «Место» показывает полный путь, это не колонка списка',
  },
  {
    path: 'src/features/printers/PrinterDetail.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'карточка принтера: поле расположения показывает полный путь, это не колонка списка',
  },
  {
    path: 'src/features/places/PlaceTree.svelte',
    kind: 'FULL_BY_DESIGN',
    reason:
      'результаты поиска по дереву мест: показывают полный путь места, чтобы различить одноимённые узлы',
  },
  {
    path: 'src/features/places/PlaceTreeNode.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'подсказка узла дерева мест: полный путь самого места, а не устройства в списке',
  },
  {
    path: 'src/lib/components/PlacePicker.svelte',
    kind: 'FULL_BY_DESIGN',
    reason:
      'поле выбора места: показывает полный путь выбранного места (displayText в script — см. ограничение гейта)',
  },
  {
    path: 'src/features/groups/GroupPanel.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'место ГРУППЫ (place_path группы): у групп нет сокращённого варианта в DTO',
  },
  {
    path: 'src/features/groups/GroupTreeNode.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'место ГРУППЫ в дереве групп (place_path группы): сокращённого варианта нет',
  },
  {
    path: 'src/features/groups/AddToGroupModal.svelte',
    kind: 'FULL_BY_DESIGN',
    reason: 'место ГРУППЫ в диалоге добавления (place_path группы): сокращённого варианта нет',
  },

  // --- SLOT_TEXT: short читается в script и уходит в слот Dropdown (title недоступен) ---
  {
    path: 'src/features/acts/ActFormItemsTable.svelte',
    kind: 'SLOT_TEXT',
    reason:
      'подстрока строки-группы в пикере позиции акта: place_path_short идёт в слот Dropdown, полного пути в файле нет',
  },

  // --- DATA_ONLY: идентификатор есть, текст пользователю не печатается ---
  {
    path: 'src/features/reports/ReportsPage.svelte',
    kind: 'DATA_ONLY',
    reason:
      'описание колонок отчёта (key: place_path) и типы строк; ячейки печатает ReportTable (REPORT_TABLE)',
  },
  {
    path: 'src/features/settings/TemplateEditor.svelte',
    kind: 'DATA_ONLY',
    reason:
      'справочник переменных шаблона акта (`act.place_path`): описание переменной, не колонка',
  },

  // --- DEMO: витрина дизайн-системы с вымышленными данными ---
  {
    path: 'src/features/showcase/',
    kind: 'DEMO',
    reason: 'каталог витрины дизайн-системы: вымышленные строки-образцы, а не рабочие экраны',
  },
];

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

/** Все элементы с тегами из `tagNames` как {attrs, body, line}. */
function collectElements(markup, src, tagNames) {
  const elements = [];
  const lineOf = (idx) => src.slice(0, idx).split('\n').length;
  const re = new RegExp(`<(${tagNames.join('|')})[\\s>]`, 'g');

  for (const m of markup.matchAll(re)) {
    const start = m.index;
    const openEnd = endOfOpeningTag(markup, start);
    if (openEnd < 0) continue;
    const closeIdx = markup.indexOf(`</${m[1]}`, openEnd);
    if (closeIdx < 0) continue;
    elements.push({
      attrs: markup.slice(start, openEnd + 1),
      body: markup.slice(openEnd + 1, closeIdx),
      line: lineOf(start),
    });
  }
  return elements;
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

/** Тело функции по имени — для REPORT_TABLE. */
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
// Проверки по видам вердикта (возвращают список {message, hint})
// ---------------------------------------------------------------------------

function checkRowCell(src) {
  const out = [];
  const markup = markupOnly(src);
  const cells = collectElements(markup, src, ['td']);
  const placeCells = cells.filter((c) => SHORT_RE.test(c.body));

  if (placeCells.length === 0) {
    out.push({
      message: 'INV-1: ни одна ячейка <td> не рендерит сокращённый путь (place_path_short)',
      hint:
        'Колонка «Место» снова показывает полный путь — ровно тот дефект, из-за которого ' +
        'сгруппированный и плоский виды списка устройств разошлись (WR-04). ' +
        'Текст ячейки обязан читать place_path_short, приходящий с бэкенда.',
    });
    return out;
  }

  for (const cell of placeCells) {
    const title = titleExpression(cell.attrs);
    if (title === null) {
      out.push({
        message: `INV-2: ячейка с place_path_short (строка ${cell.line}) не имеет атрибута title={...}`,
        hint: 'Сокращённый путь без полного в title — единственный способ увидеть полное расположение исчезает.',
      });
    } else if (!FULL_RE.test(title)) {
      out.push({
        message: `INV-2: ячейка с place_path_short (строка ${cell.line}) имеет title={${title.trim()}} без полного пути`,
        hint: 'В title обязан стоять ПОЛНЫЙ путь (full_path / place_path) — «полное расположение в одном наведении».',
      });
    }
    if (FULL_RE.test(cell.body)) {
      out.push({
        message: `INV-3: ячейка с place_path_short (строка ${cell.line}) рендерит в тексте ещё и полный путь`,
        hint: 'Полный путь принадлежит title, а не тексту ячейки — иначе сокращение декоративно.',
      });
    }
  }
  return out;
}

function checkLinkButton(src, minSites) {
  const out = [];
  const markup = markupOnly(src);
  const elements = collectElements(markup, src, ['button', 'a']);
  const placeEls = elements.filter((e) => SHORT_RE.test(e.body));

  if (placeEls.length < minSites) {
    out.push({
      message: `найдено ${placeEls.length} кнопок/ссылок с сокращённым путём в тексте — меньше ${minSites}`,
      hint: 'Часть мест показывает полный путь в тексте либо разметка переехала: текст обязан быть `*_place_path_short` (откат на полный допустим), title — полный.',
    });
  }
  for (const el of placeEls) {
    const title = titleExpression(el.attrs);
    if (title === null) {
      out.push({
        message: `кнопка/ссылка с сокращённым путём (строка ${el.line}) не имеет title={...}`,
        hint: 'Без title полный путь недоступен.',
      });
    } else if (!FULL_RE.test(title)) {
      out.push({
        message: `кнопка/ссылка с сокращённым путём (строка ${el.line}) имеет title={${title.trim()}} без полного пути`,
        hint: 'В title обязан стоять ПОЛНЫЙ путь.',
      });
    }
  }
  return out;
}

function checkReportTable(src) {
  const out = [];
  const display = functionBody(src, 'formatCellDisplay');
  const title = functionBody(src, 'formatCellTitle');

  if (display === null || title === null) {
    out.push({
      message: `INV-4: не найдены функции formatCellDisplay/formatCellTitle (${display === null ? 'display' : 'title'})`,
      hint: 'Их переименовали или удалили — асимметрия «текст сокращён, title полный» больше никем не удерживается. Обнови гейт вместе с рефакторингом, осознанно.',
    });
    return out;
  }
  if (!SHORT_RE.test(display)) {
    out.push({
      message: 'INV-4: formatCellDisplay не читает place_path_short',
      hint: 'Колонка «Место» в отчётах снова печатает полный путь.',
    });
  }
  if (SHORT_RE.test(title)) {
    out.push({
      message: 'INV-4: formatCellTitle читает place_path_short',
      hint: 'В title обязан идти ПОЛНЫЙ путь; если и он сокращён — полного пути в отчёте не увидеть нигде.',
    });
  }
  return out;
}

function checkPassthrough(src) {
  const out = [];
  const scripts = [...src.matchAll(/<script[\s\S]*?<\/script>/gi)].map((m) => m[0]).join('\n');
  const code = stripComments(scripts);
  if (!/\bfull_path\s*:/.test(code)) {
    out.push({
      message: 'в <script> нет ключа `full_path:`',
      hint: 'Объект передаёт дальше только сокращённый путь — полного потребитель не получит.',
    });
  }
  if (!/\bplace_path_short\s*:/.test(code)) {
    out.push({
      message: 'в <script> нет ключа `place_path_short:`',
      hint: 'Объект передаёт дальше только полный путь — потребитель (список, карточка) покажет его в тексте вместо сокращённого.',
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// Обход и сборка нарушений (чистая: без process.exit — вызывается и из --selftest)
// ---------------------------------------------------------------------------

class GateReadError extends Error {}

const isScanned = (name) =>
  (name.endsWith('.svelte') || name.endsWith('.ts')) &&
  !name.endsWith('.d.ts') &&
  !/^bindings.*\.ts$/.test(name);

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(full));
    else if (entry.isFile() && isScanned(entry.name)) out.push(full);
  }
  return out;
}

/** Относительные пути (`src/...`, с `/`) всех проверяемых файлов, отсортированные. */
function listSourceFiles(root) {
  try {
    return walk(path.join(root, SRC_DIR))
      .map((f) => path.relative(root, f).split(path.sep).join('/'))
      .sort();
  } catch {
    throw new GateReadError(
      `не удалось обойти ${SRC_DIR}/. Каталог переехал/удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
    );
  }
}

function diskReader(root) {
  return (rel) => {
    try {
      return fs.readFileSync(path.join(root, rel), 'utf8');
    } catch {
      throw new GateReadError(
        `не удалось прочитать ${rel}. Файл переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
      );
    }
  };
}

const covers = (v, file) => v.path === file || (v.path.endsWith('/') && file.startsWith(v.path));

function matchVerdict(file, verdicts) {
  return (
    verdicts.find((v) => v.path === file) ??
    verdicts.find((v) => v.path.endsWith('/') && file.startsWith(v.path)) ??
    null
  );
}

/**
 * @param {(rel: string) => string} read
 * @param {string[]} files  относительные пути `src/...`
 * @param {Array<{path:string, kind:string, reason?:string, minSites?:number}>} verdicts
 * @returns {{violations: Array<{file:string, code:string, message:string, hint:string}>,
 *            stats: {familyFiles:number, shortConsumers:number}}}
 */
function collectViolations(read, files, verdicts = VERDICTS) {
  const violations = [];
  const add = (file, code, message, hint) => violations.push({ file, code, message, hint });

  // P4 + дубли (качество самой таблицы, без чтения файлов)
  const seen = new Set();
  for (const v of verdicts) {
    if (!ALL_KINDS.has(v.kind)) {
      add(
        v.path,
        'P4',
        `неизвестный вид вердикта «${v.kind}»`,
        `Допустимы: ${[...ALL_KINDS].join(', ')}.`,
      );
      continue;
    }
    const isDir = v.path.endsWith('/');
    if (!isDir && !(v.path.endsWith('.svelte') || v.path.endsWith('.ts'))) {
      add(
        v.path,
        'P4',
        'путь вердикта — не файл .svelte/.ts и не каталог с `/` на конце',
        'Каталог пишется с `/` на конце (иначе префикс захватит соседей: `features/requests` и `features/requests_old`).',
      );
    }
    if (CONSUMER_KINDS.has(v.kind) && (isDir || !v.path.endsWith('.svelte'))) {
      add(
        v.path,
        'P4',
        `вид ${v.kind} применим только к файлу .svelte (разметка), не к каталогу и не к .ts`,
        'Для .ts и каталогов подходят FULL_BY_DESIGN, DATA_ONLY и DEMO.',
      );
    }
    if (
      EXEMPT_KINDS.has(v.kind) &&
      (typeof v.reason !== 'string' || v.reason.trim().length < MIN_REASON)
    ) {
      add(
        v.path,
        'P4',
        `причина освобождения (${v.kind}) короче ${MIN_REASON} символов`,
        'Освобождение без объяснения невозможно проверить через год: напиши, ПОЧЕМУ путь здесь допустим.',
      );
    }
    if (seen.has(v.path)) {
      add(v.path, 'P3', 'дубль пути в таблице вердиктов', 'Оставь одну запись.');
    }
    seen.add(v.path);
  }

  // Содержимое без комментариев (одно чтение на файл)
  const raw = new Map();
  const code = new Map();
  for (const f of files) {
    const src = read(f);
    raw.set(f, src);
    code.set(f, stripComments(src));
  }
  const isFamily = (f) => SHORT_RE.test(code.get(f)) || FULL_RE.test(code.get(f));

  // P1: полнота
  const familyFiles = files.filter(isFamily);
  for (const f of familyFiles) {
    if (matchVerdict(f, verdicts) === null) {
      add(
        f,
        'P1',
        'файл использует идентификатор пути (place_path_short / full_path / place_path), но не имеет вердикта',
        'Добавь вердикт в VERDICTS с причиной (потребитель SHORT: ROW_CELL / LINK_BUTTON; освобождение: FULL_BY_DESIGN / DATA_ONLY) либо исправь компонент: в списках текст — place_path_short, title — полный путь.',
      );
    }
  }

  // P2 / P3: по вердиктам
  const consumerFiles = new Set();
  for (const v of verdicts) {
    if (!ALL_KINDS.has(v.kind)) continue;
    const matched = files.filter((f) => covers(v, f));
    if (matched.length === 0) {
      add(
        v.path,
        'P3',
        'вердикт не совпал ни с одним файлом под src/',
        'Файл переехал, переименован или удалён: обнови или удали запись осознанно (устаревшее освобождение молча оставляет дыру).',
      );
      continue;
    }

    if (CONSUMER_KINDS.has(v.kind)) {
      for (const f of matched) {
        if (f.endsWith('.svelte') && !v.path.endsWith('/')) consumerFiles.add(f);
      }
    }

    // P3: идентификатор семейства исчез из всех совпавших файлов
    const anyFamily = matched.some(isFamily);
    if (!anyFamily) {
      add(
        v.path,
        'P3',
        `вердикт ${v.kind}: идентификаторов семейства пути в файле(ах) больше нет`,
        'Освобождение устарело (код переписан) — удали запись либо верни проверяемое.',
      );
    }

    for (const f of matched) {
      const src = raw.get(f);
      if (f.endsWith('.svelte') === false && CONSUMER_KINDS.has(v.kind)) continue;

      let found = [];
      if (v.kind === 'ROW_CELL') found = checkRowCell(src);
      else if (v.kind === 'LINK_BUTTON') found = checkLinkButton(src, v.minSites ?? 1);
      else if (v.kind === 'REPORT_TABLE') found = checkReportTable(src);
      else if (v.kind === 'PASSTHROUGH') found = checkPassthrough(src);
      for (const x of found) add(f, 'P2', x.message, x.hint);

      if (v.kind === 'SLOT_TEXT') {
        if (!SHORT_RE.test(code.get(f))) {
          add(
            f,
            'P3',
            'вердикт SLOT_TEXT: сокращённого пути в файле больше нет',
            'Освобождение устарело — удали запись либо верни проверяемое.',
          );
        }
        if (FULL_RE.test(code.get(f))) {
          add(
            f,
            'P3',
            'вердикт SLOT_TEXT: в файле появился полный путь',
            'SLOT_TEXT допускает только сокращённый путь в слоте списка; файл с полным путём получает ROW_CELL / FULL_BY_DESIGN.',
          );
        }
        if (f.endsWith('.svelte') && SHORT_RE.test(markupOnly(raw.get(f)))) {
          add(
            f,
            'P3',
            'вердикт SLOT_TEXT: в разметке появился сокращённый путь (place_path_short)',
            'Файл теперь потребитель SHORT в разметке — переведи вердикт в ROW_CELL / LINK_BUTTON, чтобы контракт «текст short, title full» удерживался.',
          );
        }
      }

      if (v.kind === 'FULL_BY_DESIGN' || v.kind === 'DATA_ONLY') {
        if (!FULL_RE.test(code.get(f)) && isFamily(f)) {
          add(
            f,
            'P3',
            `вердикт ${v.kind}: идентификатора полного пути в файле больше нет`,
            'В файле остался только сокращённый путь — вердикт неверен, переведи в потребительский вид.',
          );
        }
        if (f.endsWith('.svelte') && SHORT_RE.test(markupOnly(raw.get(f)))) {
          add(
            f,
            'P3',
            `вердикт ${v.kind}: в разметке появился сокращённый путь (place_path_short)`,
            'Файл теперь потребитель SHORT — переведи вердикт в ROW_CELL / LINK_BUTTON, чтобы контракт «текст short, title full» удерживался.',
          );
        }
      }
    }
  }

  // P5: антивакуум
  if (consumerFiles.size < MIN_SHORT_CONSUMERS) {
    add(
      `${SRC_DIR}/`,
      'P5',
      `потребителей сокращённого пути ${consumerFiles.size} — меньше порога ${MIN_SHORT_CONSUMERS}`,
      'Либо вердикты потребителей пропали, либо обход перестал их видеть — «0 нарушений» было бы вакуумным. Если число законно упало, обнови MIN_SHORT_CONSUMERS осознанно.',
    );
  }
  if (verdicts.length < MIN_VERDICTS) {
    add(
      `${SRC_DIR}/`,
      'P5',
      `вердиктов ${verdicts.length} — меньше порога ${MIN_VERDICTS}`,
      'Записи таблицы удалены — освобождения больше не закодированы. Если число законно упало, обнови MIN_VERDICTS осознанно.',
    );
  }

  return {
    violations,
    stats: { familyFiles: familyFiles.length, shortConsumers: consumerFiles.size },
  };
}

// ---------------------------------------------------------------------------
// --report: инвентарь семейства (exit 0)
// ---------------------------------------------------------------------------

function report(read, files, verdicts) {
  let n = 0;
  for (const f of files) {
    const c = stripComments(read(f));
    const s = (c.match(SHORT_RE_G) ?? []).length;
    const u = (c.match(FULL_RE_G) ?? []).length;
    if (s === 0 && u === 0) continue;
    n++;
    const v = matchVerdict(f, verdicts);
    console.error(
      `${TAG} ${f} — SHORT×${s} FULL×${u} — ${v ? `${v.kind}${v.reason ? ` (${v.reason.slice(0, 40)}…)` : ''}` : 'БЕЗ ВЕРДИКТА'}`,
    );
  }
  console.error(`${TAG} REPORT — ${n} файлов семейства из ${files.length} просмотренных`);
}

// ---------------------------------------------------------------------------

function printViolations(violations) {
  for (const v of violations) {
    console.error(`${TAG} ${v.file} — ${v.code} (${v.message})`);
    console.error(`${TAG}   ${v.hint}`);
  }
}

function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--help') || argv.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-place-path-short.mjs [--src=<ui-dir>] [--report | --selftest]`,
    );
    process.exit(0);
  }
  const argSrc = argv.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (argv.includes('--selftest')) {
    selftest(SRC_ROOT);
    return;
  }

  let files;
  let read;
  try {
    files = listSourceFiles(SRC_ROOT);
    read = diskReader(SRC_ROOT);
  } catch (e) {
    console.error(`${TAG} FAIL — ${e.message}`);
    process.exit(1);
  }

  if (argv.includes('--report')) {
    report(read, files, VERDICTS);
    process.exit(0);
  }

  let result;
  try {
    result = collectViolations(read, files, VERDICTS);
  } catch (e) {
    if (e instanceof GateReadError) {
      console.error(`${TAG} FAIL — ${e.message}`);
      process.exit(1);
    }
    throw e;
  }

  printViolations(result.violations);
  if (result.violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${result.violations.length} нарушений контракта колонки «Место». ` +
        'Этот класс дефекта уже уезжал в сборку (WR-04) — не «чинить» гейт, а вернуть инвариант или осознанно добавить вердикт.',
    );
    process.exit(1);
  }
  console.error(
    `${TAG} PASS — 0 нарушений (${result.stats.familyFiles} файлов семейства, ` +
      `${result.stats.shortConsumers} потребителей SHORT, ${VERDICTS.length} вердиктов)`,
  );
  process.exit(0);
}

// ---------------------------------------------------------------------------
// --selftest
// ---------------------------------------------------------------------------

/**
 * Заменяет якорь `from` на `to`. Якорь обязан встречаться РОВНО один раз:
 * иначе `replace` молча заденет соседний фрагмент или не изменит ничего, и
 * кейс «докажет» не то, что заявлено (или не мутирует ничего).
 */
function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL — якорь «${label}» встречается ${n} раз (нужно ровно 1)`);
    process.exit(1);
  }
  return source.replace(from, () => to);
}

function selftest(root) {
  const realFiles = listSourceFiles(root);
  const disk = diskReader(root);

  /** Прогон на реальном дереве с подменой содержимого `over` (Map rel->text) и добавками `extra`. */
  const run = ({ over = new Map(), extra = [], files = realFiles, verdicts = VERDICTS } = {}) => {
    const all = [...files, ...extra.filter((e) => !files.includes(e))];
    const read = (rel) => (over.has(rel) ? over.get(rel) : disk(rel));
    return collectViolations(read, all, verdicts);
  };
  const swap = (rel, from, to, label) =>
    new Map([[rel, mutate(disk(rel), from, to, `${rel}: ${label}`)]]);

  const GBD = 'src/features/acts/ActDetail.svelte';
  const CLEAN_ROW =
    '<table><tbody><tr><td title={x.place_path}>{x.place_path_short}</td></tr></tbody></table>';
  const rowVerdict = (p) => ({ path: p, kind: 'ROW_CELL' });

  const cases = [
    {
      name: 'контроль: реальное дерево чисто, потребителей достаточно',
      run: () => run(),
      expect: [],
      check: (r) => r.stats.shortConsumers >= MIN_SHORT_CONSUMERS,
    },
    {
      name: 'обход: bindings*.ts и *.d.ts не сканируются, ui/src/lib/components виден',
      run: () => run(),
      expect: [],
      check: () =>
        !realFiles.some((f) => /\/bindings[^/]*\.ts$|\.d\.ts$/.test(f)) &&
        realFiles.includes('src/lib/components/MovementTimeline.svelte'),
    },
    {
      name: '4a: новый .svelte с сырым full_path без вердикта — P1',
      run: () =>
        run({
          extra: ['src/features/zz_synthetic/Row.svelte'],
          over: new Map([['src/features/zz_synthetic/Row.svelte', '<td>{row.full_path}</td>']]),
        }),
      expect: ['P1'],
    },
    {
      name: '4a: новый .ts с идентификатором пути без вердикта — P1',
      run: () =>
        run({
          extra: ['src/lib/utils/zz_synthetic.ts'],
          over: new Map([
            ['src/lib/utils/zz_synthetic.ts', 'export const f = (r) => r.place_path;'],
          ]),
        }),
      expect: ['P1'],
    },
    {
      // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
      // Блочно-первая вычистка стирала бы чтение пути между `templates/*.html` и
      // ближайшим `*/` ниже — P1 молчало бы.
      name: '4a: `/*` в строчном комментарии не прячет идентификатор пути — P1',
      run: () =>
        run({
          extra: ['src/lib/utils/zz_comment_trap.ts'],
          over: new Map([
            [
              'src/lib/utils/zz_comment_trap.ts',
              [
                '// шаблоны лежат в templates/*.html',
                'export const f = (r) => r.place_path;',
                '/* настоящий блочный комментарий */',
                '',
              ].join('\n'),
            ],
          ]),
        }),
      expect: ['P1'],
    },
    {
      name: '4b-1: GroupAddDevicesModal — текст ячейки снова full_path — P2',
      run: () =>
        run({
          over: swap(
            'src/features/groups/GroupAddDevicesModal.svelte',
            "{d.place_path_short ?? '—'}",
            "{d.full_path ?? '—'}",
            'текст ячейки читает full_path',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: '4b-2: MovementTimeline — текст одной кнопки без short — P2',
      run: () =>
        run({
          over: swap(
            'src/lib/components/MovementTimeline.svelte',
            '{entry.from_place_path_short ?? entry.from_place_path}',
            '{entry.from_place_path}',
            'текст from-кнопки без short',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: '4c: GroupContentsTable — title вместо полного пути читает short — P2',
      run: () =>
        run({
          over: swap(
            'src/features/groups/GroupContentsTable.svelte',
            'title={d.place_path ?? undefined}',
            'title={d.place_path_short ?? undefined}',
            'title читает short',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: '4d: освобождение устарело — из FULL_BY_DESIGN-файла исчез путь — P3',
      run: () => run({ over: new Map([[GBD, '<div>пусто</div>']]) }),
      expect: ['P3'],
    },
    {
      name: '4d: вердикт на несуществующий путь — P3',
      run: () =>
        run({
          verdicts: [
            ...VERDICTS,
            {
              path: 'src/features/zz_gone/Gone.svelte',
              kind: 'FULL_BY_DESIGN',
              reason: 'файл удалён, запись осталась (синтетика)',
            },
          ],
        }),
      expect: ['P3'],
      // не «идентификатор исчез» (соседнее P3-правило тоже сработало бы на пустом списке файлов)
      check: (r) => r.violations.some((v) => v.message.startsWith('вердикт не совпал')),
    },
    {
      name: '4d: в FULL_BY_DESIGN-файле появился short в разметке — P3',
      run: () => run({ over: new Map([[GBD, `${disk(GBD)}\n<td>{x.place_path_short}</td>\n`]]) }),
      expect: ['P3'],
    },
    {
      name: '4d: дубль пути в таблице — P3',
      run: () => run({ verdicts: [...VERDICTS, VERDICTS.find((v) => v.path === GBD)] }),
      expect: ['P3'],
    },
    {
      name: '4e: вакуум — скану один файл и одна запись — P5',
      run: () =>
        run({
          files: ['src/features/devices/DeviceListRow.svelte'],
          verdicts: [rowVerdict('src/features/devices/DeviceListRow.svelte')],
        }),
      expect: ['P5'],
    },
    {
      name: '4e: потребителей достаточно, а вердиктов мало — P5 именно по вердиктам',
      run: () => {
        const consumers = VERDICTS.filter(
          (v) => CONSUMER_KINDS.has(v.kind) && v.path.endsWith('.svelte'),
        );
        return run({ files: consumers.map((v) => v.path), verdicts: consumers });
      },
      expect: ['P5'],
      check: (r) =>
        r.stats.shortConsumers >= MIN_SHORT_CONSUMERS &&
        r.violations.every((v) => v.message.startsWith('вердиктов')),
    },
    {
      name: '4e: вердиктов достаточно, а потребителей мало — P5 именно по потребителям',
      run: () => {
        const drop = 'src/features/devices/DeviceListRow.svelte';
        const pad = 'src/features/zz_synthetic/Pad.svelte';
        return run({
          files: realFiles.filter((f) => f !== drop),
          extra: [pad],
          over: new Map([[pad, '<span>{x.place_path}</span>']]),
          verdicts: [
            ...VERDICTS.filter((v) => v.path !== drop),
            { path: pad, kind: 'DATA_ONLY', reason: 'синтетическая добивка числа вердиктов' },
          ],
        });
      },
      expect: ['P5'],
      check: (r) =>
        r.stats.shortConsumers === MIN_SHORT_CONSUMERS - 1 &&
        r.violations.every((v) => v.message.startsWith('потребителей')),
    },
    {
      name: '4f: DeviceRowMenu — из объекта убран ключ place_path_short — P2',
      run: () =>
        run({
          over: swap(
            'src/features/devices/DeviceRowMenu.svelte',
            "      place_path_short: dev.place_path_short ?? '',\n",
            '',
            'ключ place_path_short убран',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: 'ReportTable: formatCellDisplay перестал читать short — P2 (INV-4)',
      run: () =>
        run({
          over: swap(
            'src/features/reports/ReportTable.svelte',
            "return formatPlaceCell(row, col, () => row.place_path_short ?? '');",
            'return formatPlaceCell(row, col, (p) => p);',
            'display без short',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: 'ReportTable: formatCellTitle читает short — P2 (INV-4)',
      run: () =>
        run({
          over: swap(
            'src/features/reports/ReportTable.svelte',
            'return formatPlaceCell(row, col, (p) => p);',
            "return formatPlaceCell(row, col, () => row.place_path_short ?? '');",
            'title с short',
          ),
        }),
      expect: ['P2'],
    },
    {
      name: 'INV-3: ячейка рендерит в тексте и short, и full — P2',
      run: () =>
        run({
          extra: ['src/features/zz_synthetic/Both.svelte'],
          over: new Map([
            [
              'src/features/zz_synthetic/Both.svelte',
              '<td title={x.full_path}>{x.place_path_short} {x.full_path}</td>',
            ],
          ]),
          verdicts: [...VERDICTS, rowVerdict('src/features/zz_synthetic/Both.svelte')],
        }),
      expect: ['P2'],
    },
    {
      name: 'SLOT_TEXT: в пикере акта в разметке появился short — P3',
      run: () =>
        run({
          over: new Map([
            [
              'src/features/acts/ActFormItemsTable.svelte',
              disk('src/features/acts/ActFormItemsTable.svelte') +
                '\n<td>{x.place_path_short}</td>\n',
            ],
          ]),
        }),
      expect: ['P3'],
    },
    {
      name: 'SLOT_TEXT: в пикере акта появился полный путь — P3',
      run: () =>
        run({
          over: new Map([
            [
              'src/features/acts/ActFormItemsTable.svelte',
              disk('src/features/acts/ActFormItemsTable.svelte') +
                '\n<script>const f = x.full_path;</script>\n',
            ],
          ]),
        }),
      expect: ['P3'],
    },
    {
      name: 'P4: причина освобождения короче 20 символов',
      run: () =>
        run({
          verdicts: VERDICTS.map((v) => (v.path === GBD ? { ...v, reason: 'карточка' } : v)),
        }),
      expect: ['P4'],
    },
    {
      name: 'P4: каталог без `/` на конце',
      run: () =>
        run({
          verdicts: VERDICTS.map((v) =>
            v.path === 'src/features/showcase/' ? { ...v, path: 'src/features/showcase' } : v,
          ),
        }),
      // каталог без `/` — P4; его файлы остаются без вердикта (P1) и вердикт «не совпал» (P3)
      expect: ['P1', 'P3', 'P4'],
    },
    {
      name: 'P4: неизвестный вид вердикта',
      run: () =>
        run({
          verdicts: VERDICTS.map((v) => (v.path === GBD ? { ...v, kind: 'SOMETHING' } : v)),
        }),
      // вердикт с неизвестным видом пропускается, файл остаётся без вердикта по сути (matchVerdict находит его → P1 нет)
      expect: ['P4'],
    },
    {
      name: 'ложное срабатывание: <td title={place_path}>{place_path_short}</td> с ROW_CELL — чисто',
      run: () =>
        run({
          extra: ['src/features/zz_synthetic/Ok.svelte'],
          over: new Map([['src/features/zz_synthetic/Ok.svelte', CLEAN_ROW]]),
          verdicts: [...VERDICTS, rowVerdict('src/features/zz_synthetic/Ok.svelte')],
        }),
      expect: [],
    },
  ];

  let failed = 0;
  let ran = 0;
  for (const c of cases) {
    ran++;
    const r = c.run();
    const got = [...new Set(r.violations.map((v) => v.code))].sort();
    const want = [...c.expect].sort();
    const setOk = got.length === want.length && got.every((x, i) => x === want[i]);
    const extraOk = c.check ? c.check(r) : true;
    if (!setOk || !extraOk) {
      failed++;
      console.error(
        `${TAG} SELFTEST FAIL — ${c.name}: ожидалось {${want.join(',') || '—'}}, получено {${got.join(',') || '—'}}${extraOk ? '' : ' (доп. проверка не прошла)'}`,
      );
      for (const v of r.violations.slice(0, 6))
        console.error(`${TAG}     ${v.file} — ${v.code}: ${v.message}`);
    }
  }

  if (failed > 0) {
    console.error(`${TAG} SELFTEST FAIL — ${failed} из ${ran} кейсов не прошли`);
    process.exit(1);
  }
  console.error(`${TAG} SELFTEST PASS — ${ran} кейсов (мутанты по реальным файлам)`);
  process.exit(0);
}

main();
