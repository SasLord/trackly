#!/usr/bin/env node
// [check-act-number-no-regex-split] Постоянный гейт против регенерации номера
// акта из «сырого» значения на клиенте (D-05, Фазы 40.3-03/40.3-08, находки
// N-1 / WR-08 / WR-07 аудита v1.4).
//
// Почему он существует: ReturnModal.svelte склеивал тост/предпросмотр возврата
// из `number_raw` (сырое значение) + суффикса, вычисленного через
// `.replace(/^\d+/, '')` или простой конкатенацией — код предполагал, что
// номер акта ВСЕГДА начинается с цифр. С Фазы 40.2 (NUM-14) номер акта —
// свободный текст по шаблону («2026/09-1»), поэтому такой код искажает
// отображаемый номер. Сервер уже отдаёт готовое `ActDto.number` — искажение
// чисто клиентское.
//
// 40.3-03 закрыл ОДНУ буквальную форму (тост, regex-литерал `/^\d/` внутри
// `.replace(`) и поставил узкий гейт под неё. Верификация фазы (BLOCKER-3)
// нашла: (WR-08) та же ошибка осталась строкой выше в ТОМ ЖЕ файле
// (предпросмотр `parentNumber`/`predictedSubNumber`), и (WR-07) узкий гейт
// пропускал 4 реалистичные вариации того же дефекта — группу `/^(\d+)/`,
// символьный класс `/^[0-9]+/`, вынесенную в `const` регулярку, и, что
// показательно, саму конкатенацию `${number_raw}в${sub}` БЕЗ единого
// `.replace(...)` — то есть буквальную форму WR-08.
//
// Гейт 40.3-08 заменяет узкую проверку на ДВА независимых, более широких
// семантических правила:
//   (A) DIGIT_PREFIX_REGEX_LITERAL_RE — любой regex-литерал, начинающийся с
//       anchored digit-prefix (`/^\d/`, `/^(\d+)/`, `/^[0-9]+/`), ГДЕ УГОДНО
//       в файле — не только внутри `.replace(`, поэтому ловит и inline, и
//       вынесенную в `const` форму;
//   (B) NUMBER_RAW_RE — любое употребление идентификатора `number_raw` в
//       файле, ОТСУТСТВУЮЩЕМ в explicit whitelist (`ALLOWED_NUMBER_RAW_FILES`)
//       — ловит ЛЮБУЮ форму использования сырого номера для отображения,
//       включая конкатенацию без regex вовсе.
//
// Единственный легитимный `number_raw` в проекте — `ActFormBody.svelte`
// (префилл поля номера в режиме редактирования акта, ДРУГОЙ контекст: там
// number_raw — редактируемое исходное значение, а не реконструкция для
// отображения) — явно занесён в whitelist, не пропущен бесконтрольно.
//
// Оба правила проверяются встроенным self-testом (`--selftest`) на 7
// фикстурах (5 позитивных: все известные формы дефекта; 2 негативных: чистый
// код + легитимный whitelisted usage) — доказывает полноту детекции без
// ручной проверки одной строки.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-act-number-no-regex-split.mjs              # проверить репозиторий
//   node scripts/check-act-number-no-regex-split.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-act-number-no-regex-split.mjs --selftest   # встроенный self-test (in-memory)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';
import { walk as walkRoots, UI_ROOT as ROOTS_UI } from './_roots.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-number-no-regex-split]';

// Обход файлов (.svelte/.ts, без src/bindings.ts и node_modules) — общий walk() из _roots.mjs.

