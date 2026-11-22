#!/usr/bin/env node
// [check-group-tree-context-menu] Постоянный гейт контекстного меню строки дерева
// групп (фаза 41.8, GRW-03 / D-06): правый клик по строке узла открывает ТО ЖЕ
// меню, что кебаб «⋯» этой строки — меню группы у узла-группы, меню типа у
// узла-типа.
//
// Почему он существует: `oncontextmenu` на строке, `preventDefault` в
// обработчике и выбор меню через `bind:this` — три независимых места в
// GroupTreeNode.svelte; убрать или перепутать любое можно, не сломав ни
// `svelte-check`, ни `eslint`, ни `pnpm build`. JS-тест-раннера нет, поэтому —
// структурный гейт со встроенным self-testом (как check-groups-section.mjs).
//
// Правила (по тексту БЕЗ комментариев, `_strip.mjs`):
//   (A) открывающий тег строки узла (`class="group-tree-row"`, role="treeitem")
//       несёт `oncontextmenu={<обработчик>}`;
//   (B) обработчик — функция `<обработчик>(<e>: MouseEvent)`, в ней на ВЕРХНЕМ
//       уровне тела, в порядке: `const <m> = <селектор>()`, охрана
//       `if (!<m>) return;`, `<e>.preventDefault()`, `<m>.openAt(<e>.clientX,
//       <e>.clientY)`. Порядок несущий: preventDefault ДО охраны подавил бы
//       родное меню там, где своего нет (D-06); preventDefault внутри охраны или
//       после return — родное меню браузера всплывало бы поверх нашего. Счёт
//       «preventDefault есть в теле» эти перестановки не видит;
//   (C) селектор возвращает `typeMenu` при `showTypeMenu` и `groupMenu` при
//       `showGroupMenu` (и ничего не путает местами);
//   (D) `showTypeMenu` выводится из `item.kind === 'type'`, `showGroupMenu` — из
//       `item.kind === 'group'`;
//   (E) `bind:this={typeMenu}` ровно один и стоит в ветке `{#if showTypeMenu`,
//       `bind:this={groupMenu}` ровно один и стоит в ветке `{:else if
//       showGroupMenu` — т. е. контекстное меню и кебаб — ОДИН экземпляр
//       ActionMenu, отсюда «тот же набор действий» по построению.
//
// Self-test (`--selftest`): позитив по реальному исходнику + мутанты реального
// исходника. Якорь каждой мутации обязан встречаться ровно один раз (иначе
// мутант мог бы задеть соседнюю функцию — handleChevronClick тоже зовёт
// preventDefault/stopPropagation). Негативная фикстура обязана провалить гейт
// именно СВОИМ правилом (по метке [A]..[E]).
//
// Чего гейт НЕ видит (честно): рантайм ActionMenu.openAt (позиционирование,
// фокус) и права (canMutate/canEditTypes) — это вне GRW-03.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-group-tree-context-menu.mjs              # проверить репозиторий
//   node scripts/check-group-tree-context-menu.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-group-tree-context-menu.mjs --selftest   # мутанты реального исходника

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-tree-context-menu]';
const NODE_FILE = 'src/features/groups/GroupTreeNode.svelte';

