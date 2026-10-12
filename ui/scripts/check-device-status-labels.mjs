#!/usr/bin/env node
// [check-device-status-labels] Постоянный гейт «подписи и тона статусов
// устройств читаются из единого справочника, а не из хардкода» (фаза 41.1,
// план 41.1-10, GRD-04, D-06 / D-07 / D-09).
//
// Почему он существует: статусы устройств были размножены по семи экранам —
// карты `Record<number, string>`, массивы `{ id: 1, label: 'На складе' }`,
// карты «имя -> тон». Когда справочник стал шестизначным (вместо четырёх
// статусов — шесть), каждая такая копия молча разошлась с сервером. Чинится
// не правкой копий, а запретом самой формы: подписи приходят из стора
// `$lib/stores/deviceStatuses.svelte`, тон решается по `code`.
//
// Гейт СТРУКТУРНЫЙ, а не литеральный: он не краснит слово «В работе» само по
// себе (это ещё и статус ЗАЯВКИ, и имя вкладки отчёта), а ловит формы, в
// которых слово-статус стоит как значение справочника.
//
// Правила (сообщение начинается с кода правила в квадратных скобках):
//   [A] хардкод подписи по числовому id, две формы:
//       (1) объектный литерал / `Record<number, string>` с числовым ключом и
//           значением-словом статуса (`1: 'На складе'`), а также пара
//           `[1, 'На складе']`;
//       (2) объектный литерал `{ id: <число>, label|name: '<слово-статус>' }`
//           (и обратный порядок полей) — элемент массива опций/вкладок.
//           Сосед `{ id: null, label: 'Все' }` нарушением не является;
//   [B] карта «имя -> тон»: ключ — слово-статус, значение — токен тона
//       (default|accent|success|warning|destructive), либо идентификатор
//       оканчивается на `_BY_NAME`;
//   [C] `status_id`/`statusId` сравнивается с числовым литералом 1-6, либо
//       задан им в литерале (`status_id: 1`), либо объявлена константа
//       `*STATUS_ID = <число>`;
//   [D] (только полный режим) найдено не меньше MIN_CONSUMERS файлов,
//       импортирующих `$lib/stores/deviceStatuses.svelte`: иначе стор никто не
//       читает и гейт «зелёный» по пустоте;
//   [E] таблица освобождений EXEMPT: не меньше MIN_EXEMPT записей, у каждой
//       причина не короче 20 символов, каталожный префикс заканчивается на `/`
//       (иначе `features/requests` освободил бы и `features/requests_old`), а
//       в полном режиме каждая запись обязана совпасть хотя бы с одним файлом
//       (устаревшее освобождение молча глушило бы гейт).
//
// Список проверяемых файлов ВЫВОДИТСЯ обходом src/**/*.svelte и src/**/*.ts
// (кроме сгенерированного src/bindings.ts) — ничего не перечислено вручную,
// поэтому новый экран с хардкодом не может остаться незамеченным. Освободить
// файл можно только записью в EXEMPT с причиной.
//
// Комментарии (HTML, JS) перед проверкой затираются с сохранением длины и
// переводов строк: цитаты старых форм в комментариях гейт не обманывают, а
// номера строк в сообщениях честные.
//
// Self-test (`--selftest`): корректная фикстура в памяти (обязана пройти) и
// мутанты, каждый получен заменой с ПРОВЕРКОЙ, что якорь встречается ровно
// один раз. Ожидание сверяется ТОЧНЫМ множеством кодов правил.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-device-status-labels.mjs                       # весь репозиторий
//   node scripts/check-device-status-labels.mjs --scope=src/features/devices
//   node scripts/check-device-status-labels.mjs --src=<dir>           # копия каталога ui/
//   node scripts/check-device-status-labels.mjs --selftest            # встроенный self-test
//
// `--scope=<префикс пути от ui/>` ограничивает проверку поддеревом (правила D и
// проверка устаревших освобождений в этом режиме не выполняются) — для
// поэтапной миграции экранов.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-status-labels]';

const STORE_IMPORT = '$lib/stores/deviceStatuses.svelte';
const GENERATED = new Set(['src/bindings.ts']);

