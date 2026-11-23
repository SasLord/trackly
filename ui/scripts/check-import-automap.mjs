#!/usr/bin/env node
// [check-import-automap] Golden-гейт автомаппинга заголовков CSV импорта устройств
// (quick 261123-vn6, 39 W6 + 40.4 WR-14).
//
// ЗАЧЕМ. Авто-маппинг живёт в двух местах: `autoMapHeaders` в
// `DeviceImportCsvModal.svelte` (продукт) и Rust-зеркале `auto_map` в
// `crates/trackly-app/tests/devices_csv_import.rs` (тесты). Тест по зеркалу,
// которое разошлось с продуктом, проверяет не продукт. Общая golden-фикстура
// `crates/trackly-app/tests/fixtures/devices/automap-golden.json`
// (заголовок -> поле, '' = не распознан) читается обеими сторонами: Rust-тестом
// `auto_map_golden_fixture` и этим гейтом.
//
// Читает НАСТОЯЩИЙ исходник модалки: вырезает функцию `autoMapHeaders`, снимает
// типы TypeScript и ИСПОЛНЯЕТ её.
// Проверки:
//   [A0] фикстура и функция найдены, случаев больше 10 (защита от вакуума);
//   [A1] каждый заголовок фикстуры даёт записанное поле;
//   [A2] инвентарь ОТ ИСХОДНИКА: каждая метка `case '...'` функции есть в
//        фикстуре (алиас, добавленный только в модалку, не проскочит).
//
// Чего гейт НЕ видит (честно): Rust-сторону — её держит тест
// `auto_map_golden_fixture`; логику вне функции autoMapHeaders (например,
// пользовательский ручной выбор поля на шаге 2).
//
// Usage:
//   node scripts/check-import-automap.mjs             # реальные файлы
//   node scripts/check-import-automap.mjs --selftest  # мутанты реального исходника
//
// Зависимости: node:*, typescript (devDependency ui/).

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-import-automap]';
const UI_ROOT = path.resolve(__dirname, '..');
const MODAL = path.join(UI_ROOT, 'src/features/devices/DeviceImportCsvModal.svelte');
const FIXTURE = path.resolve(
  UI_ROOT,
  '../crates/trackly-app/tests/fixtures/devices/automap-golden.json',
);

/** Тело `function autoMapHeaders(...) { ... }` по балансу скобок; null — не найдено. */
function extractFunction(source) {
  const start = source.indexOf('function autoMapHeaders');
  if (start === -1) return null;
  const open = source.indexOf('{', source.indexOf(')', start));
  let depth = 0;
  for (let i = open; i < source.length; i++) {
    if (source[i] === '{') depth++;
    else if (source[i] === '}' && --depth === 0) return source.slice(start, i + 1);
  }
  return null;
}

export function check(modalSource, fixtureText) {
  const failures = [];
  let cases;
  try {
    cases = JSON.parse(fixtureText).cases;
  } catch (e) {
    return [`[A0] фикстура не читается: ${e.message}`];
  }
  if (!Array.isArray(cases) || cases.length <= 10) {
    return ['[A0] в фикстуре меньше 11 случаев (вакуум)'];
  }
  const fnSource = extractFunction(modalSource);
  if (fnSource === null) return ['[A0] в модалке нет функции autoMapHeaders'];

  const js = ts.transpileModule(fnSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2020 },
  }).outputText;
  let autoMap;
  try {
    autoMap = new Function(`${js}\nreturn autoMapHeaders;`)();
  } catch (e) {
    return [`[A0] autoMapHeaders не исполняется: ${e.message}`];
  }

  for (const { header, field } of cases) {
    const got = autoMap([header])[header] ?? '';
    if (got !== field) {
      failures.push(
        `[A1] заголовок ${JSON.stringify(header)}: модалка даёт ${JSON.stringify(got)}, фикстура ${JSON.stringify(field)}`,
      );
    }
  }

  const known = new Set(cases.map((c) => c.header.trim()));
  for (const m of fnSource.matchAll(/case\s+'([^']*)'\s*:/g)) {
    if (!known.has(m[1])) {
      failures.push(
        `[A2] алиас ${JSON.stringify(m[1])} есть в модалке, но не в фикстуре (Rust-зеркало его не знает)`,
      );
    }
  }
  return failures;
}

function selftest(modal, fixture) {
  const base = check(modal, fixture);
  if (base.length > 0) {
    console.error(`${TAG} SELFTEST FAIL — контроль не зелёный:\n  ${base.join('\n  ')}`);
    return 1;
  }
  // Якорь каждого мутанта обязан встречаться ровно один раз, иначе мутация
  // отключит соседний фрагмент и «красный» гейт окажется красным не по той причине.
  const mutants = [
    {
      name: 'убран алиас «Местоположение» из модалки',
      from: "          case 'Местоположение':\n",
      to: '',
      expect: 'A1',
    },
    {
      name: 'алиас «Расположение» ведёт не в place',
      from: "          case 'Расположение':\n          case 'Местоположение':\n            return 'place';",
      to: "          case 'Расположение':\n            return 'status';\n          case 'Местоположение':\n            return 'place';",
      expect: 'A1',
    },
    {
      name: 'лишний алиас только в модалке',
      from: "          case 'Комплектация':\n",
      to: "          case 'Комплектация':\n          case 'Набор':\n",
      expect: 'A2',
    },
  ];
  let bad = 0;
  for (const m of mutants) {
    const n = modal.split(m.from).length - 1;
    if (n !== 1) {
      console.error(`${TAG} SELFTEST FAIL — якорь «${m.name}» встречается ${n} раз, нужен ровно 1`);
      bad++;
      continue;
    }
    const res = check(modal.replace(m.from, m.to), fixture);
    if (!res.some((f) => f.startsWith(`[${m.expect}]`))) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» не пойман ([${m.expect}] ожидался)`);
      bad++;
    } else {
      console.log(`${TAG} мутант «${m.name}»: пойман`);
    }
  }
  // Фикстура с лишним случаем, которого модалка не знает, тоже красная.
  const fx = JSON.parse(fixture);
  fx.cases.push({ header: 'Нигде', field: 'place' });
  if (!check(modal, JSON.stringify(fx)).some((f) => f.startsWith('[A1]'))) {
    console.error(`${TAG} SELFTEST FAIL — случай фикстуры без алиаса в модалке не пойман`);
    bad++;
  } else {
    console.log(`${TAG} мутант «случай фикстуры без алиаса»: пойман`);
  }
  if (bad > 0) return 1;
  console.log(`${TAG} SELFTEST PASS — ${mutants.length + 1} мутантов пойманы`);
  return 0;
}

const modal = fs.readFileSync(MODAL, 'utf8');
const fixture = fs.readFileSync(FIXTURE, 'utf8');

if (process.argv.includes('--selftest')) {
  process.exit(selftest(modal, fixture));
}

const failures = check(modal, fixture);
if (failures.length > 0) {
  console.error(`${TAG} FAIL — ${failures.length} нарушений:\n  ${failures.join('\n  ')}`);
  process.exit(1);
}
console.log(`${TAG} PASS — 0 нарушений`);
