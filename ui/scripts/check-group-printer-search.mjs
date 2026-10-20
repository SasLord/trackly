#!/usr/bin/env node
// [check-group-printer-search] Структурный гейт call-site поиска принтеров для
// привязки к группе (GroupPrintersList.svelte, fetchCandidates).
//
// Почему он существует: поиск принтеров звал `devices.listGrouped` с
// `type_id: PRINTER_TYPE_ID`, а `list_grouped` игнорирует `type_id` (вердикт
// реестра devices_filter_registry, решение D-08a отложило его предикаты). Окно
// `limit` резалось ДО фильтра типа, и принтеры вытеснялись устройствами других
// типов. Правка call-site: один `devices.list` с `type_id`, окно режет сервер.
// Компиляционные гейты (svelte-check, eslint, build) слепы к возврату на
// `listGrouped`: оба вызова типизируются одинаково. JS-тест-раннера в проекте
// нет, поэтому call-site закрепляется грепающим гейтом (как check-groups-section).
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он ловит возврат к `listGrouped`, потерю
// `type_id: PRINTER_TYPE_ID` и появление флагов якорей, но НЕ доказывает работу
// сервера: что `list` режет окно после фильтра типа, закреплено серверным тестом
// printer_picker_filter_shape_narrows_by_type_before_the_window в
// crates/trackly-app/tests/devices_filter_registry.rs.
//
// Правила (над текстом <script> без комментариев):
//   (A) есть тело fetchCandidates, и в нём ровно один вызов `.list(`;
//   (B) в файле нет listGrouped и нет listByIds;
//   (C) в фильтре, переданном в `.list(`, есть `type_id: PRINTER_TYPE_ID`
//       (идентификатор, не литерал и не null);
//   (D) в теле fetchCandidates нет hide_anchor_members, anchors_only,
//       exclude_anchors, member_of_group_id (пикеры видят все устройства,
//       решение D-10 фазы 41.1).
// Файл или функцию не разобрать — нарушение с понятным сообщением, не молчаливый
// PASS; отсутствие файла — красный, не SKIP.
//
// Self-test (`--selftest`): чистый РЕАЛЬНЫЙ исходник обязан пройти; затем мутанты
// по реальному исходнику через mutate() с проверкой ровно одного вхождения якоря
// (иначе мутант мог бы снять гейт у соседнего фрагмента). Каждый мутант обязан
// провалить именно своё правило.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-group-printer-search.mjs              # проверить репозиторий
//   node scripts/check-group-printer-search.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-group-printer-search.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-printer-search]';

const FILE = 'src/features/groups/GroupPrintersList.svelte';
const ANCHOR_FLAGS = [
  'hide_anchor_members',
  'anchors_only',
  'exclude_anchors',
  'member_of_group_id',
];

/**
 * Затирает JS-комментарии (строчные и блочные), не трогая строковые литералы;
 * длина и переводы строк сохраняются. Комментарии буквально цитируют
 * проверяемые имена (listGrouped), и без вычистки гейт «краснел» бы по ним
 * или «проходил» по цитате при удалённом коде.
 */
function stripComments(src) {
  let out = '';
  let i = 0;
  let mode = 'code'; // 'code' | 'line' | 'block' | "'" | '"' | '`'
  while (i < src.length) {
    const c = src[i];
    const next = src[i + 1];
    if (mode === 'code') {
      if (c === '/' && next === '/') {
        mode = 'line';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === '/' && next === '*') {
        mode = 'block';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === "'" || c === '"' || c === '`') mode = c;
      out += c;
      i += 1;
      continue;
    }
    if (mode === 'line') {
      if (c === '\n') {
        mode = 'code';
        out += c;
      } else out += ' ';
      i += 1;
      continue;
    }
    if (mode === 'block') {
      if (c === '*' && next === '/') {
        mode = 'code';
        out += '  ';
        i += 2;
        continue;
      }
      out += c === '\n' ? '\n' : ' ';
      i += 1;
      continue;
    }
    // строковый литерал
    if (c === '\\') {
      out += c + (next ?? '');
      i += 2;
      continue;
    }
    if (c === mode) mode = 'code';
    out += c;
    i += 1;
  }
  return out;
}

