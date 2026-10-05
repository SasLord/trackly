#!/usr/bin/env node
// [check-property-removal] Гейт копирайта скрытия/удаления свойства типа группы
// (фаза 41, GAP-2, план 41-31).
//
// Почему он существует: `propertyRemovalCopy` — JS-зеркало серверного решения
// `delete_property` (`filled_group_count(id) > 0` — скрыть, иначе удалить
// физически, D-13). `svelte-check` / `pnpm build` не ловят ни расхождение
// порога, ни чтение не того поля: типы совпадают, а администратору молча
// обещают «можно будет вернуть» там, где свойство уже удалено навсегда.
//
// Гейт ИСПОЛНЯЮЩИЙ и состоит из двух частей.
//  1. Golden-фикстура `scripts/fixtures/property-removal/cases.json`: входы
//     (`name`, `filled_group_count`, `is_required`) и ТОЧНЫЕ ожидаемые тексты.
//     Ту же фикстуру читает Rust-тест
//     `crates/trackly-app/tests/groups_property_removal_parity.rs` и сверяет с
//     серверным исходом (`archived == (kind == "hide")`).
//  2. Структурные правила для `GroupTypePropertiesTable.svelte` (текст без
//     комментариев, пробелы и переводы строк нормализованы — Prettier
//     переносит текст кнопки на отдельную строку):
//       S1 — импорт `propertyRemovalCopy` из `./propertyRemoval`;
//       S2 — в файле нет литералов «Скрыть свойство», «можно будет вернуть»,
//            «Удалить безвозвратно» (копирайт живёт только в модуле);
//       S3 — нет литерала «Скрыть» между тегами (`>Скрыть<`): пункт меню
//            берётся из `removal.menuLabel`.
//
// `--selftest` прогоняет те же проверки на встроенных реализациях: эталон
// обязан пройти, мутанты M1-M7 — провалиться.
//
// Фикстура содержит только вымышленное имя свойства.
//
// Usage:
//   node scripts/check-property-removal.mjs                # проверить репозиторий
//   node scripts/check-property-removal.mjs --impl=<path>  # проверить копию propertyRemoval.ts
//   node scripts/check-property-removal.mjs --selftest     # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const DEFAULT_IMPL = path.join(UI_ROOT, 'src/features/groups/propertyRemoval.ts');
const SVELTE = path.join(UI_ROOT, 'src/features/groups/GroupTypePropertiesTable.svelte');
const FIXTURE = path.join(__dirname, 'fixtures/property-removal/cases.json');
const TAG = '[check-property-removal]';
const MIN_CASES = 6;

const EXPECTED_FIELDS = [
  ['kind', 'kind'],
  ['menu_label', 'menuLabel'],
  ['modal_title', 'modalTitle'],
  ['body', 'body'],
  ['confirm_label', 'confirmLabel'],
  ['confirm_variant', 'confirmVariant'],
  ['error_toast', 'errorToast'],
];

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
  if (typeof mod.propertyRemovalCopy !== 'function') {
    return ['модуль не экспортирует propertyRemovalCopy — копирайт больше никем не проверяется'];
  }
  const failures = [];
  for (const c of cases) {
    const label = `${c.name} / filled=${c.filled_group_count} required=${c.is_required}`;
    try {
      const got = mod.propertyRemovalCopy({
        name: c.name,
        filledGroupCount: c.filled_group_count,
        isRequired: c.is_required,
      });
      for (const [snake, camel] of EXPECTED_FIELDS) {
        if (got[camel] !== c.expected[snake]) {
          failures.push(
            `${label}: ${camel} = ${JSON.stringify(got[camel])}, ожидалось ${JSON.stringify(c.expected[snake])}`,
          );
        }
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
    !/import\s*\{[^}]*\bpropertyRemovalCopy\b[^}]*\}\s*from\s*['"]\.\/propertyRemoval['"]/.test(
      text,
    )
  ) {
    failures.push(
      'S1: нет импорта propertyRemovalCopy из ./propertyRemoval — таблица ушла от единого источника копирайта',
    );
  }
  for (const literal of ['Скрыть свойство', 'можно будет вернуть', 'Удалить безвозвратно']) {
    if (text.includes(literal)) {
      failures.push(
        `S2: в таблице литерал «${literal}» — копирайт должен жить только в propertyRemoval.ts`,
      );
    }
  }
  if (/>\s*Скрыть\s*</.test(text)) {
    failures.push(
      'S3: литерал «Скрыть» между тегами — пункт меню должен браться из removal.menuLabel',
    );
  }
  return failures;
}