// Сколько файлов обязаны читать стор после перевода всех экранов (план 41.1-13).
const MIN_CONSUMERS = 8;
const SELFTEST_MIN_CONSUMERS = 2;
const MIN_EXEMPT = 7;
const SELFTEST_MIN_EXEMPT = 2;

const STATUS_WORDS = [
  'Ожидает ремонта',
  'На списание',
  'На складе',
  'В работе',
  'На ремонте',
  'В ремонте',
  'Списано',
];
const WORD_ALT = STATUS_WORDS.join('|');
const TONES = 'default|accent|success|warning|destructive';
const Q = '[\'"`]';
const NOT_IDENT = '(?<![\\p{L}\\p{N}_$.])';

// Вердикты по инвентарю, собранному грепом ОТ РЕПОЗИТОРИЯ (план 41.1-10):
//   переведено в плане 10: devices/DeviceListRow, DeviceGroupRow, DeviceFilters,
//     DeviceFormBody (+ DevicesPage — загрузка справочника);
//   переведено в плане 13: groups/GroupAddDevicesModal, groups/GroupContentsTable,
//     places/PlaceContents (карта «имя -> тон» по status_name);
//   освобождено с причиной — таблица ниже.
const EXEMPT = [
  {
    path: 'src/features/showcase/',
    reason: 'витрина дизайн-системы: демонстрация тонов бейджей и таблицы с произвольными данными',
  },
  {
    path: 'src/features/cartridges/',
    reason: 'статусы КАРТРИДЖЕЙ (4 статуса, свой справочник), не устройств (D-07)',
  },
  {
    path: 'src/features/reports/ReportSubNav.svelte',
    reason: 'подписи подвкладок отчётов («В работе», «На складе») — названия отчётов, не статусы',
  },
  {
    path: 'src/features/reports/ReportsPage.svelte',
    reason: 'имена вкладок отчётов и список статусов картриджей (id 3 — «На заправке»)',
  },
  {
    path: 'src/features/reports/ReportFilters.svelte',
    reason: 'опции «На складе»/«В эксплуатации» — фильтр is_storage по месту, не статус устройства',
  },
  {
    path: 'src/features/requests/',
    reason: '«В работе» — статус ЗАЯВКИ (in_progress), у заявок свой жизненный цикл',
  },
  {
    path: 'src/features/dashboard/DashboardPage.svelte',
    reason: '«В работе» — счётчик заявок на виджете, а не статус устройства',
  },
  {
    path: 'src/lib/stores/deviceStatuses.svelte.ts',
    reason: 'сам стор: единственная таблица тонов по code, подписей он не содержит',
  },
];

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
}

/** Затирает JS-комментарии, не трогая строковые литералы; длина сохраняется. */
function stripComments(src) {
  let out = '';
  let i = 0;
  let mode = 'code';
  while (i < src.length) {
    const c = src[i];
    const next = src[i + 1];
    if (mode === 'code') {
      if (c === '/' && next === '/') {
        mode = 'line';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === '/' && next === '*') {
        mode = 'block';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === "'" || c === '"' || c === '`') mode = c;
      out += c;
      i += 1;
      continue;
    }
    if (mode === 'line') {
      if (c === '\n') {
        mode = 'code';
        out += c;
      } else out += ' ';
      i += 1;
      continue;
    }
    if (mode === 'block') {
      if (c === '*' && next === '/') {
        mode = 'code';
        out += '  ';
        i += 2;
        continue;
      }
      out += c === '\n' ? '\n' : ' ';
      i += 1;
      continue;
    }
    if (c === '\\') {
      out += c + (next ?? '');
      i += 2;
      continue;
    }
    if (c === '\n' && mode !== '`') mode = 'code';
    else if (c === mode) mode = 'code';
    out += c;
    i += 1;
  }
  return out;
}

