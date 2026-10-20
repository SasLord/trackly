#!/usr/bin/env node
// [check-device-type-labels] Постоянный гейт «в UI нет карты id типа устройства
// -> подпись» (фаза 41.12, план 41.12-04, INT-14, GRD-04, D-01).
//
// Почему он существует: подпись типа жила в UI копиями. Фильтр отчётов держал
// `{ id: 2, name: 'Расходник' }` (в справочнике device_types второй тип —
// «Принтер»; экран предлагал тип, которого нет), пикер «Добавить устройства в
// группу» — `TYPE_OPTIONS` с верными подписями, но всё равно копией. Теперь типы
// приходят командой `device_types_list` в стор `$lib/stores/deviceTypes.svelte`,
// а копию, даже с верной подписью, гейт краснит: справочник расширяется данными.
//
// Почему отдельный гейт, а не правило в check-device-status-labels: тот гейт
// освобождает ReportsPage целиком (имена вкладок отчётов и статусы картриджей),
// правило статусов этот файл не увидело бы.
//
// Правила (сообщение начинается с кода в квадратных скобках):
//   [T1] копия справочника: числовой id рядом с подписью типа («Устройство»,
//        «Принтер», «Расходник») — литерал `{ id|value|type_id|key: N,
//        name|label: '<подпись>' }` (и обратный порядок полей), запись
//        `N: '<подпись>'` и пара `[N, '<подпись>']`. Карты с ключом-видом
//        (`device: 'Устройство'` в PlaceContents / NumberTakenPopup) не
//        флагаются: левый операнд не число;
//   [T2] массив подписей типов подряд: `'Устройство', 'Принтер'` (в любом
//        порядке) — индекс массива молча становится id;
//   [T3] строковый литерал `'Расходник'` целиком (с заглавной, любые кавычки).
//        D-01: освобождения для этого слова в UI НЕТ — именованный псевдоним
//        старой подписи при импорте CSV живёт только в Rust (device_service.rs)
//        и освобождён в Rust-гейте device_type_label_gate. Строчное «расходник»
//        в тексте про картриджи не флагается;
//   [T4] антивакуум (только полный режим): файлов, импортирующих
//        `$lib/stores/deviceTypes.svelte`, не меньше MIN_CONSUMERS — иначе стор
//        никто не читает, а гейт зелёный по пустоте;
//   [E]  таблица освобождений EXEMPT: причина не короче 20 символов, путь — файл
//        (.svelte|.ts) либо каталог с `/` на конце; в полном режиме запись
//        обязана совпасть хотя бы с одним файлом (устаревшее освобождение молча
//        глушило бы гейт). Реальная таблица пуста: первый прогон по репозиторию
//        нарушений не дал; движок [E] проверяется self-test'ом с таблицей-параметром.
//
// Список файлов ВЫВОДИТСЯ обходом src/**/*.svelte и src/**/*.ts — ничего не
// перечислено вручную. Единственный пропуск вне EXEMPT — сгенерированный
// src/bindings.ts (GENERATED, причина ниже). Рукописные bindings-phase6/9.ts
// сканируются. Режим `--report` печатает инвентарь: просмотренные файлы,
// потребителей стора и пропуски с причиной.
//
// Чего гейт НЕ видит (честно):
//   - подпись, склеенная из частей или построенная косвенно (`['Уст', 'ройство']`);
//   - слово «Расходник» в КОММЕНТАРИИ: комментарии вырезаются, чтобы цитата
//     истории не краснила гейт; запрет слова в комментариях ui/src держит
//     приёмочный grep плана 41.12-04;
//   - фактическую загрузку стора в рантайме (WKWebView, руны): это проверка
//     человеком в запущенном приложении (human-check плана 41.12-07).
//
// Self-test (`--selftest`): мутанты по РЕАЛЬНЫМ исходникам (якорь обязан
// встретиться ровно один раз) и синтетические файлы; ожидание сверяется ТОЧНЫМ
// множеством кодов.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-device-type-labels.mjs                 # весь репозиторий
//   node scripts/check-device-type-labels.mjs --src=<dir>     # копия каталога ui/
//   node scripts/check-device-type-labels.mjs --report        # инвентарь
//   node scripts/check-device-type-labels.mjs --selftest      # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-device-type-labels]';
const SRC_DIR = 'src';

