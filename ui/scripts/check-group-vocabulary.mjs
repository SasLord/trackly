#!/usr/bin/env node
// [check-group-vocabulary] Постоянный гейт словаря GRD-06 (фаза 41): слово
// «группа» в пользовательских надписях UI означает ОДНУ сущность — группу
// устройств (раздел «Группы»). Прежняя свёртка одинаковых устройств
// называется «Свернуть одинаковые» / «Свёрнуто: N».
//
// Почему он существует: пока одно слово означает две вещи, раздел «Группы»
// вводит пользователя в заблуждение (SPEC 14). Переименование пяти строк
// без гейта откатится при первой же правке соседнего экрана.
//
// Почему гейт сравнивает с allowlist, а не требует «ровно 0»: легитимные
// надписи о группах уже есть и будут появляться (раздел «Группы» в сайдбаре,
// подсказка D-19 в форме устройства, строка D-28 в журнале перемещений).
// Гейт «ровно 0» пришлось бы выключить в первый же день; allowlist —
// это пары «файл + маркер»: у файлов из разных частей приложения допустима
// только конкретная подстрока, а не любая надпись со словом «группа».
//
// Почему статически: JS-тест-раннера в проекте нет, вводить его нельзя;
// `svelte-check` / `eslint` / `pnpm build` словарь не проверяют.
//
// Как работает: из каждого ui/src/**/*.svelte и *.ts (кроме сгенерированного
// src/bindings.ts) вырезаются комментарии (// /* */ <!-- -->), после чего в
// оставшемся тексте ищутся корни [Гг]руп / [Сс]груп. Идентификаторы кода
// (DeviceGroup, groupExpanded) — латиница и корню не соответствуют; всё
// кириллическое, что осталось после вырезания комментариев, — это
// пользовательский литерал (строка, текст между тегами, aria-label, title,
// placeholder).
//
// Self-test (`--selftest`) прогоняет ТУ ЖЕ функцию сканирования на фикстурах
// в памяти, поэтому «гейт зелёный» отличимо от «гейт ничего не ищет».
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-group-vocabulary.mjs              # проверить репозиторий
//   node scripts/check-group-vocabulary.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-group-vocabulary.mjs --selftest   # встроенный self-test (in-memory)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-vocabulary]';

// Корни слова «группа» в пользовательских литералах.
const ROOT_RE = /[Гг]руп|[Сс]груп/g;

// Сгенерированный файл — не источник надписей.
const SKIP_FILES = new Set(['src/bindings.ts']);

// Allowlist: { file | prefix, markers }. `markers: null` — файл пропускается
// целиком (раздел «Группы» принадлежит своему словарю). Иначе допустимо
// ТОЛЬКО вхождение корня внутри одного из маркеров (узко, не весь файл).
const ALLOWLIST = [
  { prefix: 'src/features/groups/', markers: null },
  { file: 'src/lib/api/groups.ts', markers: null },
  { file: 'src/features/layout/sidebar-config.ts', markers: ['Группы'] },
  { file: 'src/features/devices/DeviceFormBody.svelte', markers: ['Место задаётся группой'] },
  {
    file: 'src/lib/components/MovementTimeline.svelte',
    markers: ['в составе группы', 'перенос группы'],
  },
  {
    // 41-25 (W-10): синтезированный заголовок пакета в отчёте «Перемещения»
    // (пакет без строки группы в видимом наборе) — тип «Группа» и причина
    // «перенос группы». Только эти два литерала, не весь файл.
    file: 'src/features/reports/ReportTable.svelte',
    markers: ['Группа', 'перенос группы'],
  },
  {
    // D-23: единственные надписи о группах в «Местах» (строка модалки массового
    // переноса и тост) — формы группа/группы/групп; свёртка одинаковых здесь не
    // упоминается. ШИРИНА записи: маркер 'групп' покрывает и строчное
    // «группировать» (корень совпадает с началом маркера), но НЕ «Группировать»
    // с заглавной и НЕ «сгруппировать» (корень начинается раньше маркера) —
    // эти формы ловит selftest. Вне блока D-23 слово «группа» в файле
    // появляться не должно.
    file: 'src/features/places/PlaceContents.svelte',
    markers: ['групп'],
  },
];