/** Делит .svelte на script / markup (каждый — той же длины, остальное затёрто). */
function splitSvelte(src) {
  const blocks = [];
  for (const m of src.matchAll(/<script\b[^>]*>[\s\S]*?<\/script>/g)) {
    blocks.push({ kind: 'script', start: m.index, end: m.index + m[0].length });
  }
  for (const m of src.matchAll(/<style\b[^>]*>[\s\S]*?<\/style>/g)) {
    blocks.push({ kind: 'style', start: m.index, end: m.index + m[0].length });
  }
  let script = blank(src);
  for (const b of blocks.filter((x) => x.kind === 'script')) {
    script = script.slice(0, b.start) + src.slice(b.start, b.end) + script.slice(b.end);
  }
  let markup = src;
  for (const b of blocks) {
    markup = markup.slice(0, b.start) + blank(src.slice(b.start, b.end)) + markup.slice(b.end);
  }
  markup = markup.replace(/<!--[\s\S]*?-->/g, (m) => blank(m));
  return { script: stripComments(script), markup };
}

/** Участки кода файла: для .svelte — script и markup, для .ts — весь файл. */
function codeParts(file, src) {
  if (file.endsWith('.svelte')) {
    const { script, markup } = splitSvelte(src);
    return [script, markup];
  }
  return [stripComments(src)];
}

function lineOf(src, idx) {
  return src.slice(0, idx).split('\n').length;
}

const RE_A_NUMERIC_KEY = new RegExp(
  `${NOT_IDENT}\\d+\\s*:\\s*(${Q})\\s*(?:${WORD_ALT})\\s*\\1`,
  'gu',
);
const RE_A_PAIR = new RegExp(`\\[\\s*\\d+\\s*,\\s*(${Q})\\s*(?:${WORD_ALT})\\s*\\1\\s*\\]`, 'gu');
const RE_A_ID_LABEL = new RegExp(
  `\\bid\\s*:\\s*\\d+\\s*,\\s*(?:label|name)\\s*:\\s*(${Q})\\s*(?:${WORD_ALT})\\s*\\1`,
  'gu',
);
const RE_A_LABEL_ID = new RegExp(
  `\\b(?:label|name)\\s*:\\s*(${Q})\\s*(?:${WORD_ALT})\\s*\\1\\s*,\\s*id\\s*:\\s*\\d+`,
  'gu',
);
const RE_B_NAME_TONE = new RegExp(
  `(?:(${Q})\\s*(?:${WORD_ALT})\\s*\\1|${NOT_IDENT}Списано)\\s*:\\s*(${Q})(?:${TONES})\\2`,
  'gu',
);
const RE_B_BY_NAME = /\b\w*_BY_NAME\b/g;
const RE_C_CMP = /\b(?:status_id|statusId)\b\s*[!=]==?\s*[1-6]\b/g;
const RE_C_CMP_REV = /\b[1-6]\s*[!=]==?\s*[\w.?]*\b(?:status_id|statusId)\b/g;
const RE_C_LITERAL = /\b(?:status_id|statusId)\b\s*:\s*[1-6]\b(?![\w.])/g;
const RE_C_CONST = /\b\w*STATUS_ID\b\s*(?::[^=;\n]+)?=\s*\d+/g;

const SCAN_RULES = [
  ['A', RE_A_NUMERIC_KEY, 'карта «числовой id -> подпись статуса» — подписи берутся из стора'],
  ['A', RE_A_PAIR, 'пара [id, подпись статуса] — подписи берутся из стора'],
  [
    'A',
    RE_A_ID_LABEL,
    'элемент { id, label } с подписью статуса — опции берутся из orderedStatuses()',
  ],
  [
    'A',
    RE_A_LABEL_ID,
    'элемент { label, id } с подписью статуса — опции берутся из orderedStatuses()',
  ],
  ['B', RE_B_NAME_TONE, 'карта «имя статуса -> тон» — тон решает statusTone(code)'],
  ['B', RE_B_BY_NAME, 'идентификатор *_BY_NAME — карта «имя -> …» запрещена, нужен code'],
  ['C', RE_C_CMP, 'status_id сравнивается с числовым литералом — используйте code статуса'],
  ['C', RE_C_CMP_REV, 'числовой литерал сравнивается со status_id — используйте code статуса'],
  ['C', RE_C_LITERAL, 'status_id: <число> в литерале — подставляйте id по code из справочника'],
  ['C', RE_C_CONST, 'константа *STATUS_ID с числом — используйте statusIdByCode(code)'],
];

function exemptionFor(file, exempt) {
  return exempt.find((e) => (e.path.endsWith('/') ? file.startsWith(e.path) : file === e.path));
}