// (A) Regex-литерал, начинающийся с anchored digit-prefix: `/^\d`, `/^(\d`,
// `/^[0-9]` — ловит буквальный литерал ГДЕ УГОДНО в файле (inline внутри
// `.replace(...)` ИЛИ вынесенный в `const`), независимо от группы/класса.
//
// WR-14 (40.3): обратная косая ОБЯЗАТЕЛЬНА (`/^data:/`, `/^dev\//` — не digit-prefix), а слева
// от `/` не стоит символ, после которого `/` — деление, а не начало regex-литерала.
// Комментарии вырезаются до матча (см. scanSource): строка-пояснение, цитирующая шаблон,
// не нарушение. Строковые литералы НЕ вырезаются: `row['number_raw']` — настоящее
// употребление, а не текст.
const DIGIT_PREFIX_REGEX_LITERAL_RE = /(?<![\w)\]])\/\^\(?(?:\\d|\[0-9\])/g;

// (B) Любое употребление `number_raw` вне explicit whitelist.
const NUMBER_RAW_RE = /\bnumber_raw\b/g;
const ALLOWED_NUMBER_RAW_FILES = new Set(['src/features/acts/ActFormBody.svelte']);

/** Номер строки (1-based) по индексу символа в тексте. */
function lineAt(src, index) {
  let line = 1;
  for (let i = 0; i < index; i++) {
    if (src.charCodeAt(i) === 10) line++;
  }
  return line;
}

/**
 * Чистая функция сканирования: принимает уже прочитанную строку исходника,
 * не трогает fs. Переиспользуется и реальным сканированием файлов (через
 * checkFile), и self-testом (над строками из памяти, без временных файлов).
 */
function scanSource(relPath, rawSrc, violations) {
  const src = stripComments(rawSrc); // длина и переводы строк сохраняются (номера строк верны)
  DIGIT_PREFIX_REGEX_LITERAL_RE.lastIndex = 0;
  let m;
  while ((m = DIGIT_PREFIX_REGEX_LITERAL_RE.exec(src)) !== null) {
    const line = lineAt(src, m.index);
    const snippet = src.slice(m.index, Math.min(src.length, m.index + 80)).split('\n')[0];
    violations.push(
      `${relPath}:${line}: digit-prefix regex-литерал (/^\\d, /^(\\d, /^[0-9]) — код ` +
        `предполагает, что значение (например номер акта) начинается с цифр (D-05). ` +
        `Найдено: ${snippet}`,
    );
  }

  if (!ALLOWED_NUMBER_RAW_FILES.has(relPath)) {
    NUMBER_RAW_RE.lastIndex = 0;
    let m2;
    while ((m2 = NUMBER_RAW_RE.exec(src)) !== null) {
      const line = lineAt(src, m2.index);
      violations.push(
        `${relPath}:${line}: number_raw вне whitelist — сырой номер акта используется для ` +
          `отображения/реконструкции вместо готового ActDto.number (D-05, WR-08).`,
      );
    }
  }
}

/** Тонкая обёртка: читает файл через fs и вызывает scanSource. */
function checkFile(root, relPath, violations) {
  scanSource(relPath, fs.readFileSync(path.join(root, relPath), 'utf8'), violations);
}

/**
 * Встроенный self-test: фикстуры над строками из памяти (без временных
 * файлов) — позитивных (все известные формы дефекта WR-07/WR-08) + 2
 * негативных (чистый код + легитимный whitelisted number_raw).
 */
function runSelfTest() {
  const fixtures = [
    {
      name: 'literal /^\\d+/ внутри .replace(',
      relPath: 'fixture.ts',
      src: "n.replace(/^\\d+/, '');",
      expectViolations: 1,
    },
    {
      name: 'group /^(\\d+)/ внутри .replace(',
      relPath: 'fixture.ts',
      src: "n.replace(/^(\\d+)/, '');",
      expectViolations: 1,
    },
    {
      name: 'char class /^[0-9]+/ внутри .replace(',
      relPath: 'fixture.ts',
      src: "n.replace(/^[0-9]+/, '');",
      expectViolations: 1,
    },
    {
      name: 'вынесенная в const регулярка',
      relPath: 'fixture.ts',
      src: "const RE = /^\\d+/; n.replace(RE, '');",
      expectViolations: 1,
    },
    {
      name: 'number_raw конкатенация без .replace (WR-08)',
      relPath: 'src/features/acts/FakeFile.svelte',
      src: '`${act.number_raw}в${sub}`',
      expectViolations: 1,
    },
    {
      name: 'чистый код (негатив)',
      relPath: 'fixture.ts',
      src: 'const x = n.trim();',
      expectViolations: 0,
    },
    {
      name: 'whitelisted number_raw в ActFormBody.svelte (негатив)',
      relPath: 'src/features/acts/ActFormBody.svelte',
      src: 'initialAct!.number_raw',
      expectViolations: 0,
    },
    // WR-14/IN-07 (40.3): ложные срабатывания прежнего широкого матча.
    {
      name: 'WR-14: /^data:/ — не digit-prefix, без обратной косой (негатив)',
      relPath: 'fixture.ts',
      src: 'if (/^data:/.test(u)) return;\nif (/^dev\\//.test(p)) return;\nconst d = /^default/.test(k);',
      expectViolations: 0,
    },
    {
      name: 'WR-14: комментарий, цитирующий /^\\d+/, не нарушение (негатив)',
      relPath: 'fixture.ts',
      src: "// раньше было n.replace(/^\\d+/, '')\nconst x = 1;",
      expectViolations: 0,
    },
    {
      name: 'WR-14: комментарий с number_raw вне whitelist не нарушение (негатив)',
      relPath: 'src/features/acts/FakeFile.svelte',
      src: '<!-- number_raw — сырое значение, здесь не используется -->\n<p>{act.number}</p>',
      expectViolations: 0,
    },
    {
      name: "WR-14: number_raw в строковом доступе row['number_raw'] остаётся нарушением",
      relPath: 'src/features/acts/FakeFile.svelte',
      src: "const v = row['number_raw'];",
      expectViolations: 1,
    },
    {
      name: 'WR-14: номер строки нарушения сохраняется после вырезания комментария',
      relPath: 'fixture.ts',
      src: '/* a\nb */\nconst RE = /^\\d+/;',
      expectViolations: 1,
      expectLine: 3,
    },
  ];

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSource(f.relPath, f.src, v);
    const ok =
      v.length === f.expectViolations &&
      (f.expectLine === undefined || v[0].startsWith(`${f.relPath}:${f.expectLine}:`));
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (expected ${f.expectViolations}, got ${v.length})`,
    );
    if (!ok) failed++;
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
      `${TAG} Usage: node scripts/check-act-number-no-regex-split.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    const ok = runSelfTest();
    process.exit(ok ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const srcDir = path.join(SRC_ROOT, 'src');
  if (!fs.existsSync(srcDir)) {
    console.error(`${TAG} FAIL — не найден каталог src (${srcDir}).`);
    process.exit(1);
  }

  const files = walkRoots({
    exts: ['.svelte', '.ts'],
    roots: path.resolve(SRC_ROOT) === ROOTS_UI ? null : [srcDir],
  }).map((abs) => path.relative(SRC_ROOT, abs).split(path.sep).join('/'));
  const violations = [];
  for (const rel of files) {
    checkFile(SRC_ROOT, rel, violations);
  }

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений (файлов: ${files.length})`);
  process.exit(0);
}

main();