/** Содержимое парных скобок, начиная с индекса открывающей. */
function extractBalanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return src.slice(openIdx + 1, i);
    }
  }
  return null;
}

/** Содержимое блока <script ...>...</script> (HTML-часть с апострофами в разбор не попадает). */
function scriptBlock(svelte) {
  const m = /<script\b[^>]*>([\s\S]*?)<\/script>/.exec(svelte);
  return m ? m[1] : null;
}

/** Тело `async function fetchCandidates(...) { ... }` или null. */
function fetchCandidatesBody(code) {
  const m = /\bfunction\s+fetchCandidates\s*\(/.exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

/**
 * Чистая функция сканирования: принимает уже прочитанный исходник компонента,
 * не трогает fs. Переиспользуется и реальным сканированием, и self-testом.
 * Возвращает массив строк-нарушений, каждая начинается с буквы правила.
 */
function scanSource(raw, violations) {
  const script = scriptBlock(raw);
  if (script === null) {
    violations.push(`(A) ${FILE}: нет блока <script> — гейт не может проверить поиск принтеров.`);
    return;
  }
  const code = stripComments(script);

  // (B) возврата к свёртке и круговому ходу быть не должно.
  for (const name of ['listGrouped', 'listByIds']) {
    if (new RegExp(`\\b${name}\\b`).test(code)) {
      violations.push(
        `(B) ${FILE}: найден ${name}. Поиск принтеров идёт одним devices.list: listGrouped игнорирует type_id (D-08a), listByIds нужен был только для раскрытия свёртки.`,
      );
    }
  }

  const body = fetchCandidatesBody(code);
  if (body === null) {
    violations.push(
      `(A) ${FILE}: не найдена функция fetchCandidates — гейт не может проверить call-site. Если она переименована, обнови гейт вместе с ней.`,
    );
    return;
  }

  // (A) ровно один .list( в теле.
  const calls = [...body.matchAll(/\.list\s*\(/g)];
  if (calls.length !== 1) {
    violations.push(
      `(A) ${FILE}: в fetchCandidates ${calls.length} вызовов .list(, нужен ровно один (devices.list с type_id).`,
    );
  } else {
    // (C) type_id: PRINTER_TYPE_ID в переданном фильтре.
    const parenOpen = body.indexOf('(', calls[0].index);
    const args = extractBalanced(body, parenOpen, '(', ')') ?? '';
    if (!/(?<![\w$])type_id\s*:\s*PRINTER_TYPE_ID\s*(?:,|\}|$)/.test(args)) {
      violations.push(
        `(C) ${FILE}: в фильтре devices.list нет \`type_id: PRINTER_TYPE_ID\` (идентификатор, не литерал и не null): без типа сервер вернёт устройства всех типов.`,
      );
    }
  }

  // (D) флагов якорей в пикере нет.
  for (const flag of ANCHOR_FLAGS) {
    if (new RegExp(`\\b${flag}\\b`).test(body)) {
      violations.push(
        `(D) ${FILE}: в fetchCandidates флаг якоря ${flag}. Пикеры видят все устройства (D-10 фазы 41.1).`,
      );
    }
  }
}

function checkRepo(root, violations) {
  const abs = path.join(root, FILE);
  let raw;
  try {
    raw = fs.readFileSync(abs, 'utf8');
  } catch {
    violations.push(
      `(A) ${FILE}: файл не читается (${abs}); отсутствие файла — нарушение, не SKIP.`,
    );
    return;
  }
  scanSource(raw, violations);
}

/** Замена с проверкой, что якорь встречается ровно один раз (D-26). */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
  }
  return source.replace(from, () => to);
}

