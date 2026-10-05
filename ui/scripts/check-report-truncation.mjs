#!/usr/bin/env node
// [check-report-truncation] Гейт уведомления об усечении отчёта «Перемещения»
// (фаза 41, W-B03 / GAP-5, план 41-33).
//
// Почему он существует: `movementsTruncationNotice` — JS-зеркало серверной
// `movements_truncation_notice` (печать и CSV, план 41-30). `svelte-check` /
// `eslint` / `pnpm build` не ловят ни расхождение формулы или текста, ни чтение
// не того поля: `rows.rows.length` вместо `rows.total` типы пропускают, а
// баннер тогда молча не появляется никогда (длина массива не бывает больше
// самой себя).
//
// Гейт ИСПОЛНЯЮЩИЙ и состоит из двух частей.
//  1. Golden-фикстура `scripts/fixtures/report-truncation/cases.json` (создана
//     планом 41-30; читается С ДИСКА, в скрипт не копируется): входы
//     (`shown`, `total`) и ТОЧНЫЙ ожидаемый текст либо `null`. Ту же фикстуру
//     читает Rust-тест `report_trunc_notice_matches_golden_fixture`.
//  2. Структурные правила для `ReportsPage.svelte` (текст без комментариев,
//     пробелы схлопнуты):
//       S1 — импорт `movementsTruncationNotice` из `./truncationNotice`;
//       S2 — в файле нет литерала «Показано записей» (текст живёт в модуле);
//       S3 — каждый вызов `movementsTruncationNotice(shown, total)`: первый
//            аргумент заканчивается на `.length` и не содержит `.total`, второй
//            заканчивается на `.total` — усечение определяется по `total` ответа
//            сервера, а не по длине массива и не с переставленными аргументами;
//            хотя бы один вызов в файле есть.
//
// `--selftest` прогоняет те же проверки на встроенных реализациях: эталон
// обязан пройти, мутанты M1-M8 — провалиться.
//
// Фикстура содержит только числа (никаких реальных данных).
//
// Usage:
//   node scripts/check-report-truncation.mjs                # проверить репозиторий
//   node scripts/check-report-truncation.mjs --impl=<path>  # проверить копию truncationNotice.ts
//   node scripts/check-report-truncation.mjs --selftest     # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const DEFAULT_IMPL = path.join(UI_ROOT, 'src/features/reports/truncationNotice.ts');
const SVELTE = path.join(UI_ROOT, 'src/features/reports/ReportsPage.svelte');
const FIXTURE = path.join(__dirname, 'fixtures/report-truncation/cases.json');
const TAG = '[check-report-truncation]';
const MIN_CASES = 6;

function parseArgs(argv) {
  const args = { impl: DEFAULT_IMPL, selftest: false };
  for (const arg of argv) {
    if (arg.startsWith('--impl=')) args.impl = path.resolve(process.cwd(), arg.slice(7));
    else if (arg === '--selftest') args.selftest = true;
  }
  return args;
}

/** Снимает аннотации типов с .ts и возвращает исполняемый ESM-исходник. */
function transpileToEsm(tsSource, label) {
  const require = createRequire(import.meta.url);
  let ts;
  try {
    ts = require('typescript');
  } catch {
    console.error(
      `${TAG} FAIL — не разрешается пакет \`typescript\` (прямой devDependency ui/package.json); запусти \`pnpm install\` в ui/.`,
    );
    process.exit(1);
  }
  return ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: label,
    reportDiagnostics: false,
  }).outputText;
}

async function loadModuleFromTs(tsSource, label) {
  const js = transpileToEsm(tsSource, label);
  // data:-URL: модуль самодостаточен (ни одного import).
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
  if (
    !cases.some((c) => c.expected === null) ||
    !cases.some((c) => typeof c.expected === 'string')
  ) {
    console.error(
      `${TAG} FAIL — в фикстуре нет кейсов обоих исходов (строка и null): гейт перестал различать «усечено» и «не усечено»`,
    );
    process.exit(1);
  }
  return cases;
}