const STORE_IMPORT = '$lib/stores/deviceTypes.svelte';
// Фильтр отчётов и пикер типа в «Добавить устройства в группу».
const MIN_CONSUMERS = 2;
const SELFTEST_MIN_CONSUMERS = 1;

// Сгенерированный specta-файл: содержит имена типов Rust, не UI-копии. В git не
// лежит (ui/src/bindings.ts в .gitignore), пересоздаётся prebuild'ом.
const GENERATED = new Map([
  [
    'src/bindings.ts',
    'сгенерирован specta (в .gitignore), перезаписывается prebuild; UI-копий не содержит',
  ],
]);

// Освобождения: { path, reason }. Пусто — первый прогон по ui/src чист.
const EXEMPT = [];

const LABELS = 'Устройство|Принтер|Расходник';
const Q = '[\'"`]';
const NOT_IDENT = '(?<![\\p{L}\\p{N}_$.])';
const KEY = '(?:id|value|type_id|typeId|key)';
const NAME = '(?:name|label)';

const RULES = [
  {
    code: 'T1',
    re: [
      new RegExp(
        `${NOT_IDENT}${KEY}\\s*:\\s*\\d+\\s*,\\s*${NAME}\\s*:\\s*${Q}(?:${LABELS})${Q}`,
        'gu',
      ),
      new RegExp(
        `${NOT_IDENT}${NAME}\\s*:\\s*${Q}(?:${LABELS})${Q}\\s*,\\s*${KEY}\\s*:\\s*\\d+`,
        'gu',
      ),
      new RegExp(`${NOT_IDENT}\\d+\\s*:\\s*${Q}(?:${LABELS})${Q}`, 'gu'),
      new RegExp(`\\[\\s*\\d+\\s*,\\s*${Q}(?:${LABELS})${Q}\\s*\\]`, 'gu'),
    ],
    msg: 'копия справочника типов: числовой id рядом с подписью типа; берите типы из $lib/stores/deviceTypes.svelte',
  },
  {
    code: 'T2',
    re: [
      new RegExp(`${Q}Устройство${Q}\\s*,\\s*${Q}Принтер${Q}`, 'gu'),
      new RegExp(`${Q}Принтер${Q}\\s*,\\s*${Q}Устройство${Q}`, 'gu'),
    ],
    msg: 'массив подписей типов подряд: индекс станет id; берите типы из $lib/stores/deviceTypes.svelte',
  },
  {
    code: 'T3',
    re: [new RegExp(`${Q}Расходник${Q}`, 'gu')],
    msg: "литерал 'Расходник': такого типа в справочнике нет (D-01: псевдоним импорта живёт только в Rust)",
  },
];

// ---------------------------------------------------------------------------
// Разбор
// ---------------------------------------------------------------------------

/**
 * Затирает комментарии (HTML, блочные, строчные `//`), сохраняя длину и переводы
 * строк. Один проход слева направо; `//` и `/*` после нечётного числа кавычек на
 * строке считаются частью строки.
 */
function stripComments(src) {
  const out = src.split('');
  const blank = (from, to) => {
    for (let i = from; i < to && i < src.length; i++) if (out[i] !== '\n') out[i] = ' ';
  };
  let lineStart = 0;
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (c === '\n') {
      lineStart = i + 1;
      i++;
      continue;
    }
    if (src.startsWith('<!--', i)) {
      const end = src.indexOf('-->', i + 4);
      const to = end < 0 ? src.length : end + 3;
      blank(i, to);
      i = to;
      continue;
    }
    if (c === '/' && (src[i + 1] === '/' || src[i + 1] === '*')) {
      const quotes = (src.slice(lineStart, i).match(/['"`]/g) ?? []).length;
      if (quotes % 2 === 0) {
        let to;
        if (src[i + 1] === '/') {
          const nl = src.indexOf('\n', i);
          to = nl < 0 ? src.length : nl;
        } else {
          const end = src.indexOf('*/', i + 2);
          to = end < 0 ? src.length : end + 2;
        }
        blank(i, to);
        const lastNl = src.lastIndexOf('\n', to - 1);
        if (lastNl >= i) lineStart = lastNl + 1;
        i = to;
        continue;
      }
    }
    i++;
  }
  return out.join('');
}

function lineOf(text, index) {
  let n = 1;
  for (let i = 0; i < index; i++) if (text.charCodeAt(i) === 10) n++;
  return n;
}

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(full));
    else if (/\.(svelte|ts)$/.test(entry.name)) out.push(full);
  }
  return out;
}