function allowEntryFor(relPath) {
  return ALLOWLIST.find((e) => (e.file ? e.file === relPath : relPath.startsWith(e.prefix)));
}

/**
 * Вырезает комментарии, сохраняя нумерацию строк. Намеренно без автомата по
 * кавычкам: в русских doc-комментариях встречаются одиночные кавычки и
 * апострофы, и строко-ориентированный разборщик «залипал» бы в литерале.
 * `//` считается комментарием только в начале строки или после пробела /
 * `;{(,`, поэтому `https://...` не затрагивается.
 */
export function stripComments(src) {
  const blank = (m) => m.replace(/[^\n]/g, ' ');
  return src
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[\s;{(,])\/\/[^\n]*/g, (m, pre) => pre + ' '.repeat(m.length - pre.length));
}

/**
 * Единственная функция сканирования — её же вызывает self-test.
 * Возвращает строки нарушений `файл:строка литерал`.
 */
export function scanSource(relPath, rawSrc, violations) {
  if (SKIP_FILES.has(relPath)) return;
  const entry = allowEntryFor(relPath);
  if (entry && entry.markers === null) return;

  const text = stripComments(rawSrc);
  const spans = [];
  for (const marker of entry?.markers ?? []) {
    let from = 0;
    for (;;) {
      const at = text.indexOf(marker, from);
      if (at === -1) break;
      spans.push([at, at + marker.length]);
      from = at + marker.length;
    }
  }

  ROOT_RE.lastIndex = 0;
  let m;
  while ((m = ROOT_RE.exec(text)) !== null) {
    const idx = m.index;
    if (spans.some(([a, b]) => idx >= a && idx < b)) continue;
    const line = text.slice(0, idx).split('\n').length;
    const lineText = text.split('\n')[line - 1].trim();
    violations.push(`${relPath}:${line} ${lineText}`);
  }
}

function walk(dir, out) {
  for (const name of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, name.name);
    if (name.isDirectory()) {
      if (name.name === 'node_modules' || name.name === 'dist') continue;
      walk(full, out);
    } else if (name.name.endsWith('.svelte') || name.name.endsWith('.ts')) {
      out.push(full);
    }
  }
}

function scanRoot(root) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) return { violations: [`${TAG} нет каталога ${srcDir}`], files: 0 };
  const files = [];
  walk(srcDir, files);
  const violations = [];
  for (const f of files) {
    const rel = path.relative(root, f).split(path.sep).join('/');
    scanSource(rel, fs.readFileSync(f, 'utf8'), violations);
  }
  return { violations, files: files.length };
}