/** Возвращает список текстов провалов (пустой — всё в порядке). */
function runCases(mod, cases) {
  if (typeof mod.movementsTruncationNotice !== 'function') {
    return [
      'модуль не экспортирует movementsTruncationNotice — уведомление больше никем не проверяется',
    ];
  }
  const failures = [];
  for (const c of cases) {
    const label = `${c.name} (shown=${c.shown}, total=${c.total})`;
    try {
      const got = mod.movementsTruncationNotice(c.shown, c.total);
      if (got !== c.expected) {
        failures.push(
          `${label}: вернул ${JSON.stringify(got)}, ожидалось ${JSON.stringify(c.expected)}`,
        );
      }
    } catch (e) {
      failures.push(`${label}: исключение ${e.message}`);
    }
  }
  return failures;
}

/** Текст .svelte без комментариев, пробелы/переводы строк схлопнуты в один пробел. */
function normalizeSvelte(src) {
  return src
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/(^|\s)\/\/[^\n]*/g, ' ')
    .replace(/\s+/g, ' ');
}

/** Структурные правила S1-S3; возвращает список провалов. */
function runStructural(svelteSrc) {
  const failures = [];
  const text = normalizeSvelte(svelteSrc);
  if (
    !/import\s*\{[^}]*\bmovementsTruncationNotice\b[^}]*\}\s*from\s*['"]\.\/truncationNotice['"]/.test(
      text,
    )
  ) {
    failures.push(
      'S1: нет импорта movementsTruncationNotice из ./truncationNotice — экран ушёл от единого источника текста',
    );
  }
  if (text.includes('Показано записей')) {
    failures.push(
      'S2: в ReportsPage литерал «Показано записей» — текст должен жить только в truncationNotice.ts',
    );
  }
  const calls = [
    ...text.matchAll(/\bmovementsTruncationNotice\(\s*([^(),]+?)\s*,\s*([^()]+?)\s*\)/g),
  ];
  if (calls.length === 0) {
    failures.push(
      'S3: нет вызова movementsTruncationNotice(shown, total) — баннер усечения не выводится',
    );
  }
  for (const [call, shown, total] of calls) {
    if (!/\.total$/.test(total)) {
      failures.push(
        `S3: «${call}» — второй аргумент «${total}» не заканчивается на .total: усечение нельзя определять по длине массива строк`,
      );
    }
    if (!/\.length$/.test(shown) || /\.total/.test(shown)) {
      failures.push(
        `S3: «${call}» — первый аргумент «${shown}» должен быть числом полученных строк (.length), без .total`,
      );
    }
  }
  return failures;
}

// Эталон для --selftest (идентичен truncationNotice.ts по поведению) и мутанты.
const GOOD = `
export function movementsTruncationNotice(shown, total) {
  if (total <= shown) return null;
  return \`Показано записей: \${shown} из \${total} (самые ранние). Сузьте период или фильтры, чтобы увидеть остальные.\`;
}
`;

const GOOD_SVELTE = `
<script lang="ts">
  import { movementsTruncationNotice } from './truncationNotice';
  const truncationNotice = $derived(
    reportTypeKey() === 'movements' && rows
      ? movementsTruncationNotice(rows.rows.length, rows.total)
      : null,
  );
</script>
{#if truncationNotice}<div class="truncation-notice" role="status">{truncationNotice}</div>{/if}
`;

const IMPL_MUTANTS = [
  {
    id: 'M1',
    name: 'всегда null (уведомление никогда не появляется)',
    from: 'if (total <= shown) return null;',
    to: 'return null;',
  },
  {
    id: 'M2',
    name: 'строгое < вместо <= (уведомление при total == shown)',
    from: 'total <= shown',
    to: 'total < shown',
  },
  {
    id: 'M3',
    name: '«самые ранние» заменено на «самые поздние»',
    from: 'самые ранние',
    to: 'самые поздние',
  },
  {
    id: 'M4',
    name: 'числа в тексте переставлены местами',
    from: '${shown} из ${total}',
    to: '${total} из ${shown}',
  },
];

