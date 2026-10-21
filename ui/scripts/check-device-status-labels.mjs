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
//       Освобождение ПО ПРАВИЛУ (RULE_EXEMPT, не по файлу): в файле, где рядом
//       живут устройственная и картриджная половины (places/PlaceContents),
//       правило [B] снимается ТОЛЬКО с объявления `const CARTRIDGE_… = { … }`
//       (и ссылок на этот идентификатор), если в его ключах нет слов
//       устройственных статусов. Правила [A] и [C] в файле действуют целиком;
//   [E] таблица освобождений EXEMPT: не меньше MIN_EXEMPT записей, у каждой
//       причина не короче 20 символов, каталожный префикс заканчивается на `/`
//       (иначе `features/requests` освободил бы и `features/requests_old`), а
//       в полном режиме каждая запись обязана совпасть хотя бы с одним файлом
//       (устаревшее освобождение молча глушило бы гейт).
//
//   [F] карта тонов и места вызова Badge (план 41.1-30, GRD-04; решение
//       2026-10-17 отменило R-06: все шесть статусов — soft-чипы, ожидающий
//       статус отличается от действующего яркостью, а не формой):
//       [F1] литерал TONES в сторе разобран, ключи — ровно шесть кодов;
//       [F2] ни одна запись не `appearance: 'dot'`;
//       [F3] `muted: true` ровно у на_ремонте и на_списание;
//       [F4] пары на_ремонте/в_ремонте и на_списание/списано делят variant;
//       [F5] КАЖДЫЙ открывающий тег `<Badge` в файле-импортёре statusTone
//            обязан попасть ровно в одну категорию (план 41.11-04, WR-07, D-22):
//              A «Badge статуса» — в теге `statusTone(` либо ИМЯ., где ИМЯ —
//                переменная, объявленная как `const|let ИМЯ = $derived(statusTone(…))`
//                с ПРОИЗВОЛЬНЫМ именем (tone, chip, st…); variant, appearance и
//                muted обязаны быть из ОДНОГО источника, спред запрещён;
//              B «не статусный литерал» — variant/appearance/size/muted либо
//                отсутствуют, либо заданы строковым литералом (`variant="accent"`);
//              C «освобождён вердиктом» — запись в BADGE_EXEMPT (файл + маркер
//                в теге + причина не короче 20 символов); сегодня одна —
//                картриджный `cartridgeStatusVariant(`;
//            всё остальное — нарушение F5 «не отнесён ни к одной категории».
//            Охват строится от ФАЙЛА-ИМПОРТЁРА (обход репозитория), а не от слова
//            `tone` в теге. MIN_STATUS_BADGES — лишь нижняя граница числа
//            бейджей категории A, а не носитель охвата. [F5-vacuous] — мало
//            проверенных тегов/файлов, гейт пуст.
//
//   [F6] нецветовой признак различия внутри пары (план 41.11-09, D-16 / D-18 /
//        D-34): после правки токенов пара «на ремонте / в ремонте» (как и «на
//        списание / списано») по цвету не различается, поэтому различие обязана
//        нести обводка приглушённого чипа (WCAG 1.4.1). Гейт читает
//        src/lib/components/Badge.svelte (отсутствие файла — нарушение, не
//        пропуск) и по скобочному балансу разбирает блоки тонов warning/danger:
//          — `&.badge-m-soft.badge-m-muted { … }` обязан содержать box-shadow или
//            border на `var(--tr-<тон>-muted-border)`;
//          — тот же блок обязан содержать ветку `@media (forced-colors: active)`
//            с outline/border, значение которого не none / 0 / transparent
//            (в Windows high contrast inset box-shadow отбрасывается);
//          — `&.badge-m-soft { … }` (действующий статус) не содержит ни
//            box-shadow/border/outline, ни forced-colors.
//        Носителей различия внутри пары три, F6 — лишь один из них: инверсию
//        флагов muted ловит F3, обводку и её forced-colors-ветку — F6,
//        прозрачность подложки — R2 в check-contrast.mjs.
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
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-status-labels]';

const STORE_IMPORT = '$lib/stores/deviceStatuses.svelte';
const GENERATED = new Set(['src/bindings.ts']);

// Сколько файлов обязаны читать стор после перевода всех экранов (план 41.1-13).
const MIN_CONSUMERS = 9;
const SELFTEST_MIN_CONSUMERS = 3;
const MIN_EXEMPT = 7;
// Нижняя граница инвентаря [F5]: сегодня пять мест вызова в пяти файлах
// (DeviceListRow, DeviceGroupRow, PlaceContents, GroupContentsTable,
// GroupAddDevicesModal). Меньше — правило проверяет пустоту.
const MIN_STATUS_BADGES = 5;
const MIN_STATUS_BADGE_FILES = 5;
const SELFTEST_MIN_STATUS_BADGES = 2;
const SELFTEST_MIN_STATUS_BADGE_FILES = 2;
const STORE_PATH = 'src/lib/stores/deviceStatuses.svelte.ts';
const BADGE_PATH = 'src/lib/components/Badge.svelte';
const TONE_CODES = ['на_складе', 'в_работе', 'на_ремонте', 'в_ремонте', 'на_списание', 'списано'];
const MUTED_CODES = ['на_ремонте', 'на_списание'];
const TONE_PAIRS = [
  ['на_ремонте', 'в_ремонте', 'warning'],
  ['на_списание', 'списано', 'destructive'],
];
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
//     places/PlaceContents (устройственная половина; картриджная карта
//     освобождена ПО ПРАВИЛУ, таблица RULE_EXEMPT), acts/ActFormItemsTable;
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

// Освобождения ПО ПРАВИЛУ (а не по файлу): правило [B] снимается только с
// картриджной карты `const CARTRIDGE_… = { … }` (D-07). Всё остальное в файле,
// включая правила [A]/[C] и любую другую карту «имя -> тон», проверяется.
const RULE_EXEMPT = [
  {
    path: 'src/features/places/PlaceContents.svelte',
    rules: ['B'],
    reason:
      'картриджная карта CARTRIDGE_…_BY_NAME: статусы картриджей вне справочника устройств (D-07)',
  },
];

// Вердикты по <Badge> в файлах-импортёрах statusTone (категория C правила [F5],
// D-22). Освобождается не файл целиком, а ТЕГ: путь файла + маркер в теге.
// Единственная запись — картриджный бейдж: статусы картриджей вне справочника
// устройств (D-07). Пополнять таблицу ради «зелёного» гейта нельзя: бейдж,
// не попавший в категорию, — это и есть дефект, который правило ищет.
const BADGE_EXEMPT = [
  {
    path: 'src/features/places/PlaceContents.svelte',
    marker: 'cartridgeStatusVariant(',
    reason:
      'бейдж статуса КАРТРИДЖА: свой справочник из 4 статусов, вне справочника устройств (D-07)',
  },
];

