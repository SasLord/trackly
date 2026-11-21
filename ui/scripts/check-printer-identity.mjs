// Гейт quick-261121-heh (INT-31): признак «это принтер» на фронте — ОДИН
// модуль `src/lib/deviceTypeIds.ts` (`DEVICE_TYPE_ID`, `PRINTER_TYPE_ID`,
// `isPrinterType`), зеркало Rust-канона `crates/trackly-core/src/domain/devices.rs`.
// До этого пакета литерал `2` жил копией в семи компонентах (PrinterMark,
// DeviceFormBody, DeviceFormModal, DeviceRowMenu, GroupsPage,
// GroupPrintersList, PrintersPage). Бэкенд держит Rust-гейт
// `crates/trackly-app/tests/printer_identity_gate.rs`.
//
// Правила (обход ui/src/**/*.svelte|*.ts, комментарии вырезаются `_strip.mjs`;
// сам модуль и сгенерированные bindings*.ts вне J1/J2):
//   [J1] объявление `const|let|var <имя с TYPE_ID/TypeId> = 2` вне модуля —
//        локальная копия признака. Значение 1 (`ANCHOR_DEVICE_TYPE_ID = 1`,
//        тип-якорь) правило не трогает: оно про принтерное 2.
//   [J2] сравнение `type_id|typeId` с литералом 2 (`===`, `!==`, `==`, `!=`, в
//        любом порядке) и поле `type_id: 2` / `typeId: 2` вне модуля.
//   [J3] сравнение с подписью «Принтер» (`=== 'Принтер'`, `'принтер' !== x`,
//        без регистра): подпись — данные справочника, не идентичность.
//   [J4] значение `PRINTER_TYPE_ID` (и `DEVICE_TYPE_ID`) в модуле не равно
//        `pub const …: i64 = N;` из Rust-канона, либо одна из строк не найдена.
//   [J5] антивакуум: модуль импортируют меньше MIN_IMPORTERS файлов.
//   [E]  таблица освобождений EXEMPT: причина не короче 20 символов, путь
//        совпадает с файлом.
//
// Чего гейт НЕ видит (честно):
//   - число через переменную с другим именем (`const k = 2; typeId === k`);
//   - `typeId={2}` в разметке, `[1, 2].includes(typeId)`, `switch (typeId) {
//     case 2: … }`, `Number(x) === 2` без имени type_id рядом;
//   - сравнение с подписью не через `===`/`!==` (`.startsWith('Принт')`,
//     `includes`), подпись из переменной;
//   - рантайм-поведение (WKWebView, руны): проверяется в запущенном приложении.
//
// Self-test (`--selftest`): мутанты по РЕАЛЬНЫМ исходникам (якорь обязан
// встретиться ровно один раз) и синтетика; ожидание — ТОЧНОЕ множество кодов.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-printer-identity.mjs               # весь репозиторий
//   node scripts/check-printer-identity.mjs --src=<dir>   # копия каталога ui/
//   node scripts/check-printer-identity.mjs --rust=<file> # другой Rust-канон
//   node scripts/check-printer-identity.mjs --selftest    # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-printer-identity]';
const SRC_DIR = 'src';