/**
 * Чистая функция сканирования: `files` — Map «POSIX-путь от ui/ -> текст».
 * Переиспользуется и реальным сканированием, и self-testом.
 */
function scanSources(files, opts, violations) {
  const { scope, exempt, minConsumers, minExempt } = opts;
  const add = (file, line, rule, msg) =>
    violations.push(`${file}${line ? `:${line}` : ''} — [${rule}] ${msg}`);
  const inScope = (file) => !scope || file.startsWith(scope);

  // --- E: сама таблица освобождений -----------------------------------------
  if (exempt.length < minExempt) {
    add(
      'ui/scripts/check-device-status-labels.mjs',
      0,
      'E',
      `в таблице освобождений ${exempt.length} записей, ожидается не меньше ${minExempt} — список опустошён.`,
    );
  }
  for (const e of exempt) {
    if ((e.reason ?? '').trim().length < 20) {
      add(e.path, 0, 'E', 'у освобождения нет внятной причины (не короче 20 символов).');
    }
    if (!e.path.endsWith('/') && !/\.(svelte|ts)$/.test(e.path)) {
      add(e.path, 0, 'E', 'каталожное освобождение обязано заканчиваться на `/`.');
    }
    if (!scope && ![...files.keys()].some((f) => exemptionFor(f, [e]))) {
      add(e.path, 0, 'E', 'освобождение не совпало ни с одним файлом — устарело, удалите запись.');
    }
  }

  // --- A / B / C --------------------------------------------------------------
  let consumers = 0;
  for (const [file, src] of files) {
    if (GENERATED.has(file)) continue;
    if (src.includes(STORE_IMPORT)) consumers++;
    if (!inScope(file)) continue;
    if (exemptionFor(file, exempt)) continue;
    for (const part of codeParts(file, src)) {
      for (const [rule, re, msg] of SCAN_RULES) {
        re.lastIndex = 0;
        for (const m of part.matchAll(re)) {
          add(file, lineOf(part, m.index), rule, `${msg}: «${m[0].replace(/\s+/g, ' ').trim()}».`);
        }
      }
    }
  }

  // --- D ----------------------------------------------------------------------
  if (!scope && consumers < minConsumers) {
    add(
      'src',
      0,
      'D',
      `стор ${STORE_IMPORT} импортируют ${consumers} файлов, ожидается не меньше ${minConsumers} — справочник никто не читает.`,
    );
  }
}

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.name.endsWith('.svelte') || e.name.endsWith('.ts')) out.push(p);
  }
  return out;
}

function checkRepo(root, scope, violations) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) {
    violations.push(`src: каталог не найден (${srcDir}).`);
    return;
  }
  const files = new Map();
  for (const abs of walk(srcDir)) {
    files.set(path.relative(root, abs).split(path.sep).join('/'), fs.readFileSync(abs, 'utf8'));
  }
  scanSources(
    files,
    { scope, exempt: EXEMPT, minConsumers: MIN_CONSUMERS, minExempt: MIN_EXEMPT },
    violations,
  );
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

// В комментариях фикстуры нарочно процитированы запрещённые формы: гейт обязан
// их игнорировать.
const CLEAN_ROW = `<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import { statusLabel, statusTone } from '${STORE_IMPORT}';
  // раньше: const STATUS_LABELS = { 1: 'На складе', 2: 'В работе' }
  // раньше: { id: 1, label: 'На складе' }
  const { device } = $props();
  const label = $derived(statusLabel(device.status_id));
  const tone = $derived(statusTone(device.status_code));
</script>

<!-- раньше: STATUS_VARIANT_BY_NAME = { 'На складе': 'default' } -->
<Badge variant={tone.variant} appearance={tone.appearance}>{label}</Badge>
`;

const CLEAN_FILTERS = `<script lang="ts">
  import Tabs from '$lib/components/Tabs.svelte';
  import { orderedStatuses } from '${STORE_IMPORT}';
  const all = { id: null, label: 'Все' };
  const tabs = $derived([all, ...orderedStatuses().map((s) => ({ id: s.id, label: s.name }))]);
</script>

<Tabs {tabs} />
`;