const SVELTE_MUTANTS = [
  {
    id: 'M5',
    name: 'литерал «Показано записей» в разметке страницы',
    from: '{truncationNotice}</div>',
    to: 'Показано записей: 1000</div>',
  },
  {
    id: 'M6',
    name: 'вместо rows.total читается rows.rows.length (чтение не того поля)',
    from: 'movementsTruncationNotice(rows.rows.length, rows.total)',
    to: 'movementsTruncationNotice(rows.rows.length, rows.rows.length)',
  },
  {
    id: 'M7',
    name: 'аргументы переставлены местами (total, длина)',
    from: 'movementsTruncationNotice(rows.rows.length, rows.total)',
    to: 'movementsTruncationNotice(rows.total, rows.rows.length)',
  },
  {
    id: 'M8',
    name: 'убран импорт movementsTruncationNotice',
    from: "import { movementsTruncationNotice } from './truncationNotice';",
    to: '',
  },
];

function assertUniqueAnchor(source, m) {
  // Якорь обязан быть в эталоне ровно один раз — иначе replace молча
  // отключит соседнюю ветку и мутант окажется неотличим от эталона.
  const occurrences = source.split(m.from).length - 1;
  if (occurrences !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — якорь мутанта ${m.id} «${m.name}» встречается ${occurrences} раз, нужен ровно 1`,
    );
    process.exit(1);
  }
}

async function selftest(cases) {
  const goodFailures = runCases(await loadModuleFromTs(GOOD, 'good.ts'), cases);
  if (goodFailures.length) {
    console.error(
      `${TAG} SELFTEST FAIL — эталон не прошёл фикстуру:\n  - ${goodFailures.join('\n  - ')}`,
    );
    process.exit(1);
  }
  const goodStructural = runStructural(GOOD_SVELTE);
  if (goodStructural.length) {
    console.error(
      `${TAG} SELFTEST FAIL — эталонная свёрстка не прошла структурные правила:\n  - ${goodStructural.join('\n  - ')}`,
    );
    process.exit(1);
  }
  for (const m of IMPL_MUTANTS) {
    assertUniqueAnchor(GOOD, m);
    // split/join, а не replace: подстановка `$`-последовательностей в replace
    // исказила бы мутант с шаблонными `${...}`.
    const mutated = GOOD.split(m.from).join(m.to);
    const failures = runCases(await loadModuleFromTs(mutated, 'mutant.ts'), cases);
    if (!failures.length) {
      console.error(
        `${TAG} SELFTEST FAIL — мутант ${m.id} «${m.name}» прошёл фикстуру: гейт его не ловит`,
      );
      process.exit(1);
    }
    console.log(`${TAG}   ${m.id} «${m.name}» пойман: ${failures[0]}`);
  }
  for (const m of SVELTE_MUTANTS) {
    assertUniqueAnchor(GOOD_SVELTE, m);
    const failures = runStructural(GOOD_SVELTE.split(m.from).join(m.to));
    if (!failures.length) {
      console.error(
        `${TAG} SELFTEST FAIL — мутант ${m.id} «${m.name}» прошёл структурные правила: гейт его не ловит`,
      );
      process.exit(1);
    }
    console.log(`${TAG}   ${m.id} «${m.name}» пойман: ${failures[0]}`);
  }
  console.log(
    `${TAG} selftest OK — эталон проходит, ${IMPL_MUTANTS.length + SVELTE_MUTANTS.length} мутантов ловятся`,
  );
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

  let svelteSrc;
  try {
    svelteSrc = fs.readFileSync(SVELTE, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${SVELTE}`);
    process.exit(1);
  }
  failures.push(...runStructural(svelteSrc));

  if (failures.length) {
    console.error(`${TAG} FAIL —\n  - ${failures.join('\n  - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK — ${cases.length} кейсов фикстуры, правила S1-S3`);
}

main();
