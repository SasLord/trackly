#!/usr/bin/env node
// [check-placepath-parity] Постоянный гейт паритета двух реализаций ОДНОЙ формулы
// сокращения пути места (Phase 39.1, PLC-07/PLC-08, D-13..D-16).
//
// Почему он существует: формула живёт в двух местах —
//   Rust — `trackly_core::domain::places::shorten_place_path` (боевая, D-01: всё,
//          что видит пользователь в списках и отчётах, сокращает бэкенд);
//   JS   — `ui/src/lib/utils/placePath.ts::previewShortenPath` (офлайн-зеркало
//          ТОЛЬКО для живого предпросмотра в «Настройки → Организация», D-11).
// Зеркало уже разошлось с оригиналом: на 2-сегментном пути под вариантом `last`
// (WR-03 из 39.1-REVIEW.md) — уехало в релиз, найдено ревьюером, починено руками
// в коммите 10707242, и после починки НИ ОДИН гейт не удерживал паритет.
// `svelte-check` тут бессилен по конструкции: типы совпадают, расходится формула.
// Собственный комментарий в placePath.ts честно признавал: «нет JS-тестов —
// проверять глазами при правке». Этот скрипт закрывает ровно ту дыру.
//
// Гейт ИСПОЛНЯЮЩИЙ, а не структурный: он реально вызывает `previewShortenPath` на
// каждом кейсе общей golden-фикстуры
//   scripts/fixtures/place-path/shorten-cases.json
// Ту же фикстуру с той же стороны проверяет Rust:
//   cargo test -p trackly-core shorten_place_path_matches_golden_fixture
// Пришпилены обе стороны — уехать может только фикстура, и то громко.
//
// Зависимости: `typescript` (уже прямой devDependency в ui/package.json, ставится
// тем же `pnpm install --frozen-lockfile`, что и всё остальное) — нужен, чтобы
// снять аннотации типов с .ts перед исполнением. Node 20-совместимо: никакого
// --experimental-strip-types, который появился только в 22.18.
//
// Usage:
//   node scripts/check-placepath-parity.mjs               # проверить репозиторий
//   node scripts/check-placepath-parity.mjs --impl=<path> # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(UI_ROOT, '..');
const DEFAULT_IMPL = path.join(UI_ROOT, 'src/lib/utils/placePath.ts');
const FIXTURE = path.join(REPO_ROOT, 'scripts/fixtures/place-path/shorten-cases.json');
const TAG = '[check-placepath-parity]';
const MIN_CASES = 20;

function parseArgs(argv) {
  const args = { impl: DEFAULT_IMPL };
  for (const arg of argv) {
    if (arg.startsWith('--impl='))
      args.impl = path.resolve(process.cwd(), arg.slice('--impl='.length));
    else if (arg === '--help' || arg === '-h') args.help = true;
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
      `${TAG} FAIL — не разрешается пакет \`typescript\`. Он — прямой devDependency ui/package.json; ` +
        'запусти `pnpm install` в ui/.',
    );
    process.exit(1);
  }
  const out = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: label,
    reportDiagnostics: false,
  });
  return out.outputText;
}

async function loadImpl(implPath) {
  let tsSource;
  try {
    tsSource = fs.readFileSync(implPath, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${implPath}`);
    process.exit(1);
  }
  const js = transpileToEsm(tsSource, path.basename(implPath));
  // data:-URL вместо временного файла: модуль самодостаточен (ни одного import),
  // поэтому разрешать относительные пути не требуется.
  const mod = await import(
    `data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`
  );
  if (typeof mod.previewShortenPath !== 'function') {
    console.error(
      `${TAG} FAIL — ${path.relative(REPO_ROOT, implPath)} не экспортирует функцию previewShortenPath. ` +
        'Её переименовали или удалили — паритет с Rust больше никем не удерживается.',
    );
    process.exit(1);
  }
  return mod.previewShortenPath;
}

function loadFixture() {
  let raw;
  try {
    raw = fs.readFileSync(FIXTURE, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не найдена фикстура ${path.relative(REPO_ROOT, FIXTURE)}`);
    process.exit(1);
  }
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    console.error(`${TAG} FAIL — фикстура не парсится как JSON: ${e.message}`);
    process.exit(1);
  }
  const cases = Array.isArray(parsed.cases) ? parsed.cases : [];
  if (cases.length < MIN_CASES) {
    console.error(
      `${TAG} FAIL — в фикстуре ${cases.length} кейсов, ожидалось не меньше ${MIN_CASES}. ` +
        'Её урезали вместо того, чтобы починить формулу?',
    );
    process.exit(1);
  }
  return cases;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.error(
      `${TAG} Usage: node scripts/check-placepath-parity.mjs [--impl=<path-to-placePath.ts>]`,
    );
    process.exit(0);
  }

  const cases = loadFixture();
  const previewShortenPath = await loadImpl(args.impl);
  const label = path.relative(REPO_ROOT, args.impl);

  const failures = [];
  for (const c of cases) {
    let actual;
    try {
      actual = previewShortenPath(c.full_path, c.variant, c.sep_ends, c.sep_last_two);
    } catch (e) {
      failures.push({ c, actual: `<исключение: ${e.message}>` });
      continue;
    }
    if (actual !== c.expected) failures.push({ c, actual });
  }

  for (const { c, actual } of failures) {
    console.error(
      `${TAG} ${label} — кейс ${c.id}: previewShortenPath(${JSON.stringify(c.full_path)}, ` +
        `${JSON.stringify(c.variant)}, ${JSON.stringify(c.sep_ends)}, ${JSON.stringify(c.sep_last_two)}) ` +
        `вернул ${JSON.stringify(actual)}, фикстура ожидает ${JSON.stringify(c.expected)}`,
    );
    if (c.note) console.error(`${TAG}   ${c.note}`);
  }

  if (failures.length > 0) {
    console.error(
      `${TAG} FAIL — ${failures.length} из ${cases.length} кейсов разошлись с golden-фикстурой. ` +
        'JS-зеркало предпросмотра рассинхронизировалось с боевой Rust-формулой ' +
        '(shorten_place_path) — этот класс дефекта уже уезжал в релиз (WR-03). ' +
        'Правь зеркало, а не фикстуру: фикстуру одновременно стережёт ' +
        '`cargo test -p trackly-core shorten_place_path_matches_golden_fixture`.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — ${cases.length} кейсов, 0 расхождений`);
  process.exit(0);
}

main();
