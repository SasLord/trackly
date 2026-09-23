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

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-number-no-regex-split]';

// Относительные (от корня ui/) POSIX-пути файлов, исключённых из сканирования.
const EXCLUDED_FILES = new Set(['src/bindings.ts']);
const EXCLUDED_DIRS = new Set(['node_modules']);
const EXTENSIONS = new Set(['.svelte', '.ts']);

// (A) Regex-литерал, начинающийся с anchored digit-prefix: `/^\d`, `/^(\d`,
// `/^[0-9]` — ловит буквальный литерал ГДЕ УГОДНО в файле (inline внутри
// `.replace(...)` ИЛИ вынесенный в `const`), независимо от группы/класса.
const DIGIT_PREFIX_REGEX_LITERAL_RE = /\/\^\(?\\?(?:d|\[0-9\])/g;

// (B) Любое употребление `number_raw` вне explicit whitelist.
const NUMBER_RAW_RE = /\bnumber_raw\b/g;
const ALLOWED_NUMBER_RAW_FILES = new Set(['src/features/acts/ActFormBody.svelte']);

/** Рекурсивно собрать файлы нужных расширений под `root/dir` (POSIX-пути от root). */
function collectFiles(root, dir, out) {
  const abs = path.join(root, dir);
  let entries;
  try {
    entries = fs.readdirSync(abs, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const entry of entries) {
    const rel = dir ? `${dir}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      if (EXCLUDED_DIRS.has(entry.name)) continue;
      collectFiles(root, rel, out);
      continue;
    }
    if (!EXTENSIONS.has(path.extname(entry.name))) continue;
    if (EXCLUDED_FILES.has(rel)) continue;
    out.push(rel);
  }
  return out;
}

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
function scanSource(relPath, src, violations) {
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
 * Встроенный self-test: 7 фикстур над строками из памяти (без временных
 * файлов) — 5 позитивных (все известные формы дефекта WR-07/WR-08) + 2
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
  ];

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSource(f.relPath, f.src, v);
    const ok = v.length === f.expectViolations;
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

  const files = collectFiles(SRC_ROOT, 'src', []);
  const violations = [];
  for (const rel of files) {
    checkFile(SRC_ROOT, rel, violations);
  }

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