// Статус ЗАЯВКИ: освобождённый файл с формой, которую иначе поймало бы правило A.
const REQUEST_ROW = `<script lang="ts">
  const STATES = [
    { id: 2, label: 'В работе' },
    { id: 3, label: 'Выполнена' },
  ];
</script>

{#each STATES as s}<span>{s.label}</span>{/each}
`;

const SHOWCASE = `<script lang="ts">
  const NAMES: Record<number, string> = { 1: 'На складе', 2: 'В работе' };
</script>
`;

const PATH_ROW = 'src/features/devices/Row.svelte';
const PATH_FILTERS = 'src/features/devices/Filters.svelte';
const PATH_REQUEST = 'src/features/requests/RequestRow.svelte';
const PATH_SHOWCASE = 'src/features/showcase/Table.svelte';
const SELF_EXEMPT = [
  { path: 'src/features/requests/', reason: '«В работе» — статус заявки, а не устройства' },
  { path: 'src/features/showcase/', reason: 'витрина: произвольные демонстрационные данные' },
];

/** Мутация с проверкой уникальности якоря: иначе мутант мог бы задеть соседа. */
function mutate(src, from, to) {
  const count = src.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`якорь мутации встречается ${count} раз(а), ожидалось ровно 1: ${from.trim()}`);
  }
  return src.replace(from, to);
}

function fixtureFiles(over = {}) {
  return new Map([
    [PATH_ROW, over.row ?? CLEAN_ROW],
    [PATH_FILTERS, over.filters ?? CLEAN_FILTERS],
    [PATH_REQUEST, over.request ?? REQUEST_ROW],
    [PATH_SHOWCASE, over.showcase ?? SHOWCASE],
    ...(over.extra ?? []),
  ]);
}

const SELF_OPTS = {
  scope: null,
  exempt: SELF_EXEMPT,
  minConsumers: SELFTEST_MIN_CONSUMERS,
  minExempt: SELFTEST_MIN_EXEMPT,
};

