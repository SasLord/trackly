#!/usr/bin/env node
// [check-group-gone-silent] Структурный гейт «исчезнувшая выбранная группа не даёт
// error-тоста» (D-41, gap UAT 41.2 теста 4(a)).
//
// Почему он существует: GroupService::delete_group шлёт entities_changed раньше
// ответа groups_delete, поэтому смонтированные GroupPanel и GroupContentsTable
// перечитывают уже удалённую группу и получают NOT_FOUND. Прежде это выводило два
// красных тоста. Теперь NOT_FOUND своей группы — тишина + сигнал onGone, страница
// сбрасывает выбор и обновляет дерево.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он НЕ доказывает отсутствие тостов в живом
// приложении — это проверяется живой проверкой на десктопе (чек-лист плана 41.2-28).
//
// Правила (над текстом файлов без комментариев):
//   (A) GroupPanel.loadCard: в catch проверка 'NOT_FOUND' стоит ВЫШЕ первого
//       pushToast(; ветка NOT_FOUND без pushToast и с вызовом onGone; onGone
//       объявлен в Props как необязательная функция.
//   (B) GroupContentsTable.loadComposition: во внешнем catch проверка 'NOT_FOUND'
//       выше pushToast(, ветка без pushToast.
//   (C) GroupsPage: <GroupPanel получает onGone=; обработчик читает selected,
//       зовёт handleSelect(null) и refreshTree(; обработчик не вызывается из
//       $effect.
//
// Usage:
//   node scripts/check-group-gone-silent.mjs              # проверить репозиторий
//   node scripts/check-group-gone-silent.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-group-gone-silent.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-gone-silent]';

const PANEL_FILE = 'src/features/groups/GroupPanel.svelte';
const TABLE_FILE = 'src/features/groups/GroupContentsTable.svelte';
const PAGE_FILE = 'src/features/groups/GroupsPage.svelte';

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