/** Индекс парной закрывающей скобки или -1. */
function matchBrace(src, openIdx, open = '{', close = '}') {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === open) depth++;
    else if (src[i] === close) {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/** Открывающий тег элемента, начиная с `<`: до `>` вне `{...}`. */
function openingTag(src, ltIdx) {
  let depth = 0;
  for (let i = ltIdx; i < src.length; i++) {
    const c = src[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return src.slice(ltIdx, i + 1);
  }
  return null;
}

/** Тело `function <name>(...)` и имя первого параметра, либо null. */
function functionBody(code, name) {
  const re = new RegExp(`function\\s+${name}\\s*\\(([^)]*)\\)[^{]*\\{`);
  const m = re.exec(code);
  if (!m) return null;
  const open = m.index + m[0].length - 1;
  const close = matchBrace(code, open);
  if (close === -1) return null;
  const param = /^\s*(\w+)/.exec(m[1])?.[1] ?? null;
  return { body: code.slice(open + 1, close), params: m[1], param };
}

/** Текст тела, где всё внутри вложенных `{...}` затёрто пробелами (верхний уровень). */
function topLevelOnly(body) {
  const out = body.split('');
  let depth = 0;
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if (c === '{') {
      depth++;
      out[i] = ' ';
    } else if (c === '}') {
      depth--;
      out[i] = ' ';
    } else if (depth > 0 && c !== '\n') out[i] = ' ';
  }
  return out.join('');
}

const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** Чистая функция: принимает исходник, складывает нарушения с метками [A]..[E]. */
function scanNode(raw, violations) {
  const code = stripComments(raw);
  const v = (rule, msg) => violations.push(`[${rule}] ${NODE_FILE}: ${msg}`);

  // --- (A) строка узла несёт oncontextmenu ----------------------------------
  const rowAttr = code.indexOf('class="group-tree-row"');
  const lt = rowAttr === -1 ? -1 : code.lastIndexOf('<', rowAttr);
  const tag = lt === -1 ? null : openingTag(code, lt);
  if (tag === null) {
    v(
      'A',
      'не найден открывающий тег строки `class="group-tree-row"` — гейт не может проверить меню.',
    );
    return;
  }
  if (!/role="treeitem"/.test(tag)) {
    v('A', 'строка `group-tree-row` без role="treeitem" — гейт смотрит не на строку узла.');
  }
  const handler = /\soncontextmenu=\{(\w+)\}/.exec(tag)?.[1] ?? null;
  if (handler === null) {
    v(
      'A',
      'у строки узла нет `oncontextmenu={…}` — правый клик не открывает меню строки (GRW-03).',
    );
    return;
  }

  // --- (B) обработчик: селектор → охрана → preventDefault → openAt -----------
  const fn = functionBody(code, handler);
  if (fn === null) {
    v('B', `не найдена функция \`${handler}\` из oncontextmenu.`);
    return;
  }
  const e = fn.param;
  if (!e || !/MouseEvent/.test(fn.params)) {
    v('B', `\`${handler}\` не принимает MouseEvent первым параметром.`);
    return;
  }
  const top = topLevelOnly(fn.body);
  const selM = /const\s+(\w+)\s*=\s*(\w+)\(\s*\)\s*;/.exec(top);
  if (!selM) {
    v('C', `в \`${handler}\` меню не берётся из функции-селектора (\`const m = activeMenu()\`).`);
    return;
  }
  const [, menuVar, selector] = selM;
  const guardRe = new RegExp(`if\\s*\\(\\s*!${esc(menuVar)}\\s*\\)\\s*return\\s*;`);
  const pdRe = new RegExp(`\\b${esc(e)}\\.preventDefault\\(\\s*\\)`);
  const openRe = new RegExp(
    `\\b${esc(menuVar)}\\.openAt\\(\\s*${esc(e)}\\.clientX\\s*,\\s*${esc(e)}\\.clientY\\s*\\)`,
  );
  const iSel = selM.index;
  const iGuard = guardRe.exec(top)?.index ?? -1;
  const iPd = pdRe.exec(top)?.index ?? -1;
  const iOpen = openRe.exec(top)?.index ?? -1;
  if (iOpen === -1) {
    v(
      'B',
      `\`${handler}\` не вызывает \`${menuVar}.openAt(${e}.clientX, ${e}.clientY)\` на верхнем уровне.`,
    );
  }
  if (iPd === -1) {
    v(
      'B',
      `\`${handler}\` не вызывает \`${e}.preventDefault()\` на верхнем уровне тела — ` +
        `родное меню браузера всплывёт поверх меню строки (или подавляется не там).`,
    );
  }
  if (iGuard === -1) {
    v(
      'B',
      `в \`${handler}\` нет охраны \`if (!${menuVar}) return;\` — при пустом меню (нет прав) нечего открывать (D-06).`,
    );
  }
  if (iPd !== -1 && iOpen !== -1 && iGuard !== -1) {
    if (!(iSel < iGuard && iGuard < iPd && iPd < iOpen)) {
      v(
        'B',
        `порядок в \`${handler}\` нарушен: ожидается селектор → охрана → preventDefault → openAt ` +
          `(получено: селектор ${iSel}, охрана ${iGuard}, preventDefault ${iPd}, openAt ${iOpen}).`,
      );
    }
  }

  // --- (C) селектор меню ----------------------------------------------------
  const sel = functionBody(code, selector);
  if (sel === null) {
    v('C', `не найдена функция-селектор \`${selector}\`.`);
  } else {
    if (!/if\s*\(\s*showTypeMenu\s*\)\s*return\s+typeMenu\s*;/.test(sel.body)) {
      v(
        'C',
        `\`${selector}\` не возвращает \`typeMenu\` при \`showTypeMenu\` — узел-тип открыл бы чужое меню.`,
      );
    }
    if (!/if\s*\(\s*showGroupMenu\s*\)\s*return\s+groupMenu\s*;/.test(sel.body)) {
      v(
        'C',
        `\`${selector}\` не возвращает \`groupMenu\` при \`showGroupMenu\` — узел-группа открыл бы чужое меню.`,
      );
    }
  }

  // --- (D) флаги меню от kind узла -----------------------------------------
  if (!/const\s+showTypeMenu\s*=\s*\$derived\(\s*item\.kind\s*===\s*'type'/.test(code)) {
    v('D', "`showTypeMenu` не выводится из `item.kind === 'type'`.");
  }
  if (!/const\s+showGroupMenu\s*=\s*\$derived\(\s*item\.kind\s*===\s*'group'/.test(code)) {
    v('D', "`showGroupMenu` не выводится из `item.kind === 'group'`.");
  }

  // --- (E) один экземпляр ActionMenu на ветку ------------------------------
  const count = (s) => code.split(s).length - 1;
  const bType = 'bind:this={typeMenu}';
  const bGroup = 'bind:this={groupMenu}';
  if (count(bType) !== 1 || count(bGroup) !== 1) {
    v(
      'E',
      `ожидается ровно по одному \`${bType}\` и \`${bGroup}\` ` +
        `(получено ${count(bType)} и ${count(bGroup)}) — контекстное меню не совпадает с кебабом.`,
    );
  } else {
    const iIfType = code.indexOf('{#if showTypeMenu');
    const iElseGroup = code.indexOf('{:else if showGroupMenu');
    const iEnd = iElseGroup === -1 ? -1 : code.indexOf('{/if}', iElseGroup);
    const iBT = code.indexOf(bType);
    const iBG = code.indexOf(bGroup);
    if (iIfType === -1 || iElseGroup === -1 || iEnd === -1) {
      v('E', 'не найдены ветки `{#if showTypeMenu` / `{:else if showGroupMenu` кебаб-меню.');
    } else if (!(iIfType < iBT && iBT < iElseGroup && iElseGroup < iBG && iBG < iEnd)) {
      v(
        'E',
        `\`${bType}\` должен стоять в ветке showTypeMenu, \`${bGroup}\` — в ветке showGroupMenu ` +
          `(меню типа и меню группы перепутаны).`,
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

/** Мутация с проверкой уникальности якоря. */
function mutate(src, from, to) {
  const count = src.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`якорь мутации встречается ${count} раз(а), ожидалось ровно 1: ${from.trim()}`);
  }
  return src.replace(from, to);
}

function runSelfTest() {
  const src = fs.readFileSync(path.join(UI_ROOT, NODE_FILE), 'utf8');
  const HANDLER_BODY =
    '    const menu = activeMenu();\n' +
    '    if (!menu) return;\n' +
    '    e.preventDefault();\n' +
    '    e.stopPropagation();\n' +
    '    menu.openAt(e.clientX, e.clientY);\n';
  const cases = [];
  try {
    cases.push({ name: 'реальный исходник (позитив)', s: src, rule: null });
    cases.push({
      name: '(а) снят oncontextmenu со строки',
      s: mutate(src, '  oncontextmenu={handleContextMenu}\n', ''),
      rule: 'A',
    });
    cases.push({
      name: '(б) oncontextmenu перенесён со строки на ячейку шеврона',
      s: mutate(
        mutate(src, '  oncontextmenu={handleContextMenu}\n', ''),
        '<span class="chevron-slot">',
        '<span class="chevron-slot" oncontextmenu={handleContextMenu}>',
      ),
      rule: 'A',
    });
    cases.push({
      name: '(в) удалён preventDefault',
      s: mutate(src, HANDLER_BODY, HANDLER_BODY.replace('    e.preventDefault();\n', '')),
      rule: 'B',
    });
    cases.push({
      // Счётный гейт («preventDefault есть в теле») здесь зелёный.
      name: '(г) preventDefault загнан внутрь охраны (подавляет не там)',
      s: mutate(
        src,
        HANDLER_BODY,
        '    const menu = activeMenu();\n' +
          '    if (!menu) {\n      e.preventDefault();\n      return;\n    }\n' +
          '    e.stopPropagation();\n' +
          '    menu.openAt(e.clientX, e.clientY);\n',
      ),
      rule: 'B',
    });
    cases.push({
      // Перестановка: preventDefault до охраны — родное меню глушится и без прав (D-06).
      name: '(д) перестановка: preventDefault до селектора и охраны',
      s: mutate(
        src,
        HANDLER_BODY,
        '    e.preventDefault();\n' + HANDLER_BODY.replace('    e.preventDefault();\n', ''),
      ),
      rule: 'B',
    });
    cases.push({
      name: '(е) перестановка: preventDefault после openAt',
      s: mutate(
        src,
        HANDLER_BODY,
        HANDLER_BODY.replace('    e.preventDefault();\n', '').replace(
          '    menu.openAt(e.clientX, e.clientY);\n',
          '    menu.openAt(e.clientX, e.clientY);\n    e.preventDefault();\n',
        ),
      ),
      rule: 'B',
    });
    cases.push({
      // Охрана цела, preventDefault есть в теле, порядок по индексам верный —
      // но он под условием: родное меню всплывает при обычном правом клике.
      name: '(е2) preventDefault под условием (не на верхнем уровне тела)',
      s: mutate(
        src,
        HANDLER_BODY,
        HANDLER_BODY.replace(
          '    e.preventDefault();\n',
          '    if (e.shiftKey) {\n      e.preventDefault();\n    }\n',
        ),
      ),
      rule: 'B',
    });
    cases.push({
      name: '(ж) обработчик открывает меню группы напрямую, минуя селектор',
      s: mutate(src, HANDLER_BODY, HANDLER_BODY.replace('activeMenu()', 'groupMenu')),
      rule: 'C',
    });
    cases.push({
      name: '(з) селектор отдаёт меню группы узлу-типу',
      s: mutate(src, 'if (showTypeMenu) return typeMenu;', 'if (showTypeMenu) return groupMenu;'),
      rule: 'C',
    });
    cases.push({
      name: '(и) showTypeMenu выводится из kind группы',
      s: mutate(
        src,
        "item.kind === 'type' && canEditTypes",
        "item.kind === 'group' && canEditTypes",
      ),
      rule: 'D',
    });
    cases.push({
      name: '(к) bind:this меню типа привязан к groupMenu',
      s: mutate(src, 'bind:this={typeMenu}', 'bind:this={groupMenu}'),
      rule: 'E',
    });
    cases.push({
      name: '(л) bind:this обоих меню переставлены местами',
      s: mutate(
        mutate(src, 'bind:this={typeMenu}', 'bind:this={__T__}'),
        'bind:this={groupMenu}',
        'bind:this={typeMenu}',
      ).replace('bind:this={__T__}', 'bind:this={groupMenu}'),
      rule: 'E',
    });
    cases.push({
      // Комментарий не должен «прятать» снятый атрибут: гейт читает текст без комментариев.
      name: '(м) oncontextmenu оставлен только в HTML-комментарии',
      s: mutate(
        mutate(src, '  oncontextmenu={handleContextMenu}\n', ''),
        '<div\n  id={rowDomId(item.key)}',
        '<!-- oncontextmenu={handleContextMenu} -->\n<div\n  id={rowDomId(item.key)}',
      ),
      rule: 'A',
    });
  } catch (err) {
    console.error(`${TAG} [selftest] FAIL — ${err.message}`);
    return false;
  }

  let failed = 0;
  for (const c of cases) {
    const v = [];
    scanNode(c.s, v);
    let ok;
    if (c.rule === null) ok = v.length === 0;
    else ok = v.length > 0 && v.every((line) => line.startsWith(`[${c.rule}]`));
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${c.name} ` +
        `(ожидалось ${c.rule === null ? '0 нарушений' : `только [${c.rule}]`}, получено ${v.length})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }
  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} из ${cases.length} фикстур не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — все ${cases.length} фикстур совпали с ожиданием.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-group-tree-context-menu.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) process.exit(runSelfTest() ? 0 : 1);

  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const abs = path.join(root, NODE_FILE);
  const violations = [];
  if (!fs.existsSync(abs)) violations.push(`${NODE_FILE}: файл не найден (${abs}).`);
  else scanNode(fs.readFileSync(abs, 'utf8'), violations);

  if (violations.length > 0) {
    for (const line of violations) console.error(`${TAG} ${line}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