const MODULE = 'src/lib/deviceTypeIds.ts';
const MODULE_IMPORT = /from\s+['"]\$lib\/deviceTypeIds['"]/;
const RUST_CANON_REL = '../crates/trackly-core/src/domain/devices.rs';
const MIN_IMPORTERS = 7;

// Сгенерированные specta-файлы: имена типов Rust, не UI-копии признака.
const GENERATED = /^src\/bindings[\w-]*\.ts$/;

// Освобождения: { path, reason }. Пусто — первый прогон по ui/src чист.
const EXEMPT = [];

const TYPE_NAME = '[\\w$]*(?:TYPE_ID|TypeId|type_id|typeId)[\\w$]*';
const RULES = [
  {
    code: 'J1',
    msg: 'локальная копия id типа принтера — импортируйте PRINTER_TYPE_ID из $lib/deviceTypeIds',
    re: [new RegExp(`\\b(?:const|let|var)\\s+${TYPE_NAME}\\s*(?::\\s*number\\s*)?=\\s*2\\b`, 'g')],
  },
  {
    code: 'J2',
    msg: 'сравнение type_id с литералом 2 — используйте isPrinterType()/PRINTER_TYPE_ID из $lib/deviceTypeIds',
    re: [
      /(?<![\w$])(?:type_id|typeId)\s*[!=]==?\s*2\b/g,
      /(?<![\w$.])2\s*[!=]==?\s*[\w$.]*(?:type_id|typeId)\b/g,
      /(?<![\w$])(?:type_id|typeId)\s*:\s*2\b/g,
    ],
  },
  {
    code: 'J3',
    msg: 'сравнение с подписью «Принтер» — признак принтера по id типа (isPrinterType)',
    re: [/[!=]==?\s*['"`]\s*принтер\s*['"`]/giu, /['"`]\s*принтер\s*['"`]\s*[!=]==?/giu],
  },
];

// ---------------------------------------------------------------------------
// Разбор
// ---------------------------------------------------------------------------

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

function listSourceFiles(root) {
  return walk(path.join(root, SRC_DIR))
    .map((f) => path.relative(root, f).split(path.sep).join('/'))
    .sort();
}

function diskReader(root) {
  return (rel) => fs.readFileSync(path.join(root, rel), 'utf8');
}

/** `pub const NAME: i64 = N;` из Rust-канона; null — строки нет. */
function rustConst(text, name) {
  const m = new RegExp(`pub const ${name}: i64 = (\\d+);`).exec(text ?? '');
  return m ? Number(m[1]) : null;
}

/** `export const NAME = N;` из модуля; null — строки нет. */
function tsConst(text, name) {
  const m = new RegExp(`export const ${name}\\s*=\\s*(\\d+);`).exec(text ?? '');
  return m ? Number(m[1]) : null;
}

// ---------------------------------------------------------------------------
// Проверка
// ---------------------------------------------------------------------------

/**
 * @param read      (rel) => текст файла ui/
 * @param files     просматриваемые файлы
 * @param rustText  текст Rust-канона (null — не найден)
 * @param opts      { exempt, minImporters, full }
 */
function collectViolations(
  read,
  files,
  rustText,
  { exempt = EXEMPT, minImporters = MIN_IMPORTERS, full = true } = {},
) {
  const violations = [];
  const importers = [];
  let scanned = 0;

  for (const e of exempt) {
    if (!e.reason || e.reason.length < 20) {
      violations.push({ code: 'E', file: e.path, msg: 'освобождение без причины (≥ 20 символов)' });
    }
    if (full && !files.includes(e.path)) {
      violations.push({ code: 'E', file: e.path, msg: 'устаревшее освобождение: файла нет' });
    }
  }

  for (const rel of files) {
    if (GENERATED.test(rel)) continue;
    if (exempt.some((e) => e.path === rel)) continue;
    scanned++;
    const text = stripComments(read(rel));
    if (MODULE_IMPORT.test(text)) importers.push(rel);
    if (rel === MODULE) continue;
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

  if (full) {
    const moduleText = files.includes(MODULE) ? stripComments(read(MODULE)) : null;
    for (const name of ['PRINTER_TYPE_ID', 'DEVICE_TYPE_ID']) {
      const ts = tsConst(moduleText, name);
      const rs = rustConst(rustText, name);
      if (ts === null || rs === null || ts !== rs) {
        violations.push({
          code: 'J4',
          file: MODULE,
          msg: `${name}: модуль = ${ts ?? 'не найден'}, trackly-core = ${rs ?? 'не найден'} — зеркало разошлось с Rust-каноном`,
        });
      }
    }
    if (importers.length < minImporters) {
      violations.push({
        code: 'J5',
        file: '(репозиторий)',
        msg: `модуль $lib/deviceTypeIds импортируют ${importers.length} файлов, нужно не меньше ${minImporters}: гейт зелёный по пустоте`,
      });
    }
  }
  return { violations, scanned, importers };
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

function selftest(root, rustText) {
  const files = listSourceFiles(root);
  const disk = diskReader(root);
  const MARK = 'src/features/devices/PrinterMark.svelte';
  const PAGE = 'src/features/printers/PrintersPage.svelte';

  const run = ({
    over = new Map(),
    extra = [],
    list = files,
    rust = rustText,
    exempt = EXEMPT,
    full = true,
  } = {}) => {
    const synth = new Map(extra);
    const read = (rel) =>
      over.has(rel) ? over.get(rel) : synth.has(rel) ? synth.get(rel) : disk(rel);
    return collectViolations(read, [...list, ...synth.keys()], rust, { exempt, full });
  };
  const codes = (r) => [...new Set(r.violations.map((v) => v.code))].sort().join(',');
  const cases = [];
  const add = (name, expected, result) =>
    cases.push({ name, expected, got: codes(result), result });

  add('контроль: реальное дерево чисто', '', run());

  // J1: PrinterMark — вернуть локальную копию константы (импорт остаётся, J5 не срабатывает)
  {
    const real = disk(MARK);
    const anchor = "import { isPrinterType } from '$lib/deviceTypeIds';";
    const over = new Map([
      [MARK, mutate(real, anchor, `${anchor}\n  const PRINTER_TYPE_ID = 2;`, 'PrinterMark import')],
    ]);
    add('PrinterMark: const PRINTER_TYPE_ID = 2', 'J1', run({ over }));
  }
  // J2: PrintersPage — сравнение с литералом
  {
    const real = disk(PAGE);
    const over = new Map([
      [
        PAGE,
        mutate(
          real,
          'result.typeId !== PRINTER_TYPE_ID',
          'result.typeId !== 2',
          'PrintersPage typeId',
        ),
      ],
    ]);
    add('PrintersPage: result.typeId !== 2', 'J2', run({ over }));
  }
  // J4: модуль разошёлся с Rust
  {
    const real = disk(MODULE);
    const over = new Map([
      [
        MODULE,
        mutate(
          real,
          'export const PRINTER_TYPE_ID = 2;',
          'export const PRINTER_TYPE_ID = 3;',
          'module value',
        ),
      ],
    ]);
    add('deviceTypeIds.ts: PRINTER_TYPE_ID = 3', 'J4', run({ over }));
  }
  add('Rust-канон не найден', 'J4', run({ rust: null }));
  add(
    'Rust-канон с другим значением',
    'J4',
    run({
      rust: (rustText ?? '').replace('PRINTER_TYPE_ID: i64 = 2;', 'PRINTER_TYPE_ID: i64 = 5;'),
    }),
  );
  // J3: синтетика — сравнение с подписью
  add(
    'синтетика: name === «Принтер»',
    'J3',
    run({
      extra: [['src/x/Synth.svelte', "<script>const p = t.name === 'Принтер';</script>\n"]],
    }),
  );
  add(
    'синтетика: «принтер» !== name',
    'J3',
    run({ extra: [['src/x/synth.ts', 'export const p = "принтер" !== t.name;\n']] }),
  );
  // J2: синтетика — обратный порядок и поле
  add(
    'синтетика: 2 === d.type_id и { type_id: 2 }',
    'J2',
    run({
      extra: [
        ['src/x/synth.ts', 'export const a = 2 === d.type_id;\nexport const f = { type_id: 2 };\n'],
      ],
    }),
  );
  // J5: синтетика — без импортёров
  add('синтетика: импортёров нет', 'J5', run({ list: [MODULE] }));
  // E: освобождение без причины
  add(
    'E: освобождение без причины',
    'E',
    run({ exempt: [{ path: 'src/lib/deviceStatusTone.ts', reason: 'коротко' }] }),
  );
  // законные формы не краснят
  add(
    'синтетика: законные формы',
    '',
    run({
      extra: [
        [
          'src/x/ok.ts',
          [
            '// const PRINTER_TYPE_ID = 2; typeId === 2 — в комментарии',
            'export const ANCHOR_DEVICE_TYPE_ID = 1;',
            'export const a = (t: number) => t === 20 || t === 12;',
            "export const label = 'Принтер';",
            "export const kinds = { printer: 'Принтер', device: 'Устройство' };",
            'export const s = statusId === 2;',
            '',
          ].join('\n'),
        ],
      ],
    }),
  );

  let failed = 0;
  for (const c of cases) {
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
  console.log(`${TAG} SELFTEST PASS — ${cases.length} кейсов`);
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const srcArg = args.find((a) => a.startsWith('--src='));
const rustArg = args.find((a) => a.startsWith('--rust='));
const ROOT = srcArg ? path.resolve(srcArg.slice('--src='.length)) : path.resolve(__dirname, '..');
const RUST = rustArg
  ? path.resolve(rustArg.slice('--rust='.length))
  : path.resolve(ROOT, RUST_CANON_REL);
const rustText = fs.existsSync(RUST) ? fs.readFileSync(RUST, 'utf8') : null;

if (args.includes('--selftest')) {
  selftest(ROOT, rustText);
} else {
  const files = listSourceFiles(ROOT);
  const res = collectViolations(diskReader(ROOT), files, rustText);
  if (res.violations.length) {
    for (const v of res.violations)
      console.error(`${TAG} [${v.code}] ${v.file}${v.line ? ':' + v.line : ''} — ${v.msg}`);
    console.error(`${TAG} FAIL — ${res.violations.length} нарушений`);
    process.exit(1);
  }
  console.log(
    `${TAG} PASS — 0 нарушений (${res.scanned} файлов, ${res.importers.length} импортёров $lib/deviceTypeIds)`,
  );
}