function runSelfTest() {
  const fixtures = [
    {
      name: 'негатив: «Группировать» вне allowlist',
      file: 'src/features/devices/Foo.svelte',
      src: '<label>Группировать похожие</label>',
      expectViolations: 1,
    },
    {
      name: 'негатив: «Группировать» в DeviceFormBody (маркер узкий)',
      file: 'src/features/devices/DeviceFormBody.svelte',
      src: '<span>Место задаётся группой</span>\n<label>Группировать</label>',
      expectViolations: 1,
    },
    {
      name: 'негатив: aria-label со словом «группу» в общем компоненте',
      file: 'src/lib/components/TableRow.svelte',
      src: "<button aria-label={open ? 'Свернуть группу' : 'Развернуть'}></button>",
      expectViolations: 1,
    },
    {
      name: 'негатив: «Сгруппировать» в .ts вне allowlist',
      file: 'src/lib/foo.ts',
      src: "export const L = 'Сгруппировать';",
      expectViolations: 1,
    },
    {
      name: 'позитив: «Место задаётся группой» в DeviceFormBody',
      file: 'src/features/devices/DeviceFormBody.svelte',
      src: '<p>Место задаётся группой</p>',
      expectViolations: 0,
    },
    {
      name: 'позитив: «в составе группы» в MovementTimeline',
      file: 'src/lib/components/MovementTimeline.svelte',
      src: "<span>{'в составе группы'}</span>",
      expectViolations: 0,
    },
    {
      name: 'позитив: «перенос группы» в MovementTimeline',
      file: 'src/lib/components/MovementTimeline.svelte',
      src: "<span>{'перенос группы'}</span>",
      expectViolations: 0,
    },
    {
      name: 'негатив: «перенос группы» в любом другом файле',
      file: 'src/features/places/PlaceEntityViewModal.svelte',
      src: "<span>{'перенос группы'}</span>",
      expectViolations: 1,
    },
    {
      name: 'негатив: прочее слово «группа» в MovementTimeline (маркеры узкие)',
      file: 'src/lib/components/MovementTimeline.svelte',
      src: "<span>{'перенос группы'}</span>\n<span>Удалить группу</span>",
      expectViolations: 1,
    },
    {
      name: 'позитив: литерал «Группа» в ReportTable (синтезированный заголовок пакета)',
      file: 'src/features/reports/ReportTable.svelte',
      src: "<td>{'Группа'}</td>",
      expectViolations: 0,
    },
    {
      name: 'позитив: литерал «перенос группы» в ReportTable',
      file: 'src/features/reports/ReportTable.svelte',
      src: "const reason = 'перенос группы';",
      expectViolations: 0,
    },
    {
      name: 'негатив: «Группировать» в ReportTable (маркеры узкие)',
      file: 'src/features/reports/ReportTable.svelte',
      src: "<td>{'Группа'}</td>\n<label>Группировать</label>",
      expectViolations: 1,
    },
    {
      name: 'негатив: «перенос группы» в DeviceList (другой файл)',
      file: 'src/features/devices/DeviceList.svelte',
      src: "const reason = 'перенос группы';",
      expectViolations: 1,
    },
    {
      name: 'позитив: формы группа/группы/групп в PlaceContents (D-23)',
      file: 'src/features/places/PlaceContents.svelte',
      src: "<p>переедут {n} {pluralizeRu(n, ['группа', 'группы', 'групп'])}</p>",
      expectViolations: 0,
    },
    {
      name: 'негатив: «Группировать» в PlaceContents (заглавная форма вне маркера)',
      file: 'src/features/places/PlaceContents.svelte',
      src: '<p>группы</p>\n<label>Группировать</label>',
      expectViolations: 1,
    },
    {
      name: 'негатив: «Сгруппировать» в PlaceContents',
      file: 'src/features/places/PlaceContents.svelte',
      src: '<label>Сгруппировать</label>',
      expectViolations: 1,
    },
    {
      name: 'негатив: «группы» из D-23 в другом файле «Мест»',
      file: 'src/features/places/PlaceMoveModal.svelte',
      src: '<p>переедут группы</p>',
      expectViolations: 1,
    },
    {
      name: 'позитив: «Группы» в sidebar-config',
      file: 'src/features/layout/sidebar-config.ts',
      src: "{ label: 'Группы' }",
      expectViolations: 0,
    },
    {
      name: 'позитив: любая надпись внутри features/groups',
      file: 'src/features/groups/GroupsPage.svelte',
      src: '<h1>Группы устройств</h1>',
      expectViolations: 0,
    },
    {
      name: 'позитив: идентификатор DeviceGroup и комментарии вне allowlist',
      file: 'src/features/devices/Bar.svelte',
      src: [
        '<script lang="ts">',
        '  // группа одинаковых устройств',
        "  /** Раскрываемость группы (id'ы под-группы) */",
        '  let g: DeviceGroup | null = null;',
        '</script>',
        '<!-- группа -->',
        '<div>{g?.count}</div>',
      ].join('\n'),
      expectViolations: 0,
    },
  ];

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSource(f.file, f.src, v);
    const ok = v.length === f.expectViolations;
    if (!ok) failed += 1;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (expected ${f.expectViolations}, got ${v.length})`,
    );
    if (!ok) for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
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
  if (args.includes('--help')) {
    console.error(
      `${TAG} Usage: node scripts/check-group-vocabulary.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const { violations, files } = scanRoot(root);
  if (violations.length > 0) {
    console.error(`${TAG} FAIL — слово «группа» вне разрешённых мест (GRD-06):`);
    for (const v of violations) console.error(`  ${v}`);
    console.error(
      `${TAG} Подсказка: вынеси надпись в features/groups или добавь пару файл+маркер в allowlist осознанно.`,
    );
    process.exit(1);
  }
  console.error(`${TAG} OK — проверено файлов: ${files}, нарушений нет.`);
  process.exit(0);
}

main();
