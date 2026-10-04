#!/usr/bin/env node
// [check-reorder] Гейт чистых функций порядка `insertionIndex` / `reorder`
// (фаза 41, D-10: перестановка свойств типа группы).
//
// Почему он существует: сам жест перетаскивания компиляционные гейты не видят
// (`svelte-check` / `eslint` / `pnpm build` зелёные на сломанном драге), зато
// чистые функции без DOM и рун проверяемы. Ошибка в них — это потерянное или
// продублированное свойство при перестановке, причём молча: типы совпадают,
// расходится арифметика индексов (классика: забыли сдвиг `to - 1` при переносе
// вниз). JS-тест-раннера в проекте нет, поэтому — отдельный node-гейт по
// образцу `check-placepath-parity.mjs`.
//
// Гейт ИСПОЛНЯЮЩИЙ: транспилирует `src/lib/utils/reorder.ts` пакетом
// `typescript` (CI на Node 20 не исполняет .ts напрямую), вызывает функции на
// каждом кейсе golden-фикстуры `scripts/fixtures/reorder/cases.json` и
// сравнивает результат с ожидаемым. Дополнительно для `reorder` проверяется,
// что исходный массив не мутирован и возвращён НОВЫЙ экземпляр.
//
// `--selftest` прогоняет те же проверки на встроенных реализациях: верная
// обязана пройти, три мутанта (без сдвига `to`, лишний сдвиг, нестрогое
// сравнение середины строки) — провалиться. Так «гейт зелёный» отличимо от
// «гейт ничего не ищет».
//
// Фикстура содержит только числа и буквы-заглушки (никаких реальных данных).
//
// Usage:
//   node scripts/check-reorder.mjs                # проверить репозиторий
//   node scripts/check-reorder.mjs --impl=<path>  # проверить копию reorder.ts
//   node scripts/check-reorder.mjs --selftest     # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const DEFAULT_IMPL = path.join(UI_ROOT, 'src/lib/utils/reorder.ts');
const FIXTURE = path.join(__dirname, 'fixtures/reorder/cases.json');
const TAG = '[check-reorder]';
const MIN_CASES = 14;

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
  return cases;
}

/** Возвращает список текстов провалов (пустой — всё в порядке). */
function runCases(mod, cases) {
  const failures = [];
  for (const fn of ['insertionIndex', 'reorder']) {
    if (typeof mod[fn] !== 'function') {
      failures.push(`модуль не экспортирует функцию ${fn} — порядок больше никем не проверяется`);
    }
  }
  if (failures.length) return failures;

  const fnsCovered = new Set();
  for (const c of cases) {
    fnsCovered.add(c.fn);
    try {
      if (c.fn === 'insertionIndex') {
        const got = mod.insertionIndex(c.input.rects, c.input.pointerY);
        if (got !== c.expected) {
          failures.push(
            `${c.name}: insertionIndex вернул ${JSON.stringify(got)}, ожидалось ${JSON.stringify(c.expected)}`,
          );
        }
      } else if (c.fn === 'reorder') {
        const before = JSON.stringify(c.input.items);
        const items = JSON.parse(before);
        const got = mod.reorder(items, c.input.from, c.input.to);
        if (JSON.stringify(got) !== JSON.stringify(c.expected)) {
          failures.push(
            `${c.name}: reorder вернул ${JSON.stringify(got)}, ожидалось ${JSON.stringify(c.expected)}`,
          );
        }
        if (JSON.stringify(items) !== before) {
          failures.push(
            `${c.name}: reorder мутировал исходный массив (${before} -> ${JSON.stringify(items)})`,
          );
        }
        if (got === items) {
          failures.push(`${c.name}: reorder вернул тот же экземпляр массива, ожидался новый`);
        }
      } else {
        failures.push(`${c.name}: неизвестная функция «${c.fn}» в фикстуре`);
      }
    } catch (e) {
      failures.push(`${c.name}: исключение ${e.message}`);
    }
  }
  for (const fn of ['insertionIndex', 'reorder']) {
    if (!fnsCovered.has(fn)) failures.push(`в фикстуре нет ни одного кейса для ${fn}`);
  }
  return failures;
}

// Эталон для --selftest (идентичен reorder.ts) и три мутанта.
const GOOD = `
export function insertionIndex(rects, pointerY) {
  for (let i = 0; i < rects.length; i++) {
    if (pointerY < rects[i].top + rects[i].height / 2) return i;
  }
  return rects.length;
}
export function reorder(items, from, to) {
  const next = items.slice();
  if (from < 0 || from >= next.length) return next;
  const [moved] = next.splice(from, 1);
  next.splice(to > from ? to - 1 : to, 0, moved);
  return next;
}
`;
const MUTANTS = [
  { name: 'без сдвига to при переносе вниз', from: 'to > from ? to - 1 : to', to: 'to' },
  { name: 'лишний сдвиг to при переносе вверх', from: 'to > from ? to - 1 : to', to: 'to - 1' },
  {
    name: 'нестрогое сравнение середины строки',
    from: 'pointerY < rects[i]',
    to: 'pointerY <= rects[i]',
  },
];

async function selftest(cases) {
  const goodFailures = runCases(await loadModuleFromTs(GOOD, 'good.ts'), cases);
  if (goodFailures.length) {
    console.error(
      `${TAG} SELFTEST FAIL — эталон не прошёл фикстуру:\n  - ${goodFailures.join('\n  - ')}`,
    );
    process.exit(1);
  }
  for (const m of MUTANTS) {
    // Якорь обязан быть в эталоне ровно один раз — иначе replace молча
    // отключит соседнюю функцию и мутант окажется неотличим от эталона.
    const occurrences = GOOD.split(m.from).length - 1;
    if (occurrences !== 1) {
      console.error(
        `${TAG} SELFTEST FAIL — якорь мутанта «${m.name}» встречается ${occurrences} раз, нужен ровно 1`,
      );
      process.exit(1);
    }
    const mutated = GOOD.replace(m.from, m.to);
    const failures = runCases(await loadModuleFromTs(mutated, 'mutant.ts'), cases);
    if (!failures.length) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» прошёл фикстуру: гейт его не ловит`);
      process.exit(1);
    }
  }
  console.log(`${TAG} selftest OK — эталон проходит, ${MUTANTS.length} мутанта ловятся`);
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
  if (failures.length) {
    console.error(`${TAG} FAIL —\n  - ${failures.join('\n  - ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK — ${cases.length} кейсов фикстуры`);
}

main();