function runSelfTest() {
  const fixtures = [];
  try {
    fixtures.push({
      name: 'корректные экраны по стору + освобождённые заявки/витрина (позитив)',
      files: fixtureFiles(),
      codes: [],
    });
    fixtures.push({
      name: 'M1 [A] карта Record<number,string> со словом статуса',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          '  const { device } = $props();\n',
          "  const { device } = $props();\n  const LABELS: Record<number, string> = { 1: 'На складе', 4: 'Списано' };\n",
        ),
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: "M2 [A] массив { id: 1, label: 'На складе' } без числовых ключей",
      files: fixtureFiles({
        filters: mutate(
          CLEAN_FILTERS,
          "  const all = { id: null, label: 'Все' };\n",
          "  const all = { id: null, label: 'Все' };\n  const STATUSES = [{ id: 1, label: 'На складе' }];\n",
        ),
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: "M3 [A] обратный порядок полей { name: 'Списано', id: 4 }",
      files: fixtureFiles({
        filters: mutate(
          CLEAN_FILTERS,
          "  const all = { id: null, label: 'Все' };\n",
          "  const all = { id: null, label: 'Все' };\n  const X = { name: 'Списано', id: 4 };\n",
        ),
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: "M4 [B] карта имя -> тон ('На складе': 'default')",
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          '  const { device } = $props();\n',
          "  const { device } = $props();\n  const TONE = { 'На складе': 'default', Списано: 'destructive' };\n",
        ),
      }),
      codes: ['B'],
    });
    fixtures.push({
      name: 'M5 [B] идентификатор STATUS_VARIANT_BY_NAME',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          '  const { device } = $props();\n',
          '  const { device } = $props();\n  const STATUS_VARIANT_BY_NAME = {};\n',
        ),
      }),
      codes: ['B'],
    });
    fixtures.push({
      name: 'M6 [C] status_id === 1 в выборе тона',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          '  const tone = $derived(statusTone(device.status_code));\n',
          "  const tone = $derived(device.status_id === 1 ? statusTone('x') : statusTone('y'));\n",
        ),
      }),
      codes: ['C'],
    });
    fixtures.push({
      name: 'M7 [C] константа STORAGE_STATUS_ID = 1',
      files: fixtureFiles({
        filters: mutate(
          CLEAN_FILTERS,
          "  const all = { id: null, label: 'Все' };\n",
          "  const all = { id: null, label: 'Все' };\n  const STORAGE_STATUS_ID = 1;\n",
        ),
      }),
      codes: ['C'],
    });
    fixtures.push({
      name: 'M7b [C] литерал фильтра status_id: 1',
      files: fixtureFiles({
        filters: mutate(
          CLEAN_FILTERS,
          "  const all = { id: null, label: 'Все' };\n",
          "  const all = { id: null, label: 'Все' };\n  const filter = { status_id: 1, name: null };\n",
        ),
      }),
      codes: ['C'],
    });
    fixtures.push({
      name: 'M8 [D] ни один экран не импортирует стор',
      files: fixtureFiles({
        row: mutate(CLEAN_ROW, `import { statusLabel, statusTone } from '${STORE_IMPORT}';`, ''),
        filters: mutate(CLEAN_FILTERS, `import { orderedStatuses } from '${STORE_IMPORT}';`, ''),
      }),
      codes: ['D'],
    });
    fixtures.push({
      name: 'M9 [E] пустая таблица освобождений (форма заявок перестаёт быть освобождённой)',
      files: fixtureFiles(),
      opts: { ...SELF_OPTS, exempt: [] },
      codes: ['A', 'E'],
    });
    fixtures.push({
      name: 'M10 [E] освобождение без причины',
      files: fixtureFiles(),
      opts: {
        ...SELF_OPTS,
        exempt: [{ path: 'src/features/requests/', reason: 'надо' }, SELF_EXEMPT[1]],
      },
      codes: ['E'],
    });
    fixtures.push({
      name: 'M11 [A] соседний каталог features/requests_old/ не попадает под освобождение features/requests/',
      files: fixtureFiles({
        extra: [['src/features/requests_old/Row.svelte', REQUEST_ROW]],
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: 'M12 [E] устаревшее освобождение (не совпало ни с одним файлом)',
      files: fixtureFiles(),
      opts: {
        ...SELF_OPTS,
        exempt: [
          ...SELF_EXEMPT,
          { path: 'src/features/gone/', reason: 'каталог давно удалён из дерева' },
        ],
      },
      codes: ['E'],
    });
    fixtures.push({
      name: 'M13 [A] мутация внутри .ts-файла (не только .svelte)',
      files: fixtureFiles({
        extra: [
          [
            'src/lib/utils/labels.ts',
            "export const L: Record<number, string> = { 3: 'На ремонте' };\n",
          ],
        ],
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: 'M14 [A] --scope: нарушение вне scope не считается, внутри — считается',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          '  const { device } = $props();\n',
          "  const { device } = $props();\n  const L = { 1: 'На складе' };\n",
        ),
        extra: [
          ['src/features/groups/Modal.svelte', "<script>const L = { 2: 'В работе' };</script>\n"],
        ],
      }),
      opts: { ...SELF_OPTS, scope: 'src/features/groups/' },
      codes: ['A'],
      onlyFile: 'src/features/groups/Modal.svelte',
    });
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSources(f.files, f.opts ?? SELF_OPTS, v);
    const codeOf = (line) => /— \[([A-E])\]/.exec(line)?.[1];
    const got = new Set(v.map(codeOf));
    let ok;
    let expected;
    if (f.codes.length === 0) {
      ok = v.length === 0;
      expected = '0 нарушений';
    } else {
      ok = f.codes.every((c) => got.has(c)) && [...got].every((c) => f.codes.includes(c));
      if (ok && f.onlyFile) ok = v.every((line) => line.startsWith(f.onlyFile));
      expected = `правила ${f.codes.join('+')}`;
    }
    const gotCodes = [...got].sort().join('+') || '—';
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (ожидалось: ${expected}; поймано: ${gotCodes})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} из ${fixtures.length} фикстур не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — все ${fixtures.length} фикстур совпали с ожиданием.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-device-status-labels.mjs [--src=<ui-dir>] [--scope=<src/...>] | --selftest`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const argScope = args.find((a) => a.startsWith('--scope='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const scope = argScope ? argScope.slice('--scope='.length).replace(/\/*$/, '/') : null;

  const violations = [];
  checkRepo(root, scope, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