function runSelfTest() {
  let real;
  try {
    real = fs.readFileSync(path.join(UI_ROOT, FILE), 'utf8');
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник не читается: ${e.message}`);
    return false;
  }
  const clean = [];
  scanSource(real, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник обязан проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  const TYPE_LINE = '            type_id: PRINTER_TYPE_ID,\n';
  let mutants;
  try {
    mutants = [
      {
        name: '(1) .list( заменён на .listGrouped(',
        src: mutate(real, '        .list(\n', '        .listGrouped(\n', 'list-grouped'),
        rules: ['A', 'B'],
      },
      {
        name: '(2) type_id: PRINTER_TYPE_ID удалён из фильтра',
        src: mutate(real, TYPE_LINE, '', 'no-type'),
        rules: ['C'],
      },
      {
        name: '(3) type_id: PRINTER_TYPE_ID заменён литералом',
        src: mutate(real, 'type_id: PRINTER_TYPE_ID,', 'type_id: 2,', 'literal-type'),
        rules: ['C'],
      },
      {
        name: '(3б) type_id: null вместо типа',
        src: mutate(real, 'type_id: PRINTER_TYPE_ID,', 'type_id: null,', 'null-type'),
        rules: ['C'],
      },
      {
        name: '(4) флаг якоря anchors_only в фильтре',
        src: mutate(
          real,
          '            group_by_condition: false,\n',
          '            group_by_condition: false,\n            anchors_only: true,\n',
          'anchor-flag',
        ),
        rules: ['D'],
      },
      {
        name: '(5) круговой ход devices.listByIds в fetchCandidates',
        src: mutate(
          real,
          '      candidates = found.items.slice(0, SEARCH_LIMIT);\n',
          '      candidates = found.items.slice(0, SEARCH_LIMIT);\n      void devices.listByIds([]);\n',
          'list-by-ids',
        ),
        rules: ['B'],
      },
      {
        name: '(6) второй вызов .list( в fetchCandidates',
        src: mutate(
          real,
          '      candidates = found.items.slice(0, SEARCH_LIMIT);\n',
          '      candidates = found.items.slice(0, SEARCH_LIMIT);\n      void devices.list({} as never, { offset: 0, limit: 1 });\n',
          'second-list',
        ),
        rules: ['A'],
      },
      {
        name: '(7) fetchCandidates переименована (разбор не молчит)',
        src: mutate(
          real,
          'async function fetchCandidates(',
          'async function fetchCandidatez(',
          'renamed',
        ),
        rules: ['A'],
      },
      {
        name: '(8) нет блока <script>',
        src: mutate(real, '<script lang="ts">', '<div lang="ts">', 'no-script').replace(
          '</script>',
          '</div>',
        ),
        rules: ['A'],
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const m of mutants) {
    const v = [];
    scanSource(m.src, v);
    const hit = m.rules.every((r) => v.some((line) => line.startsWith(`(${r})`)));
    console.error(
      `${TAG} [selftest] ${hit ? 'PASS' : 'FAIL'} — ${m.name} (ожидалось ${m.rules.join('+')}, получено ${v.length})`,
    );
    if (!hit) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  // Зелёный контроль: имена из запретного списка внутри КОММЕНТАРИЯ не краснят
  // (иначе гейт краснел бы по собственным пояснениям).
  try {
    const commented = mutate(
      real,
      '      if (seq !== searchSeq) return;\n      candidates = found.items',
      '      // раньше был listGrouped и anchors_only\n      if (seq !== searchSeq) return;\n      candidates = found.items',
      'comment-ok',
    );
    const v = [];
    scanSource(commented, v);
    const ok = v.length === 0;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — запретные имена в комментарии не краснят (ожидалось 0, получено ${v.length})`,
    );
    if (!ok) failed++;
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(
    `${TAG} [selftest] PASS — реальный исходник чист, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-group-printer-search.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  checkRepo(SRC_ROOT, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