// Эталон для --selftest (идентичен propertyRemoval.ts по поведению) и мутанты.
const GOOD = `
export function propertyRemovalCopy(input) {
  if (input.filledGroupCount > 0) {
    const required = input.isRequired ? ' Признак «Обязательное» при скрытии снимается.' : '';
    return {
      kind: 'hide',
      menuLabel: 'Скрыть',
      modalTitle: 'Скрыть свойство',
      body: \`Свойство «\${input.name}» исчезнет из форм групп. Заполненные значения останутся в базе, свойство можно будет вернуть.\${required}\`,
      confirmLabel: 'Скрыть',
      confirmVariant: 'primary',
      errorToast: 'Не удалось скрыть свойство.',
    };
  }
  return {
    kind: 'delete',
    menuLabel: 'Удалить свойство',
    modalTitle: 'Удалить свойство',
    body: \`Свойство «\${input.name}» ещё нигде не заполнено, поэтому будет удалено безвозвратно: вернуть его через «Показать скрытые» нельзя, создать можно только заново.\`,
    confirmLabel: 'Удалить безвозвратно',
    confirmVariant: 'destructive',
    errorToast: 'Не удалось удалить свойство.',
  };
}
`;

const GOOD_SVELTE = `
<script lang="ts">
  import { propertyRemovalCopy } from './propertyRemoval';
</script>
<button type="button" role="menuitem" onclick={go}>
  {removal.menuLabel}
</button>
<Modal open={true} title={removalCopy.modalTitle}>
  <Button variant={removalCopy.confirmVariant}>{removalCopy.confirmLabel}</Button>
</Modal>
`;

const IMPL_MUTANTS = [
  {
    id: 'M1',
    name: 'безусловный hide (игнорирует filledGroupCount)',
    from: 'if (input.filledGroupCount > 0) {',
    to: 'if (true) {',
  },
  {
    id: 'M2',
    name: 'порог >= 0 вместо > 0',
    from: 'input.filledGroupCount > 0',
    to: 'input.filledGroupCount >= 0',
  },
  {
    id: 'M3',
    name: 'убрана фраза про «Обязательное» в hide',
    from: "' Признак «Обязательное» при скрытии снимается.'",
    to: "''",
  },
  {
    id: 'M4',
    name: "у delete confirmVariant: 'primary'",
    from: "confirmVariant: 'destructive'",
    to: "confirmVariant: 'primary'",
  },
];

const SVELTE_MUTANTS = [
  {
    id: 'M5',
    name: 'литерал «Скрыть свойство» в разметке таблицы',
    from: 'title={removalCopy.modalTitle}',
    to: 'title="Скрыть свойство"',
  },
  {
    id: 'M6',
    name: 'пункт меню возвращён литералом «Скрыть» на отдельной строке',
    from: '{removal.menuLabel}',
    to: 'Скрыть',
  },
  {
    id: 'M7',
    name: 'убран импорт propertyRemovalCopy',
    from: "import { propertyRemovalCopy } from './propertyRemoval';",
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
    const failures = runCases(
      await loadModuleFromTs(GOOD.replace(m.from, m.to), 'mutant.ts'),
      cases,
    );
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
    const failures = runStructural(GOOD_SVELTE.replace(m.from, m.to));
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