/** Все .svelte/.ts под <root>/src (пути от root, с «/»). Пропуск GENERATED не выполняется здесь. */
function listSourceFiles(root) {
  return walk(path.join(root, SRC_DIR))
    .map((f) => path.relative(root, f).split(path.sep).join('/'))
    .sort();
}

function diskReader(root) {
  return (rel) => fs.readFileSync(path.join(root, rel), 'utf8');
}

function exemptCovers(entry, rel) {
  return entry.path.endsWith('/') ? rel.startsWith(entry.path) : rel === entry.path;
}

// ---------------------------------------------------------------------------
// Проверка
// ---------------------------------------------------------------------------

/**
 * @param read   (rel) => текст файла
 * @param files  просматриваемые файлы
 * @param opts   { exempt, minConsumers, full }
 * @returns { violations: {code,file,line?,msg}[], scanned, consumers, skipped }
 */
function collectViolations(
  read,
  files,
  { exempt = EXEMPT, minConsumers = MIN_CONSUMERS, full = true } = {},
) {
  const violations = [];
  const consumers = [];
  const skipped = [];
  let scanned = 0;

  for (const e of exempt) {
    if (!e.reason || e.reason.length < 20) {
      violations.push({
        code: 'E',
        file: e.path,
        msg: 'освобождение без причины (нужно не короче 20 символов)',
      });
    }
    if (!e.path.endsWith('/') && !/\.(svelte|ts)$/.test(e.path)) {
      violations.push({
        code: 'E',
        file: e.path,
        msg: 'путь освобождения: файл .svelte/.ts либо каталог с «/» на конце',
      });
    }
    if (full && !files.some((f) => exemptCovers(e, f))) {
      violations.push({
        code: 'E',
        file: e.path,
        msg: 'устаревшее освобождение: не совпало ни с одним файлом',
      });
    }
  }

  for (const rel of files) {
    if (GENERATED.has(rel)) {
      skipped.push({ file: rel, reason: GENERATED.get(rel) });
      continue;
    }
    const ex = exempt.find((e) => exemptCovers(e, rel));
    if (ex) {
      skipped.push({ file: rel, reason: ex.reason });
      continue;
    }
    scanned++;
    const text = stripComments(read(rel));
    if (text.includes(STORE_IMPORT)) consumers.push(rel);
    for (const rule of RULES) {
      for (const re of rule.re) {
        for (const m of text.matchAll(re)) {
          violations.push({
            code: rule.code,
            file: rel,
            line: lineOf(text, m.index),
            msg: `${rule.msg} («${m[0].replace(/\s+/g, ' ')}»)`,
          });
        }
      }
    }
  }

  if (full && consumers.length < minConsumers) {
    violations.push({
      code: 'T4',
      file: '(репозиторий)',
      msg: `стор типов читают ${consumers.length} файлов, нужно не меньше ${minConsumers}: гейт зелёный по пустоте`,
    });
  }
  return { violations, scanned, consumers, skipped };
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL — якорь «${label}» встречается ${n} раз (нужно ровно 1)`);
    process.exit(1);
  }
  return source.replace(from, () => to);
}

function selftest(root) {
  const files = listSourceFiles(root);
  const disk = diskReader(root);
  const REPORTS = 'src/features/reports/ReportsPage.svelte';
  const GROUP = 'src/features/groups/GroupAddDevicesModal.svelte';

  const run = ({
    over = new Map(),
    extra = [],
    list = files,
    exempt = EXEMPT,
    minConsumers = MIN_CONSUMERS,
    full = true,
  } = {}) => {
    const synth = new Map(extra);
    const read = (rel) =>
      over.has(rel) ? over.get(rel) : synth.has(rel) ? synth.get(rel) : disk(rel);
    return collectViolations(read, [...list, ...synth.keys()], { exempt, minConsumers, full });
  };
  const codes = (r) => [...new Set(r.violations.map((v) => v.code))].sort().join(',');

  const cases = [];
  // msg — подстрока, обязательная среди сообщений: разводит правила с одним кодом.
  const add = (name, expected, result, msg) =>
    cases.push({ name, expected, got: codes(result), result, msg });

  // (а) контроль: реальное дерево чисто
  add('контроль: реальное дерево чисто', '', run());

  // (б) ReportsPage: прежний массив с неверной подписью -> T1 + T3
  {
    const real = disk(REPORTS);
    const anchor = real.match(/ {2}const filterDeviceTypes = \$derived\([^;]*\);\n/);
    if (!anchor) {
      console.error(
        `${TAG} SELFTEST FAIL — в ReportsPage нет объявления filterDeviceTypes на $derived`,
      );
      process.exit(1);
    }
    const old =
      "  let filterDeviceTypes = $state([{ id: 1, name: 'Устройство' }, { id: 2, name: 'Расходник' }]);\n";
    const over = new Map([[REPORTS, mutate(real, anchor[0], old, 'filterDeviceTypes')]]);
    add('ReportsPage: прежний массив { id: 2, name: Расходник }', 'T1,T3', run({ over }));
  }

  // (в) GroupAddDevicesModal: копия с ВЕРНОЙ подписью -> T1
  {
    const real = disk(GROUP);
    const anchor = '...orderedTypes().map((t) => ({ id: t.id, label: t.name })),';
    const old = "{ id: 1, label: 'Устройство' }, { id: 2, label: 'Принтер' },";
    const over = new Map([[GROUP, mutate(real, anchor, old, 'typeOptions spread')]]);
    add('GroupAddDevicesModal: копия справочника с верной подписью', 'T1', run({ over }));
  }

  // (г) литерал 'Расходник' сам по себе
  add(
    "синтетика: литерал 'Расходник'",
    'T3',
    run({ extra: [['src/zz/Lit.svelte', "<script>const kind = 'Расходник';</script>"]] }),
  );
  add(
    'синтетика: литерал в двойных кавычках',
    'T3',
    run({ extra: [['src/zz/Dq.svelte', '<p>{"Расходник"}</p>']] }),
  );

  // (д) карта с ключом-видом и строчное слово — чисто
  add(
    'синтетика: карта по ключу-виду не флагается',
    '',
    run({
      extra: [
        [
          'src/zz/Kind.svelte',
          "<script>const L = { device: 'Устройство', printer: 'Принтер' };</script>",
        ],
      ],
    }),
  );
  add(
    'синтетика: строчное «расходник» в тексте — чисто',
    '',
    run({ extra: [['src/zz/Low.svelte', "<script>const t = 'Заменить расходник';</script>"]] }),
  );
  add(
    'синтетика: слово в комментарии не флагается',
    '',
    run({
      extra: [['src/zz/Cmt.ts', "// было { id: 2, name: 'Расходник' }\nexport const a = 1;\n"]],
    }),
  );

  // формы T1 и T2
  add(
    'синтетика: запись N: подпись',
    'T1',
    run({ extra: [['src/zz/Rec.ts', "export const m = { 1: 'Устройство', 2: 'Принтер' };"]] }),
  );
  add(
    'синтетика: пара [N, подпись]',
    'T1',
    run({ extra: [['src/zz/Pair.ts', "export const m = new Map([[2, 'Принтер']]);"]] }),
  );
  add(
    'синтетика: обратный порядок полей',
    'T1',
    run({ extra: [['src/zz/Rev.ts', "export const o = { label: 'Принтер', id: 2 };"]] }),
  );
  add(
    'синтетика: value/label',
    'T1',
    run({ extra: [['src/zz/Val.ts', "export const o = [{ value: 1, label: 'Устройство' }];"]] }),
  );
  add(
    'синтетика: массив подписей подряд',
    'T2',
    run({ extra: [['src/zz/Arr.ts', "export const t = ['Устройство', 'Принтер'];"]] }),
  );
  add(
    'синтетика: сосед { id: null, label: Все типы } чисто',
    '',
    run({ extra: [['src/zz/Null.ts', "export const o = { id: null, label: 'Все типы' };"]] }),
  );

  // (е) антивакуум T4: один файл вместо двух потребителей
  add('T4: потребителей меньше порога', 'T4', run({ list: [GROUP] }));
  add(
    'T4: порог не срабатывает в режиме --scope-подобном (full=false)',
    '',
    run({ list: [GROUP], full: false }),
  );

  // (ж) движок [E]
  add(
    'E: освобождение без совпавшего файла',
    'E',
    run({ exempt: [{ path: 'src/nope/', reason: 'каталог, которого не существует в дереве' }] }),
    'устаревшее освобождение',
  );
  add(
    'E: причина короче 20 символов',
    'E',
    run({ exempt: [{ path: 'src/features/showcase/', reason: 'витрина' }] }),
    'без причины',
  );
  add(
    'E: путь-каталог без «/»',
    'E',
    run({
      exempt: [{ path: 'src/features/showcase', reason: 'каталог без слэша не покрывает файлы' }],
    }),
    'каталог с «/» на конце',
  );
  add(
    'E: настоящее освобождение глушит T3 в своём файле',
    '',
    run({
      extra: [['src/zz/Ok.svelte', "<script>const k = 'Расходник';</script>"]],
      exempt: [
        { path: 'src/zz/Ok.svelte', reason: 'синтетический файл self-test, освобождён осознанно' },
      ],
    }),
  );

  // обход: GENERATED пропускается, но рукописные bindings-phase*.ts сканируются
  add(
    'обход: bindings.ts пропущен',
    '',
    run({ extra: [['src/bindings.ts', "export const x = { id: 2, name: 'Расходник' };"]] }),
  );
  add(
    'обход: bindings-phase6.ts сканируется',
    'T3',
    run({ extra: [['src/bindings-phase6.ts', "export const x = 'Расходник';"]] }),
  );

  let failed = 0;
  for (const c of cases) {
    if (c.msg && !c.result.violations.some((v) => v.msg.includes(c.msg))) {
      failed++;
      console.error(`${TAG} SELFTEST FAIL — ${c.name}: нет нарушения с текстом «${c.msg}»`);
      continue;
    }
    if (c.got !== c.expected) {
      failed++;
      console.error(
        `${TAG} SELFTEST FAIL — ${c.name}: ожидалось {${c.expected}}, получено {${c.got}}`,
      );
      for (const v of c.result.violations.slice(0, 5))
        console.error(`  [${v.code}] ${v.file}${v.line ? ':' + v.line : ''} — ${v.msg}`);
    }
  }
  if (failed) {
    console.error(`${TAG} SELFTEST FAIL — красных кейсов: ${failed} из ${cases.length}`);
    process.exit(1);
  }
  // SELFTEST_MIN_CONSUMERS нужен, чтобы константа не осталась мёртвой: режим с одним потребителем
  const one = run({ list: [GROUP], minConsumers: SELFTEST_MIN_CONSUMERS });
  if (codes(one) !== '') {
    console.error(
      `${TAG} SELFTEST FAIL — порог ${SELFTEST_MIN_CONSUMERS} на одном потребителе должен быть чистым, получено {${codes(one)}}`,
    );
    process.exit(1);
  }
  console.log(`${TAG} SELFTEST PASS — ${cases.length + 1} кейсов`);
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const srcArg = args.find((a) => a.startsWith('--src='));
const ROOT = srcArg ? path.resolve(srcArg.slice('--src='.length)) : path.resolve(__dirname, '..');

if (args.includes('--selftest')) {
  selftest(ROOT);
} else {
  const files = listSourceFiles(ROOT);
  const res = collectViolations(diskReader(ROOT), files);
  if (args.includes('--report')) {
    console.log(`${TAG} просмотрено: ${res.scanned} файлов из ${files.length}`);
    console.log(
      `${TAG} потребители стора типов (${res.consumers.length}, порог ${MIN_CONSUMERS}):`,
    );
    for (const f of res.consumers) console.log(`  ${f}`);
    console.log(`${TAG} пропущено (${res.skipped.length}):`);
    for (const s of res.skipped) console.log(`  ${s.file} — ${s.reason}`);
  }
  if (res.violations.length) {
    for (const v of res.violations)
      console.error(`${TAG} [${v.code}] ${v.file}${v.line ? ':' + v.line : ''} — ${v.msg}`);
    console.error(`${TAG} FAIL — ${res.violations.length} нарушений`);
    process.exit(1);
  }
  console.log(
    `${TAG} PASS — 0 нарушений (${res.scanned} файлов, ${res.consumers.length} потребителей стора)`,
  );
}