/** Тело `function name(...) { ... }` или null (поддерживает async). */
function functionBody(code, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

/** Тело ПОСЛЕДНЕГО `catch (...) { ... }` в тексте или null. */
function lastCatchBody(body) {
  let last = null;
  for (const m of body.matchAll(/\bcatch\b/g)) last = m;
  if (!last) return null;
  const braceOpen = body.indexOf('{', last.index);
  if (braceOpen === -1) return null;
  return extractBalanced(body, braceOpen, '{', '}');
}

/**
 * Разбор catch-блока на «проверка NOT_FOUND выше pushToast, ветка без тоста».
 * Возвращает список текстов нарушений (без префикса файла).
 */
function checkCatch(catchBody, where) {
  const out = [];
  const nf = catchBody.indexOf("'NOT_FOUND'");
  const toast = catchBody.indexOf('pushToast(');
  if (nf === -1) {
    out.push(`${where}: в catch нет проверки кода 'NOT_FOUND' — исчезнувшая группа даст error-тост.`);
    return { problems: out, branch: null };
  }
  if (toast === -1) {
    out.push(`${where}: в catch не найден pushToast( — гейт не может проверить порядок.`);
    return { problems: out, branch: null };
  }
  if (nf > toast) {
    out.push(
      `${where}: проверка 'NOT_FOUND' стоит НИЖЕ pushToast( — тост уходит раньше проверки.`,
    );
  }
  const braceOpen = catchBody.indexOf('{', nf);
  const branch = braceOpen === -1 ? null : extractBalanced(catchBody, braceOpen, '{', '}');
  if (branch === null) {
    out.push(`${where}: не разобрана ветка NOT_FOUND (нужен блок { … }).`);
  } else if (branch.includes('pushToast(')) {
    out.push(`${where}: в ветке NOT_FOUND есть pushToast( — ветка обязана молчать.`);
  }
  return { problems: out, branch };
}

function scanPanel(raw, violations) {
  const code = stripComments(raw);
  const F = PANEL_FILE;
  if (!/\bonGone\s*\?\s*:\s*\(/.test(code)) {
    violations.push(`(A) ${F}: onGone не объявлен в Props как необязательная функция.`);
  }
  const body = functionBody(code, 'loadCard');
  if (body === null) {
    violations.push(`(A) ${F}: не найдена функция loadCard — гейт не может разобрать файл.`);
    return;
  }
  const cb = lastCatchBody(body);
  if (cb === null) {
    violations.push(`(A) ${F}: в loadCard не найден catch — не разобрать.`);
    return;
  }
  const { problems, branch } = checkCatch(cb, `${F} loadCard`);
  for (const p of problems) violations.push(`(A) ${p}`);
  if (branch !== null && !/\bonGone\b/.test(branch)) {
    violations.push(`(A) ${F}: ветка NOT_FOUND в loadCard не зовёт onGone — страница не узнает.`);
  }
}

function scanTable(raw, violations) {
  const code = stripComments(raw);
  const F = TABLE_FILE;
  const body = functionBody(code, 'loadComposition');
  if (body === null) {
    violations.push(`(B) ${F}: не найдена функция loadComposition — гейт не может разобрать файл.`);
    return;
  }
  const cb = lastCatchBody(body);
  if (cb === null) {
    violations.push(`(B) ${F}: в loadComposition не найден внешний catch — не разобрать.`);
    return;
  }
  const { problems } = checkCatch(cb, `${F} loadComposition`);
  for (const p of problems) violations.push(`(B) ${p}`);
}

function scanPage(raw, violations) {
  const code = stripComments(raw);
  const F = PAGE_FILE;
  const tagStart = code.indexOf('<GroupPanel');
  const tagEnd = tagStart === -1 ? -1 : code.indexOf('/>', tagStart);
  if (tagStart === -1 || tagEnd === -1) {
    violations.push(`(C) ${F}: не найден тег <GroupPanel … /> — не разобрать.`);
    return;
  }
  const tag = code.slice(tagStart, tagEnd);
  const m = /\bonGone\s*=\s*\{\s*(\w+)\s*\}/.exec(tag);
  if (!m) {
    violations.push(`(C) ${F}: <GroupPanel> не получает onGone={обработчик}.`);
    return;
  }
  const name = m[1];
  const body = functionBody(code, name);
  if (body === null) {
    violations.push(`(C) ${F}: не найден обработчик ${name} — не разобрать.`);
    return;
  }
  if (!/\bselected\b/.test(body)) {
    violations.push(`(C) ${F}: ${name} не читает selected — сбросит выбор чужой группы.`);
  }
  if (!/\bhandleSelect\s*\(\s*null\s*\)/.test(body)) {
    violations.push(`(C) ${F}: ${name} не зовёт handleSelect(null) — выбор не сбрасывается.`);
  }
  if (!/\brefreshTree\s*\(/.test(body)) {
    violations.push(`(C) ${F}: ${name} не зовёт refreshTree( — дерево не обновляется.`);
  }
  for (const e of code.matchAll(/\$effect(?:\.pre)?\s*\(/g)) {
    const parenOpen = code.indexOf('(', e.index);
    const eb = extractBalanced(code, parenOpen, '(', ')') ?? '';
    if (new RegExp(`\\b${name}\\s*\\(`).test(eb)) {
      violations.push(`(C) ${F}: ${name} вызывается из $effect — риск effect_update_depth_exceeded.`);
    }
  }
}

function scanSource(src, violations) {
  scanPanel(src.panel, violations);
  scanTable(src.table, violations);
  scanPage(src.page, violations);
}

function readSources(root) {
  return {
    panel: fs.readFileSync(path.join(root, PANEL_FILE), 'utf8'),
    table: fs.readFileSync(path.join(root, TABLE_FILE), 'utf8'),
    page: fs.readFileSync(path.join(root, PAGE_FILE), 'utf8'),
  };
}

/** Замена с проверкой, что якорь встречается ровно один раз. */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
  }
  return source.replace(from, () => to);
}

const PANEL_NF_BLOCK =
  "      if (parseAppError(e).code === 'NOT_FOUND') {\n        onGone?.(groupId);\n        return;\n      }\n";
const PANEL_TOAST_TAIL = "        (e as Partial<AppError> | undefined)?.message ?? 'Не удалось загрузить группу.',\n      );\n";

function runSelfTest() {
  let s;
  try {
    s = readSources(UI_ROOT);
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник не читается: ${e.message}`);
    return false;
  }
  const clean = [];
  scanSource(s, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники обязаны проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  let mutants;
  try {
    mutants = [
      {
        name: '(1) loadCard без проверки NOT_FOUND',
        src: {
          ...s,
          panel: mutate(
            s.panel,
            "parseAppError(e).code === 'NOT_FOUND'",
            "parseAppError(e).code === 'SOMETHING_ELSE'",
            'panel-no-nf',
          ),
        },
        rules: ['A'],
      },
      {
        name: '(2) мутант-перестановка: проверка NOT_FOUND переехала НИЖЕ pushToast',
        src: {
          ...s,
          panel: mutate(
            mutate(s.panel, PANEL_NF_BLOCK, '', 'panel-swap-remove'),
            PANEL_TOAST_TAIL,
            PANEL_TOAST_TAIL + PANEL_NF_BLOCK,
            'panel-swap-insert',
          ),
        },
        rules: ['A'],
      },
      {
        name: '(3) ветка NOT_FOUND в loadCard не зовёт onGone',
        src: {
          ...s,
          panel: mutate(s.panel, '        onGone?.(groupId);\n', '', 'panel-no-ongone'),
        },
        rules: ['A'],
      },
      {
        name: '(4) onGone не объявлен в Props',
        src: {
          ...s,
          panel: mutate(s.panel, 'onGone?: (', 'onGoneX?: (', 'panel-no-prop'),
        },
        rules: ['A'],
      },
      {
        name: '(5) loadComposition без проверки NOT_FOUND',
        src: {
          ...s,
          table: mutate(
            s.table,
            "parseAppError(e).code === 'NOT_FOUND'",
            "parseAppError(e).code === 'SOMETHING_ELSE'",
            'table-no-nf',
          ),
        },
        rules: ['B'],
      },
      {
        name: '(6) <GroupPanel без onGone',
        src: {
          ...s,
          page: mutate(s.page, '              onGone={handleGroupGone}\n', '', 'page-no-prop'),
        },
        rules: ['C'],
      },
      {
        name: '(7) обработчик без handleSelect(null)',
        src: {
          ...s,
          page: mutate(
            s.page,
            '      handleSelect(null);\n    }\n    refreshTree();',
            '    }\n    refreshTree();',
            'page-no-select-null',
          ),
        },
        rules: ['C'],
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
  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(
    `${TAG} [selftest] PASS — реальные исходники чисты, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const srcArg = args.find((a) => a.startsWith('--src='));
  const root = srcArg ? path.resolve(srcArg.slice('--src='.length)) : UI_ROOT;
  let sources;
  try {
    sources = readSources(root);
  } catch (e) {
    console.error(`${TAG} FAIL — не читается исходник: ${e.message}`);
    process.exit(1);
  }
  const violations = [];
  scanSource(sources, violations);
  if (violations.length > 0) {
    console.error(`${TAG} FAIL — ${violations.length} нарушений:`);
    for (const v of violations) console.error(`${TAG}   ${v}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — исчезнувшая группа не даёт error-тоста (структурно).`);
}

main();