// Слова статусов, которых у КАРТРИДЖЕЙ нет (cartridge_statuses: «На складе»,
// «В работе», «На заправке», «Списано»). Карта с таким ключом — устройственная,
// даже если названа CARTRIDGE_…
const DEVICE_ONLY_WORDS = ['Ожидает ремонта', 'На списание', 'На ремонте', 'В ремонте'];

/**
 * Объявления `const CARTRIDGE_… = { … }` в участке кода: имя и диапазон
 * [start, end] (включая идентификатор и тело). Объявление с ключом из
 * DEVICE_ONLY_WORDS допустимым не считается и в результат не попадает.
 */
function cartridgeMapRanges(part) {
  const ranges = [];
  for (const m of part.matchAll(/\bconst\s+(CARTRIDGE_\w+)\b[^=;]*=\s*\{/g)) {
    const open = m.index + m[0].length - 1;
    let depth = 0;
    let end = -1;
    for (let i = open; i < part.length; i++) {
      if (part[i] === '{') depth++;
      else if (part[i] === '}' && --depth === 0) {
        end = i;
        break;
      }
    }
    if (end < 0) continue;
    const body = part.slice(open, end + 1);
    if (DEVICE_ONLY_WORDS.some((w) => body.includes(w))) continue;
    ranges.push({ name: m[1], start: m.index, end });
  }
  return ranges;
}

/** Совпадение правила [B] разрешено, если оно внутри допустимой картриджной карты или ссылается на неё. */
function bMatchAllowed(ranges, index, text) {
  if (ranges.some((r) => index >= r.start && index <= r.end)) return true;
  return ranges.some((r) => text === r.name);
}

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
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
  const ruleExempt = opts.ruleExempt ?? [];
  const badgeExempt = opts.badgeExempt ?? [];
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

  // --- E: освобождения по правилу ---------------------------------------------
  for (const e of ruleExempt) {
    if ((e.reason ?? '').trim().length < 20) {
      add(e.path, 0, 'E', 'у освобождения по правилу нет внятной причины (не короче 20 символов).');
    }
    if (!scope) {
      const src = files.get(e.path);
      const valid =
        src !== undefined && codeParts(e.path, src).some((p) => cartridgeMapRanges(p).length > 0);
      if (!valid) {
        add(
          e.path,
          0,
          'E',
          'освобождение по правилу устарело: в файле нет допустимой карты const CARTRIDGE_… = { … }.',
        );
      }
    }
  }

  // --- E: вердикты по <Badge> (категория C правила F5) ---------------------------
  for (const e of badgeExempt) {
    if ((e.reason ?? '').trim().length < 20) {
      add(e.path, 0, 'E', 'у вердикта по Badge нет внятной причины (не короче 20 символов).');
    }
    if (!(e.marker ?? '').trim()) {
      add(
        e.path,
        0,
        'E',
        'у вердикта по Badge нет маркера в теге — освобождение стало бы файловым.',
      );
    }
  }
  const usedBadgeExempt = new Set();

  // --- A / B / C --------------------------------------------------------------
  let consumers = 0;
  for (const [file, src] of files) {
    if (GENERATED.has(file)) continue;
    if (src.includes(STORE_IMPORT)) consumers++;
    if (!inScope(file)) continue;
    if (exemptionFor(file, exempt)) continue;
    const byRule = ruleExempt.find((e) => e.path === file);
    for (const part of codeParts(file, src)) {
      const ranges = byRule ? cartridgeMapRanges(part) : [];
      for (const [rule, re, msg] of SCAN_RULES) {
        re.lastIndex = 0;
        for (const m of part.matchAll(re)) {
          if (rule === 'B' && byRule?.rules.includes('B') && bMatchAllowed(ranges, m.index, m[0])) {
            continue;
          }
          add(file, lineOf(part, m.index), rule, `${msg}: «${m[0].replace(/\s+/g, ' ').trim()}».`);
        }
      }
    }
  }

  // --- F ----------------------------------------------------------------------
  if (!scope) checkToneMap(files, add);
  if (!scope) checkMutedBorder(files, add);
  let badgeTags = 0;
  let badgeFiles = 0;
  for (const [file, src] of files) {
    if (GENERATED.has(file) || !inScope(file)) continue;
    const n = checkStatusBadges(file, src, add, { badgeExempt, usedBadgeExempt });
    badgeTags += n;
    if (n > 0) badgeFiles++;
  }
  if (!scope) {
    badgeExempt.forEach((e, i) => {
      if (!usedBadgeExempt.has(i)) {
        add(
          e.path,
          0,
          'E',
          `вердикт по Badge устарел: в файле нет тега с маркером «${e.marker}» — удалите запись.`,
        );
      }
    });
  }
  if (opts.stats) Object.assign(opts.stats, { badgeTags, badgeFiles });
  const minTags = opts.minStatusBadges ?? MIN_STATUS_BADGES;
  const minFiles = opts.minStatusBadgeFiles ?? MIN_STATUS_BADGE_FILES;
  if (!scope && (badgeTags < minTags || badgeFiles < minFiles)) {
    add(
      'src',
      0,
      'F5-vacuous',
      `проверено ${badgeTags} тегов Badge статуса в ${badgeFiles} файлах, ожидается не меньше ${minTags} в ${minFiles} — правило проверяет пустоту.`,
    );
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

/** Индекс закрывающей скобки для `{` в позиции open; строки внутри пропускаются. */
function readBalanced(s, open) {
  let depth = 0;
  for (let i = open; i < s.length; i++) {
    const c = s[i];
    if (c === '"' || c === "'" || c === '`') {
      const j = s.indexOf(c, i + 1);
      if (j < 0) return -1;
      i = j;
      continue;
    }
    if (c === '{') depth++;
    else if (c === '}' && --depth === 0) return i;
  }
  return -1;
}

/** Конец открывающего тега `<Badge …>` (индекс `>` на нулевой глубине) или -1. */
function findTagEnd(markup, start) {
  for (let i = start + '<Badge'.length; i < markup.length; i++) {
    const c = markup[i];
    if (c === '{') {
      const end = readBalanced(markup, i);
      if (end < 0) return -1;
      i = end;
    } else if (c === '"' || c === "'") {
      const j = markup.indexOf(c, i + 1);
      if (j < 0) return -1;
      i = j;
    } else if (c === '>') return i;
  }
  return -1;
}

/** Атрибуты верхнего уровня тега: Map имя -> { kind: 'expr' | 'str', value }, и признак спреда. */
function parseBadgeAttrs(tag) {
  const attrs = new Map();
  let spread = false;
  let i = '<Badge'.length;
  while (i < tag.length) {
    if (tag[i] === '{') {
      const end = readBalanced(tag, i);
      if (end < 0) break;
      if (/^\{\s*\.\.\./.test(tag.slice(i, end + 1))) spread = true;
      i = end + 1;
      continue;
    }
    const m = /^([\w:-]+)\s*=\s*/.exec(tag.slice(i));
    if (m && /\s/.test(tag[i - 1] ?? ' ')) {
      i += m[0].length;
      const v = tag[i];
      if (v === '{') {
        const end = readBalanced(tag, i);
        if (end < 0) break;
        attrs.set(m[1], { kind: 'expr', value: tag.slice(i + 1, end) });
        i = end + 1;
      } else if (v === '"' || v === "'") {
        const end = tag.indexOf(v, i + 1);
        if (end < 0) break;
        attrs.set(m[1], { kind: 'str', value: tag.slice(i + 1, end) });
        i = end + 1;
      } else {
        const t = /^[^\s>]*/.exec(tag.slice(i))[0];
        attrs.set(m[1], { kind: 'str', value: t });
        i += t.length;
      }
      continue;
    }
    i++;
  }
  return { attrs, spread };
}

/** Правила F1-F4: литерал TONES в сторе. */
function checkToneMap(files, add) {
  const src = files.get(STORE_PATH);
  if (src === undefined) {
    add(STORE_PATH, 0, 'F1', 'файл стора не найден — карта тонов не проверена.');
    return;
  }
  const code = stripComments(src);
  const decl = /\bconst\s+TONES\b[^=]*=\s*\{/.exec(code);
  if (!decl) {
    add(STORE_PATH, 0, 'F1', 'литерал `const TONES = { … }` не найден.');
    return;
  }
  const open = decl.index + decl[0].length - 1;
  const end = readBalanced(code, open);
  if (end < 0) {
    add(STORE_PATH, 0, 'F1', 'литерал TONES не закрыт.');
    return;
  }
  const body = code.slice(open + 1, end);
  const entries = new Map();
  const re = /(?:^|[,{\s])(['"]?)([\p{L}\p{N}_$]+)\1\s*:\s*\{([^{}]*)\}/gu;
  for (const m of body.matchAll(re)) {
    const text = m[3];
    entries.set(m[2], {
      variant: /\bvariant\s*:\s*['"](\w+)['"]/.exec(text)?.[1] ?? null,
      appearance: /\bappearance\s*:\s*['"](\w+)['"]/.exec(text)?.[1] ?? null,
      muted: /\bmuted\s*:\s*true\b/.test(text),
      line: lineOf(code, open + 1 + m.index),
    });
  }
  for (const c of TONE_CODES) {
    if (!entries.has(c)) add(STORE_PATH, 0, 'F1', `в карте TONES нет кода «${c}».`);
  }
  for (const c of entries.keys()) {
    if (!TONE_CODES.includes(c)) add(STORE_PATH, 0, 'F1', `в карте TONES лишний код «${c}».`);
  }
  for (const [c, e] of entries) {
    if (e.appearance === 'dot') {
      add(STORE_PATH, e.line, 'F2', `«${c}» — appearance: 'dot'; все шесть статусов — soft-чипы.`);
    }
    if (!TONE_CODES.includes(c)) continue;
    const want = MUTED_CODES.includes(c);
    if (e.muted !== want) {
      add(
        STORE_PATH,
        e.line,
        'F3',
        want
          ? `у «${c}» нет muted: true — ожидающий статус сольётся с действующим.`
          : `у «${c}» стоит muted: true — приглушён должен быть только ожидающий статус.`,
      );
    }
  }
  for (const [a, b, variant] of TONE_PAIRS) {
    const ea = entries.get(a);
    const eb = entries.get(b);
    if (!ea || !eb) continue;
    if (ea.variant !== variant || eb.variant !== variant) {
      add(
        STORE_PATH,
        ea.line,
        'F4',
        `пара «${a}»/«${b}» должна делить variant '${variant}' (сейчас ${ea.variant}/${eb.variant}); различие пары — только muted.`,
      );
    }
  }
}

/**
 * Имена переменных, объявленных как `const|let ИМЯ = $derived(statusTone(…))` (имя
 * произвольное). Инициализатор может быть и составным (`$derived(cond ? statusTone(a)
 * : statusTone(b))`): достаточно вызова statusTone( до конца оператора.
 */
function statusToneNames(script) {
  const names = new Set();
  for (const m of script.matchAll(
    /\b(?:const|let)\s+(\w+)\s*(?::[^=;\n]+)?=[^;]*?\bstatusTone\(/g,
  )) {
    names.add(m[1]);
  }
  return [...names];
}

/**
 * Правило F5 для одного файла: возвращает число проверенных тегов категории A.
 * Каждый `<Badge` в файле-импортёре statusTone обязан попасть в категорию
 * A (статус) / B (литерал) / C (вердикт BADGE_EXEMPT), иначе — нарушение F5.
 */
function checkStatusBadges(file, src, add, ctx = {}) {
  if (!file.endsWith('.svelte') || file === STORE_PATH) return 0;
  const { script, markup } = splitSvelte(src);
  const imp =
    /import\s*\{[^}]*\bstatusTone\b[^}]*\}\s*from\s*['"]\$lib\/stores\/deviceStatuses\.svelte['"]/;
  if (!imp.test(script)) return 0;
  const badgeExempt = ctx.badgeExempt ?? [];
  const names = statusToneNames(script);
  const nameRe = names.length > 0 ? new RegExp(`(?<![\\w$.])(?:${names.join('|')})\\??\\.`) : null;
  let checked = 0;
  for (const m of markup.matchAll(/<Badge\b/g)) {
    const end = findTagEnd(markup, m.index);
    if (end < 0) continue;
    const tag = markup.slice(m.index, end + 1);
    const line = lineOf(markup, m.index);
    const { attrs, spread } = parseBadgeAttrs(tag);
    if (spread) {
      // Спред от источника статуса считается проверенным тегом (как раньше, до WR-07).
      if (/statusTone\(/.test(tag) || names.some((n) => new RegExp(`\\b${n}\\b`).test(tag)))
        checked++;
      add(file, line, 'F5', 'спред {...} на Badge в файле статуса — форма не поддаётся проверке.');
      continue;
    }
    const isStatus = /statusTone\(/.test(tag) || (nameRe !== null && nameRe.test(tag));
    if (!isStatus) {
      // Категория C: вердикт (файл + маркер в теге).
      const ei = badgeExempt.findIndex((e) => e.path === file && tag.includes(e.marker));
      if (ei >= 0) {
        ctx.usedBadgeExempt?.add(ei);
        continue;
      }
      // Категория B: все управляющие атрибуты — строковые литералы (или их нет).
      const literal = ['variant', 'appearance', 'size', 'muted'].every((k) => {
        const a = attrs.get(k);
        return !a || a.kind === 'str';
      });
      if (literal) continue;
      add(
        file,
        line,
        'F5',
        'Badge в файле-импортёре статуса не отнесён ни к одной категории (статус / литерал / освобождён вердиктом): выражение в variant/appearance/size/muted не от statusTone.',
      );
      continue;
    }
    checked++;
    const norm = (name, suffix) => {
      const a = attrs.get(name);
      if (!a || a.kind !== 'expr') return null;
      const v = a.value.replace(/\s+/g, '');
      return v.endsWith(suffix) ? v.slice(0, -suffix.length) : null;
    };
    const sources = {
      variant: norm('variant', '.variant'),
      appearance: norm('appearance', '.appearance'),
      muted: norm('muted', '.muted'),
    };
    const missing = Object.keys(sources).filter((k) => sources[k] === null);
    if (missing.length > 0) {
      add(
        file,
        line,
        'F5',
        `Badge статуса: нет ${missing.join(', ')} из источника тона (ожидается ${missing.map((k) => `${k}={<источник>.${k}}`).join(', ')}).`,
      );
      continue;
    }
    if (sources.variant !== sources.appearance || sources.variant !== sources.muted) {
      add(
        file,
        line,
        'F5',
        `Badge статуса: variant/appearance/muted берутся из разных источников (${sources.variant} / ${sources.appearance} / ${sources.muted}).`,
      );
    }
  }
  return checked;
}

// CSS-комментарии затираются с сохранением длины и переводов строк.
function stripCssComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, blank);
}

/** Значение outline/border «пустое»: none, 0, transparent — признак не виден. */
function isInvisibleBorderValue(value) {
  const tokens = value.trim().toLowerCase().split(/\s+/);
  return (
    tokens.some((t) => t === 'none' || t === 'transparent') || /^0(px|em|rem)?$/.test(tokens[0])
  );
}

/**
 * Правило F6: нецветовой признак различия внутри пары в Badge.svelte (D-16, D-34).
 * Разбор по скобочному балансу в <style>; комментарии затираются.
 */
function checkMutedBorder(files, add) {
  const src = files.get(BADGE_PATH);
  if (src === undefined) {
    add(
      BADGE_PATH,
      0,
      'F6',
      'файл Badge.svelte не найден — рамка приглушённого чипа не проверена.',
    );
    return;
  }
  const style = /<style\b[^>]*>([\s\S]*?)<\/style>/.exec(src);
  if (!style) {
    add(
      BADGE_PATH,
      0,
      'F6',
      'в Badge.svelte нет блока <style> — рамка приглушённого чипа не проверена.',
    );
    return;
  }
  const css = stripCssComments(style[1]);
  const blockBody = (text, headerRe) => {
    const all = [...text.matchAll(new RegExp(headerRe, 'g'))];
    if (all.length !== 1) return { count: all.length };
    const open = all[0].index + all[0][0].length - 1;
    const end = readBalanced(text, open);
    if (end < 0) return { count: 1, body: null };
    return { count: 1, body: text.slice(open + 1, end) };
  };
  for (const tone of ['warning', 'danger']) {
    const toneBlock = blockBody(css, `\\.badge-m-${tone}\\s*\\{`);
    if (toneBlock.body == null) {
      add(
        BADGE_PATH,
        0,
        'F6',
        `блок .badge-m-${tone} не найден единственным (найдено: ${toneBlock.count}) — рамка не проверена.`,
      );
      continue;
    }
    const muted = blockBody(toneBlock.body, '&\\.badge-m-soft\\.badge-m-muted\\s*\\{');
    if (muted.body == null) {
      add(
        BADGE_PATH,
        0,
        'F6',
        `в .badge-m-${tone} нет единственного блока &.badge-m-soft.badge-m-muted (найдено: ${muted.count}) — рамка не проверена.`,
      );
    } else {
      // Объявления самого блока — без вложенных at-правил.
      const mediaRe = /@media\s*\(\s*forced-colors\s*:\s*active\s*\)\s*\{/g;
      const media = mediaRe.exec(muted.body);
      let own = muted.body;
      let mediaBody = null;
      if (media) {
        const open = media.index + media[0].length - 1;
        const end = readBalanced(muted.body, open);
        if (end > 0) {
          mediaBody = muted.body.slice(open + 1, end);
          own = muted.body.slice(0, media.index) + muted.body.slice(end + 1);
        }
      }
      const token = `var(--tr-${tone}-muted-border)`;
      const hasBorder = [...own.matchAll(/(?:^|[;\s])(box-shadow|border)\s*:\s*([^;]+);/g)].some(
        (m) => m[2].includes(token),
      );
      if (!hasBorder) {
        add(
          BADGE_PATH,
          0,
          'F6',
          `у приглушённого ${tone}-чипа нет рамки на ${token}: различие пары осталось бы только оттенком (WCAG 1.4.1).`,
        );
      }
      const hasForced =
        mediaBody !== null &&
        [...mediaBody.matchAll(/(?:^|[;\s{])(outline|border)\s*:\s*([^;]+);/g)].some(
          (m) => !isInvisibleBorderValue(m[2]),
        );
      if (!hasForced) {
        add(
          BADGE_PATH,
          0,
          'F6',
          `у приглушённого ${tone}-чипа нет ветки @media (forced-colors: active) с видимым outline/border: в Windows high contrast inset box-shadow отбрасывается и признак пары пропадает.`,
        );
      }
    }
    const plain = blockBody(toneBlock.body, '&\\.badge-m-soft\\s*\\{');
    if (plain.body == null) {
      add(
        BADGE_PATH,
        0,
        'F6',
        `в .badge-m-${tone} нет единственного блока &.badge-m-soft (найдено: ${plain.count}).`,
      );
    } else if (
      /(?:^|[;\s{])(?:box-shadow|border|outline)[\w-]*\s*:|forced-colors/.test(plain.body)
    ) {
      add(
        BADGE_PATH,
        0,
        'F6',
        `у действующего ${tone}-чипа (&.badge-m-soft без muted) появилась рамка: различие пары исчезло бы — рамка только у приглушённого.`,
      );
    }
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
  const stats = {};
  scanSources(
    files,
    {
      stats,
      scope,
      exempt: EXEMPT,
      ruleExempt: RULE_EXEMPT,
      badgeExempt: BADGE_EXEMPT,
      minConsumers: MIN_CONSUMERS,
      minExempt: MIN_EXEMPT,
    },
    violations,
  );
  if (stats.badgeTags !== undefined) {
    console.error(
      `${TAG} [F5] проверено тегов Badge статуса: ${stats.badgeTags} в файлах: ${stats.badgeFiles}`,
    );
  }
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
<Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>
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

// Экран «Места»: устройства и принтеры — по стору, картриджи — своя карта
// (D-07), освобождённая по правилу, а не по файлу.
const CLEAN_PLACES = `<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import { statusTone } from '${STORE_IMPORT}';
  const CARTRIDGE_STATUS_VARIANT_BY_NAME: Record<string, 'default' | 'accent' | 'warning'> = {
    'На складе': 'default',
    'В работе': 'accent',
    'На заправке': 'warning',
  };
  const { row } = $props();
  const cv = CARTRIDGE_STATUS_VARIANT_BY_NAME[row.status_name] ?? 'default';
  function cartridgeStatusVariant(v: string) {
    return v;
  }
</script>

{#if row.kind === 'cartridge'}
  <Badge variant={cartridgeStatusVariant(cv)}>{row.status_name}</Badge>
{:else}
  <Badge
    variant={statusTone(row.status_code).variant}
    appearance={statusTone(row.status_code).appearance}
    muted={statusTone(row.status_code).muted}>{row.status_name}</Badge
  >
{/if}
`;

// Стор: единственная таблица тонов (правила F1-F4).
const CLEAN_STORE = `// Тоны по code статуса: все шесть — soft-чипы, ожидающие приглушены.
export interface StatusTone {
  variant: string;
  appearance: string;
  muted?: boolean;
}
const TONES: Record<string, StatusTone> = {
  на_складе: { variant: 'default', appearance: 'soft' },
  в_работе: { variant: 'accent', appearance: 'soft' },
  на_ремонте: { variant: 'warning', appearance: 'soft', muted: true },
  в_ремонте: { variant: 'warning', appearance: 'soft' },
  на_списание: { variant: 'destructive', appearance: 'soft', muted: true },
  списано: { variant: 'destructive', appearance: 'soft' },
};
export const statusTone = (code: string) => TONES[code];
`;

const MILESTONE_ROW = `<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
  import { statusTone } from '${STORE_IMPORT}';
  const milestone = { variant: 'default' };
  const unused = statusTone;
</script>

<Badge variant={milestone.variant}>этап</Badge>
`;

// Badge для правила F6: та же структура блоков, что у реального Badge.svelte.
const CLEAN_BADGE = `<script lang="ts">
  const { muted = false } = $props();
</script>

<span class="badge-m badge-m-warning badge-m-soft" class:badge-m-muted={muted}></span>

<style lang="scss">
  .badge-m-warning {
    &.badge-m-soft {
      background: var(--tr-warning-soft);
      color: var(--tr-warning-text);
    }
    /* комментарий с forced-colors и box-shadow: гейт его не учитывает */
    &.badge-m-soft.badge-m-muted {
      background: var(--tr-warning-muted-soft);
      color: var(--tr-warning-muted-text);
      box-shadow: inset 0 0 0 1px var(--tr-warning-muted-border);
      @media (forced-colors: active) {
        outline: 1px solid CanvasText;
        outline-offset: -1px;
      }
    }
    &.badge-m-count {
      border-color: var(--tr-warning);
    }
  }
  .badge-m-danger {
    &.badge-m-soft {
      background: var(--tr-danger-soft);
      color: var(--tr-danger-text);
    }
    &.badge-m-soft.badge-m-muted {
      background: var(--tr-danger-muted-soft);
      color: var(--tr-danger-muted-text);
      box-shadow: inset 0 0 0 1px var(--tr-danger-muted-border);
      @media (forced-colors: active) {
        outline: 1px solid CanvasText;
        outline-offset: -1px;
      }
    }
  }
</style>
`;

const PATH_PLACES = 'src/features/places/Contents.svelte';
const SELF_RULE_EXEMPT = [
  {
    path: PATH_PLACES,
    rules: ['B'],
    reason: 'картриджная карта CARTRIDGE_…: статусы картриджей вне справочника устройств',
  },
];
const SELF_BADGE_EXEMPT = [
  {
    path: PATH_PLACES,
    marker: 'cartridgeStatusVariant(',
    reason: 'бейдж статуса картриджа: свой справочник, вне справочника устройств',
  },
];
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
    [PATH_PLACES, over.places ?? CLEAN_PLACES],
    [STORE_PATH, over.store ?? CLEAN_STORE],
    [BADGE_PATH, over.badge ?? CLEAN_BADGE],
    ...(over.extra ?? []),
  ]);
}

const SELF_OPTS = {
  scope: null,
  exempt: SELF_EXEMPT,
  ruleExempt: SELF_RULE_EXEMPT,
  badgeExempt: SELF_BADGE_EXEMPT,
  minConsumers: SELFTEST_MIN_CONSUMERS,
  minExempt: SELFTEST_MIN_EXEMPT,
  minStatusBadges: SELFTEST_MIN_STATUS_BADGES,
  minStatusBadgeFiles: SELFTEST_MIN_STATUS_BADGE_FILES,
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
      // Остался один экран с Badge статуса: инвентарь [F5] тоже ниже границы.
      codes: ['D', 'F5-vacuous'],
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
    // --- Освобождение по правилу: картриджная карта в файле «Места» ----------
    fixtures.push({
      name: 'M15 [B] устройственная карта STATUS_VARIANT_BY_NAME рядом с картриджной в файле «Места»',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '  const { row } = $props();\n',
          "  const { row } = $props();\n  const STATUS_VARIANT_BY_NAME = { 'На складе': 'default', 'В ремонте': 'warning' };\n",
        ),
      }),
      codes: ['B'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M16 [B] устройственная карта без суффикса _BY_NAME (имя TONE) в файле «Места»',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '  const { row } = $props();\n',
          "  const { row } = $props();\n  const TONE = { 'На складе': 'default', Списано: 'destructive' };\n",
        ),
      }),
      codes: ['B'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M17 [B] CARTRIDGE_-карта с устройственным ключом «Ожидает ремонта» не освобождается',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          "    'На заправке': 'warning',\n",
          "    'На заправке': 'warning',\n    'Ожидает ремонта': 'warning',\n",
        ),
      }),
      codes: ['B', 'E'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M18 [B] CARTRIDGE_-карта с прежним устройственным ключом «На ремонте» не освобождается',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          "    'На заправке': 'warning',\n",
          "    'На заправке': 'warning',\n    'На ремонте': 'warning',\n",
        ),
      }),
      codes: ['B', 'E'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M19 [B] карта без префикса CARTRIDGE_ (переименование картриджной карты) краснеет',
      files: fixtureFiles({
        places: CLEAN_PLACES.replaceAll('CARTRIDGE_STATUS_VARIANT_BY_NAME', 'STATUS_TONES'),
      }),
      codes: ['B', 'E'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M20 [A] карта «id -> подпись» в файле «Места»: освобождено только правило B',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '  const { row } = $props();\n',
          "  const { row } = $props();\n  const L: Record<number, string> = { 1: 'На складе' };\n",
        ),
      }),
      codes: ['A'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M21 [C] status_id === 1 в файле «Места»: освобождено только правило B',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '  const { row } = $props();\n',
          '  const { row } = $props();\n  const storage = row.status_id === 1;\n',
        ),
      }),
      codes: ['C'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
      // Блочно-первая вычистка стирала бы `status_id === 1` между
      // `templates/*.html` и ближайшим `*/` ниже — правило C молчало бы.
      name: 'M21b [C] `/*` в строчном комментарии не прячет status_id === 1 в файле «Места»',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '  const { row } = $props();\n',
          '  const { row } = $props();\n' +
            '  // шаблоны лежат в templates/*.html\n' +
            '  const storage = row.status_id === 1;\n' +
            '  /* настоящий блочный комментарий */\n',
        ),
      }),
      codes: ['C'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M22 [E] освобождение по правилу устарело: в файле нет картриджной карты',
      files: fixtureFiles({
        places: mutate(
          mutate(
            CLEAN_PLACES,
            /const CARTRIDGE_STATUS_VARIANT_BY_NAME[\s\S]*?\};\n/.exec(CLEAN_PLACES)[0],
            '',
          ),
          "  const cv = CARTRIDGE_STATUS_VARIANT_BY_NAME[row.status_name] ?? 'default';\n",
          "  const cv = 'default';\n",
        ),
      }),
      codes: ['E'],
    });
    fixtures.push({
      name: 'M23 [B] ссылка на CARTRIDGE_-карту, объявленную с устройственным ключом, тоже краснеет (объявление стоит ниже ссылки на неё)',
      files: fixtureFiles({
        places: (() => {
          const decl = / {2}const CARTRIDGE_STATUS_VARIANT_BY_NAME[\s\S]*?\};\n/.exec(
            CLEAN_PLACES,
          )[0];
          const moved = mutate(CLEAN_PLACES, decl, '');
          return mutate(
            moved,
            '</script>',
            decl.replace("'В работе': 'accent',", "'В ремонте': 'warning',") + '</script>',
          );
        })(),
      }),
      codes: ['B', 'E'],
    });

    // --- Правило F: карта тонов и места вызова Badge -------------------------
    const ROW_TAG =
      '<Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>';
    const PLACES_MUTED = '    muted={statusTone(row.status_code).muted}>{row.status_name}</Badge\n';
    // Было F-pos с ожиданием «0 нарушений»: фикстура закрепляла ту самую слепоту,
    // которую лечит WR-07 (Badge в файле-импортёре statusTone, не отнесённый ни к
    // одной категории, пропускался). Теперь это мутант. Освобождение для
    // `milestone` в BADGE_EXEMPT добавлять ЗАПРЕЩЕНО (D-22).
    fixtures.push({
      name: 'M36 [F5] Badge в файле-импортёре statusTone с выражением variant не от статуса (milestone.variant) не отнесён ни к одной категории',
      files: fixtureFiles({
        extra: [['src/features/devices/Milestone.svelte', MILESTONE_ROW]],
      }),
      codes: ['F5'],
      onlyFile: 'src/features/devices/Milestone.svelte',
    });
    fixtures.push({
      name: 'F-pos [F5] категория B: Badge с литеральными variant/appearance/size в файле статуса не краснит',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          ROW_TAG,
          ROW_TAG +
            '\n<Badge variant="accent" appearance="count" size="sm">3 шт.</Badge>\n<Badge>без атрибутов</Badge>',
        ),
      }),
      codes: [],
    });
    fixtures.push({
      name: 'F-pos [F5] произвольное имя источника: chip = $derived(statusTone(...)) с согласованными атрибутами',
      files: fixtureFiles({
        row: mutate(
          mutate(CLEAN_ROW, 'const tone = $derived', 'const chip = $derived'),
          ROW_TAG,
          '<Badge variant={chip.variant} appearance={chip.appearance} muted={chip.muted}>{label}</Badge>',
        ),
      }),
      codes: [],
    });
    fixtures.push({
      name: 'M37 [F5] источник под именем chip (в теге нет слова tone): нет muted',
      files: fixtureFiles({
        row: mutate(
          mutate(CLEAN_ROW, 'const tone = $derived', 'const chip = $derived'),
          ROW_TAG,
          '<Badge variant={chip.variant} appearance={chip.appearance}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M38 [F5] разные источники при произвольных именах: chip.variant + tone.appearance/muted',
      files: fixtureFiles({
        row: mutate(
          mutate(
            CLEAN_ROW,
            '  const tone = $derived(statusTone(device.status_code));\n',
            '  const tone = $derived(statusTone(device.status_code));\n  const chip = $derived(statusTone(device.status_code));\n',
          ),
          ROW_TAG,
          '<Badge variant={chip.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M39 [F5] Badge с выражением variant без литералов и без statusTone не отнесён ни к одной категории',
      files: fixtureFiles({
        row: mutate(CLEAN_ROW, ROW_TAG, ROW_TAG + '\n<Badge variant={someOther}>{label}</Badge>'),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M40 [F5] cartridgeStatusVariant( в ЧУЖОМ файле не освобождён (вердикт привязан к файлу)',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          ROW_TAG,
          ROW_TAG + '\n<Badge variant={cartridgeStatusVariant(label)}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M41 [E] вердикт по Badge устарел: в файле нет тега с маркером',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '<Badge variant={cartridgeStatusVariant(cv)}>',
          '<Badge variant="default">',
        ),
      }),
      codes: ['E'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: "M24 [F2] appearance: 'dot' возвращён в на_ремонте",
      files: fixtureFiles({
        store: mutate(
          CLEAN_STORE,
          "  на_ремонте: { variant: 'warning', appearance: 'soft', muted: true },",
          "  на_ремонте: { variant: 'warning', appearance: 'dot', muted: true },",
        ),
      }),
      codes: ['F2'],
    });
    fixtures.push({
      name: 'M25 [F3] muted снят у на_списание',
      files: fixtureFiles({
        store: mutate(
          CLEAN_STORE,
          "  на_списание: { variant: 'destructive', appearance: 'soft', muted: true },",
          "  на_списание: { variant: 'destructive', appearance: 'soft' },",
        ),
      }),
      codes: ['F3'],
    });
    fixtures.push({
      name: 'M26 [F3] muted поставлен действующему в_ремонте',
      files: fixtureFiles({
        store: mutate(
          CLEAN_STORE,
          "  в_ремонте: { variant: 'warning', appearance: 'soft' },",
          "  в_ремонте: { variant: 'warning', appearance: 'soft', muted: true },",
        ),
      }),
      codes: ['F3'],
    });
    fixtures.push({
      name: "M27 [F4] variant у списано заменён на 'warning'",
      files: fixtureFiles({
        store: mutate(
          CLEAN_STORE,
          "  списано: { variant: 'destructive', appearance: 'soft' },",
          "  списано: { variant: 'warning', appearance: 'soft' },",
        ),
      }),
      codes: ['F4'],
    });
    fixtures.push({
      name: 'M28 [F1] шестой код (списано) удалён из карты',
      files: fixtureFiles({
        store: mutate(
          CLEAN_STORE,
          "  списано: { variant: 'destructive', appearance: 'soft' },\n",
          '',
        ),
      }),
      codes: ['F1'],
    });
    fixtures.push({
      name: 'M29 [F5] у Badge в строке списка удалён muted={tone.muted}',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          ROW_TAG,
          '<Badge variant={tone.variant} appearance={tone.appearance}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M30 [F5] в «Местах» muted из чужого источника (tone.muted вместо statusTone(row.status_code).muted)',
      files: fixtureFiles({
        places: mutate(
          CLEAN_PLACES,
          '    muted={statusTone(row.status_code).muted}>',
          '    muted={tone.muted}>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_PLACES,
    });
    fixtures.push({
      name: 'M31 [F5] тег с variant от statusTone без appearance и без muted',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          ROW_TAG,
          ROW_TAG + '\n<Badge variant={statusTone(device.status_code).variant}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M32 [F5] тег со спредом {...tone}',
      files: fixtureFiles({
        row: mutate(CLEAN_ROW, ROW_TAG, '<Badge {...tone}>{label}</Badge>'),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M33 [F5] muted литералом muted={true} вместо источника тона',
      files: fixtureFiles({
        row: mutate(
          CLEAN_ROW,
          ROW_TAG,
          '<Badge variant={tone.variant} appearance={tone.appearance} muted={true}>{label}</Badge>',
        ),
      }),
      codes: ['F5'],
      onlyFile: PATH_ROW,
    });
    fixtures.push({
      name: 'M34 [F5-vacuous] ни одного Badge от statusTone (импорты остались, теги заменены)',
      files: fixtureFiles({
        row: mutate(CLEAN_ROW, ROW_TAG, '<span>{label}</span>'),
        places: mutate(
          mutate(
            CLEAN_PLACES,
            '    variant={statusTone(row.status_code).variant}\n    appearance={statusTone(row.status_code).appearance}\n',
            '',
          ),
          PLACES_MUTED,
          '    >{row.status_name}</Badge\n',
        ),
      }),
      codes: ['F5-vacuous'],
    });
    // --- Мутанты по РЕАЛЬНОМУ исходнику (D-25а): настоящий DeviceListRow.svelte ---
    // Якоря уникальны в реальном файле (mutate() бросает при ≠1 вхождении, D-26).
    const REAL_ROW_PATH = 'src/features/devices/DeviceListRow.svelte';
    const REAL_ROW = fs.readFileSync(path.join(UI_ROOT, REAL_ROW_PATH), 'utf8');
    const REAL_TONE_DECL = '  const tone = $derived(statusTone(device.status_code));';
    const REAL_BADGE =
      '      <Badge variant={tone.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>';
    const CHIP_DECL = REAL_TONE_DECL + '\n  const chip = $derived(statusTone(device.status_code));';
    const realMutant = (decl, badge) =>
      fixtureFiles({
        extra: [[REAL_ROW_PATH, mutate(mutate(REAL_ROW, REAL_TONE_DECL, decl), REAL_BADGE, badge)]],
      });
    fixtures.push({
      name: 'R0 [F5] немутированный реальный DeviceListRow.svelte — ноль нарушений',
      files: fixtureFiles({ extra: [[REAL_ROW_PATH, REAL_ROW]] }),
      codes: [],
    });
    fixtures.push({
      name: 'R1 [F5] реальный DeviceListRow: шестой Badge от chip = $derived(statusTone(...)) без muted (в теге нет слова tone)',
      files: realMutant(
        CHIP_DECL,
        REAL_BADGE +
          '\n      <Badge variant={chip.variant} appearance={chip.appearance}>{label}</Badge>',
      ),
      codes: ['F5'],
      onlyFile: REAL_ROW_PATH,
    });
    fixtures.push({
      name: 'R2 [F5] реальный DeviceListRow: шестой Badge с разными источниками chip.variant / tone.appearance / tone.muted',
      files: realMutant(
        CHIP_DECL,
        REAL_BADGE +
          '\n      <Badge variant={chip.variant} appearance={tone.appearance} muted={tone.muted}>{label}</Badge>',
      ),
      codes: ['F5'],
      onlyFile: REAL_ROW_PATH,
    });
    fixtures.push({
      name: 'R3 [F5] реальный DeviceListRow: шестой Badge variant={someOther} не отнесён ни к одной категории',
      files: realMutant(
        REAL_TONE_DECL,
        REAL_BADGE + '\n      <Badge variant={someOther}>{label}</Badge>',
      ),
      codes: ['F5'],
      onlyFile: REAL_ROW_PATH,
    });
    fixtures.push({
      name: 'R4 [F5] реальный DeviceListRow: шестой Badge от chip с тремя атрибутами из одного источника — ноль нарушений',
      files: realMutant(
        CHIP_DECL,
        REAL_BADGE +
          '\n      <Badge variant={chip.variant} appearance={chip.appearance} muted={chip.muted}>{label}</Badge>',
      ),
      codes: [],
    });
    fixtures.push({
      name: 'M35 [F1] файл стора отсутствует в дереве',
      files: (() => {
        const f = fixtureFiles();
        f.delete(STORE_PATH);
        return f;
      })(),
      codes: ['F1'],
    });
    // --- Мутанты правила F6 по РЕАЛЬНОМУ Badge.svelte (D-25а, D-26) -------------
    // Якоря многострочные и привязаны к строке с токеном тона: голая ветка
    // `@media (forced-colors: active) {` встречается дважды и в якорях не
    // используется; mutate() бросает при числе вхождений ≠ 1.
    const REAL_BADGE_SRC = fs.readFileSync(path.join(UI_ROOT, BADGE_PATH), 'utf8');
    const shadow = (tone) => `      box-shadow: inset 0 0 0 1px var(--tr-${tone}-muted-border);\n`;
    const FORCED = '      @media (forced-colors: active) {\n';
    const BRANCH = `${FORCED}        outline: 1px solid CanvasText;\n        outline-offset: -1px;\n      }\n`;
    const badgeMutant = (badge) => fixtureFiles({ badge });
    fixtures.push({
      name: 'F6-R0 немутированный реальный Badge.svelte — ноль нарушений',
      files: badgeMutant(REAL_BADGE_SRC),
      codes: [],
    });
    fixtures.push({
      name: 'F6-1 реальный Badge: рамка убрана из warning-блока muted',
      files: badgeMutant(mutate(REAL_BADGE_SRC, shadow('warning'), '')),
      codes: ['F6'],
      msgIncludes: 'рамк',
      msgExcludes: 'forced-colors',
    });
    fixtures.push({
      name: 'F6-2 реальный Badge: рамка убрана из danger-блока muted',
      files: badgeMutant(mutate(REAL_BADGE_SRC, shadow('danger'), '')),
      codes: ['F6'],
      msgIncludes: 'рамк',
      msgExcludes: 'forced-colors',
    });
    fixtures.push({
      name: 'F6-3 реальный Badge: рамка добавлена действующему (немутированному) soft-чипу warning',
      files: badgeMutant(
        mutate(
          REAL_BADGE_SRC,
          '    &.badge-m-soft {\n      background: var(--tr-warning-soft);\n',
          '    &.badge-m-soft {\n      box-shadow: inset 0 0 0 1px var(--tr-warning-muted-border);\n      background: var(--tr-warning-soft);\n',
        ),
      ),
      codes: ['F6'],
      msgIncludes: 'рамк',
      msgExcludes: 'forced-colors',
    });
    fixtures.push({
      name: 'F6-4 реальный Badge: ветка forced-colors снята из warning-блока muted (box-shadow на месте)',
      files: badgeMutant(mutate(REAL_BADGE_SRC, shadow('warning') + BRANCH, shadow('warning'))),
      codes: ['F6'],
      msgIncludes: 'forced-colors',
      msgExcludes: 'рамк',
    });
    fixtures.push({
      name: 'F6-5 реальный Badge: ветка forced-colors снята из danger-блока muted (box-shadow на месте)',
      files: badgeMutant(mutate(REAL_BADGE_SRC, shadow('danger') + BRANCH, shadow('danger'))),
      codes: ['F6'],
      msgIncludes: 'forced-colors',
      msgExcludes: 'рамк',
    });
    const OUTLINE = '        outline: 1px solid CanvasText;\n';
    fixtures.push({
      name: 'F6-6 реальный Badge: в forced-colors warning-блока outline: none (свойство на месте, признак пропал)',
      files: badgeMutant(
        mutate(
          REAL_BADGE_SRC,
          shadow('warning') + FORCED + OUTLINE,
          shadow('warning') + FORCED + '        outline: none;\n',
        ),
      ),
      codes: ['F6'],
      msgIncludes: 'forced-colors',
      msgExcludes: 'рамк',
    });
    fixtures.push({
      name: 'F6-7 реальный Badge: в forced-colors danger-блока outline: 0',
      files: badgeMutant(
        mutate(
          REAL_BADGE_SRC,
          shadow('danger') + FORCED + OUTLINE,
          shadow('danger') + FORCED + '        outline: 0;\n',
        ),
      ),
      codes: ['F6'],
      msgIncludes: 'forced-colors',
      msgExcludes: 'рамк',
    });
    fixtures.push({
      name: 'F6-8 реальный Badge: в forced-colors danger-блока outline: 1px solid transparent',
      files: badgeMutant(
        mutate(
          REAL_BADGE_SRC,
          shadow('danger') + FORCED + OUTLINE,
          shadow('danger') + FORCED + '        outline: 1px solid transparent;\n',
        ),
      ),
      codes: ['F6'],
      msgIncludes: 'forced-colors',
      msgExcludes: 'рамк',
    });
    fixtures.push({
      name: 'F6-9 Badge.svelte отсутствует в дереве',
      files: (() => {
        const f = fixtureFiles();
        f.delete(BADGE_PATH);
        return f;
      })(),
      codes: ['F6'],
    });
    // --- Связка D-29a: инверсию пары ловит F3, а не F6 -------------------------
    const REAL_STORE_SRC = fs.readFileSync(path.join(UI_ROOT, STORE_PATH), 'utf8');
    fixtures.push({
      name: 'F3-инверсия реальный стор: muted переехал с на_ремонте на в_ремонте (пара инвертирована) — ловит F3, не F6',
      files: fixtureFiles({
        store: mutate(
          mutate(
            REAL_STORE_SRC,
            "  на_ремонте: { variant: 'warning', appearance: 'soft', muted: true },",
            "  на_ремонте: { variant: 'warning', appearance: 'soft' },",
          ),
          "  в_ремонте: { variant: 'warning', appearance: 'soft' },",
          "  в_ремонте: { variant: 'warning', appearance: 'soft', muted: true },",
        ),
      }),
      codes: ['F3'],
    });
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSources(f.files, f.opts ?? SELF_OPTS, v);
    const codeOf = (line) => /— \[([A-F][\w-]*)\]/.exec(line)?.[1];
    const got = new Set(v.map(codeOf));
    let ok;
    let expected;
    if (f.codes.length === 0) {
      ok = v.length === 0;
      expected = '0 нарушений';
    } else {
      ok = f.codes.every((c) => got.has(c)) && [...got].every((c) => f.codes.includes(c));
      if (ok && f.onlyFile) ok = v.every((line) => line.startsWith(f.onlyFile));
      if (ok && f.msgIncludes) {
        const f6 = v.filter((line) => line.includes('— [F6]'));
        ok =
          f6.length > 0 &&
          f6.every(
            (line) =>
              line.includes(f.msgIncludes) && !(f.msgExcludes && line.includes(f.msgExcludes)),
          );
      }
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
