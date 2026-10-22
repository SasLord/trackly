#!/usr/bin/env node
// [check-places-expand-groups] Структурный гейт режима «Разворачивать группы» и
// строки группы в разделе «Места» (PlacesPage.svelte + PlaceContents.svelte).
//
// Почему он существует: PlaceContents ремонтируется по {#key place.id:token} на
// каждом выборе узла, поэтому режим, объявленный локальным $state внутри него,
// молча сбрасывался бы при переходе между местами. Режим обязан принадлежать
// PlacesPage (по образцу onlyHere), храниться в localStorage и никогда не
// сбрасываться. Компиляционные гейты (svelte-check, eslint, build) слепы ко всему
// этому: локальный $state и контролируемый проп типизируются одинаково, а рантайм-
// ошибки рун они не видят вовсе.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он закрепляет владение режимом, порядок чекбоксов,
// литерал `expandGroups: true` у счётчика массового переноса и инертность строки
// группы. Он НЕ доказывает, что чекбокс переживает ремонт в живом приложении и что
// нет effect_update_depth_exceeded — это проверяется живой проверкой фазы.
//
// Правила (над текстом файлов без комментариев):
//   (A) PlacesPage владеет режимом: ключ 'trackly:places:expandGroups' объявлен
//       ровно один раз; чтение сравнивает с '1'; `let expandGroups = $state(`
//       инициализируется вызовом чтения; в <PlaceContents> переданы
//       {expandGroups} и onExpandGroupsChange; тело handleShowBlockedContents не
//       трогает expandGroups; запись в localStorage не стоит внутри $effect.
//   (B) PlaceContents: expandGroups и onExpandGroupsChange — поля Props; локального
//       объявления режима нет; чекбокс place-contents-expand-groups стоит в блоке
//       controls-right строго после place-contents-only-here и до кнопки
//       «Перенести всё содержимое в…»; подпись «Разворачивать группы» на месте.
//   (C) Запросы places_contents: ровно два; тот, что с `nested: true`
//       (счётчик массового переноса), шлёт литерал `expandGroups: true`; основной
//       шлёт `expandGroups` без литерала.
//   (D) Ветка `row.kind === 'group'` обязана существовать (нижний порог
//       антивакуума); в ней нет openView( и role="button". Шеврон (<RowChevron … />)
//       из подсчёта вырезается: ПОСЛЕ этого остаётся ровно один onclick (кнопка-имя),
//       так что тело строки инертно, а шеврон — единственное исключение. openGroup(
//       вызван в ветке (кнопка-имя), двойной клик по строке ведёт в openGroup(
//       (прямо или через обработчик, чьё тело зовёт openGroup( и не зовёт openView();
//       тело openGroup содержит `#/groups?id=` и не зовёт openView.
//   (E) Шеврон: в файле ровно один <RowChevron (общий компонент; собственной
//       кнопки tr-row-chevron нет), он стоит в ветке группы, его onclick зовёт
//       toggleGroup(, а toggleGroup меняет expandedGroupIds.
//   (F) Загрузка состава: apiCall('places_group_members') ровно один раз и внутри
//       loadMembers, не внутри $effect; какой-то $effect зовёт loadMembers( (эффект
//       не мёртв) и не пишет состояние состава; внутри loadMembers запись
//       membersByGroup/loadedKeyByGroup стоит ПОСЛЕ `.then(` (после await).
//   (G) handleExpandGroupsChange при истинном значении обнуляет expandedGroupIds и
//       зовёт onExpandGroupsChange(; чекбокс place-contents-expand-groups подключён
//       именно к нему.
//   (H) Второго уровня раскрытия нет: строки состава и общий сниппет rowCells не
//       содержат шеврона; сниппет rowCells объявлен и используется не меньше двух
//       раз (обычные строки и строки состава), а ветка группы рисует состав через него.
// Файл не читается или не разобрать — нарушение с понятным сообщением, не SKIP.
//
// Self-test (`--selftest`): чистые РЕАЛЬНЫЕ исходники проходят; затем мутанты по
// реальным исходникам через mutate() с проверкой ровно одного вхождения якоря
// (иначе мутант мог бы снять гейт у соседнего фрагмента). Порядок защищён мутантом-
// ПЕРЕСТАНОВКОЙ (swap) блоков чекбоксов, а не удалением.
//
// Zero-dependency: node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-places-expand-groups.mjs              # проверить репозиторий
//   node scripts/check-places-expand-groups.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-places-expand-groups.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-places-expand-groups]';

const PAGE_FILE = 'src/features/places/PlacesPage.svelte';
const CONTENTS_FILE = 'src/features/places/PlaceContents.svelte';
const STORAGE_KEY = 'trackly:places:expandGroups';

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

/** Тело `function name(...) { ... }` или null. */
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

/**
 * Содержимое ветки `{#if <cond>} … ` до `{:else` или `{/if}` на ТОМ ЖЕ уровне
 * вложенности (вложенные {#if}/{:else} внутри ветки её не обрывают).
 */
function ifBranch(code, openMarker) {
  const start = code.indexOf(openMarker);
  if (start === -1) return null;
  const from = start + openMarker.length;
  const tokens = /\{#if\b|\{\/if\}|\{:else/g;
  tokens.lastIndex = from;
  let depth = 0;
  let m;
  while ((m = tokens.exec(code))) {
    if (m[0] === '{#if') depth++;
    else if (m[0] === '{/if}') {
      if (depth === 0) return code.slice(from, m.index);
      depth--;
    } else if (depth === 0) return code.slice(from, m.index);
  }
  return null;
}

function count(text, re) {
  return [...text.matchAll(re)].length;
}

function scanPage(raw, violations) {
  const code = stripComments(raw);
  const F = PAGE_FILE;

  // (A1) ключ объявлен ровно один раз (на хранилище не должно быть второго владельца).
  const keyCount = code.split(`'${STORAGE_KEY}'`).length - 1;
  if (keyCount !== 1) {
    violations.push(
      `(A) ${F}: ключ '${STORAGE_KEY}' встречается ${keyCount} раз, нужно ровно одно объявление.`,
    );
  }

  // (A2) чтение сравнивает с '1'.
  const read = functionBody(code, 'readPersistedExpandGroups');
  if (read === null) {
    violations.push(
      `(A) ${F}: не найдена функция readPersistedExpandGroups — гейт не может проверить чтение режима.`,
    );
  } else if (!/===\s*'1'/.test(read)) {
    violations.push(
      `(A) ${F}: readPersistedExpandGroups не сравнивает значение с '1' — иное значение в хранилище не должно включать режим (T-41.2-33).`,
    );
  }

  // (A3) состояние инициализируется чтением.
  if (!/\blet\s+expandGroups\s*=\s*\$state\(\s*readPersistedExpandGroups\(\)\s*\)/.test(code)) {
    violations.push(
      `(A) ${F}: нет \`let expandGroups = $state(readPersistedExpandGroups())\` — владелец режима обязан читать его из хранилища.`,
    );
  }

  // (A4) проброс в <PlaceContents>.
  const tagStart = code.indexOf('<PlaceContents');
  const tagEnd = tagStart === -1 ? -1 : code.indexOf('/>', tagStart);
  if (tagStart === -1 || tagEnd === -1) {
    violations.push(
      `(A) ${F}: не найден тег <PlaceContents … /> — гейт не может проверить проброс.`,
    );
  } else {
    const tag = code.slice(tagStart, tagEnd);
    if (!/\{expandGroups\}/.test(tag)) {
      violations.push(`(A) ${F}: в <PlaceContents> не передан {expandGroups}.`);
    }
    if (!/\bonExpandGroupsChange\s*=/.test(tag)) {
      violations.push(`(A) ${F}: в <PlaceContents> не передан onExpandGroupsChange.`);
    }
  }

  // (A5) режим не сбрасывается в handleShowBlockedContents.
  const blocked = functionBody(code, 'handleShowBlockedContents');
  if (blocked === null) {
    violations.push(
      `(A) ${F}: не найдена функция handleShowBlockedContents — гейт не может проверить, что режим не сбрасывается.`,
    );
  } else if (/expandGroups/i.test(blocked)) {
    violations.push(
      `(A) ${F}: handleShowBlockedContents трогает режим разворота. «Показать содержимое» ничего не говорит о режиме — он не сбрасывается никогда (UI-SPEC §11.1).`,
    );
  }

  // (A6) запись в хранилище — в обработчике, не в $effect.
  for (const m of code.matchAll(/\$effect(?:\.pre)?\s*\(/g)) {
    const parenOpen = code.indexOf('(', m.index);
    const body = extractBalanced(code, parenOpen, '(', ')') ?? '';
    if (/persistExpandGroups|localStorage/.test(body)) {
      violations.push(
        `(A) ${F}: запись режима в localStorage внутри $effect — риск effect_update_depth_exceeded. Писать надо в обработчике onExpandGroupsChange.`,
      );
    }
  }
}

function scanContents(raw, violations) {
  const code = stripComments(raw);
  const F = CONTENTS_FILE;

  // (B1) поля Props.
  const ifaceAt = code.search(/\binterface\s+Props\s*\{/);
  const iface = ifaceAt === -1 ? null : extractBalanced(code, code.indexOf('{', ifaceAt), '{', '}');
  if (iface === null) {
    violations.push(
      `(B) ${F}: не найден interface Props — гейт не может проверить контролируемую пару.`,
    );
  } else {
    if (!/\bexpandGroups\s*:/.test(iface)) violations.push(`(B) ${F}: expandGroups не поле Props.`);
    if (!/\bonExpandGroupsChange\s*:/.test(iface)) {
      violations.push(`(B) ${F}: onExpandGroupsChange не поле Props.`);
    }
  }

  // (B2) локального состояния режима нет.
  if (/\b(?:let|const|var)\s+expandGroups\s*=/.test(code)) {
    violations.push(
      `(B) ${F}: найдено локальное объявление expandGroups. Режим принадлежит PlacesPage: компонент ремонтируется по {#key} и локальное состояние сбрасывалось бы при выборе узла.`,
    );
  }

  // (B3) порядок: controls-right < только здесь < разворачивать < кнопка переноса.
  const iRight = code.indexOf('class="controls-right"');
  const ids = [
    ['place-contents-only-here', code.indexOf('id="place-contents-only-here"')],
    ['place-contents-expand-groups', code.indexOf('id="place-contents-expand-groups"')],
  ];
  const iMove = code.indexOf('Перенести всё содержимое в…');
  for (const [id, pos] of ids) {
    if (pos === -1) violations.push(`(B) ${F}: не найден чекбокс id="${id}".`);
    if (count(code, new RegExp(`id="${id}"`, 'g')) > 1) {
      violations.push(`(B) ${F}: id="${id}" встречается больше одного раза.`);
    }
  }
  if (iRight === -1) violations.push(`(B) ${F}: не найден блок controls-right.`);
  if (iMove === -1) violations.push(`(B) ${F}: не найдена кнопка «Перенести всё содержимое в…».`);
  const [, iOnly] = ids[0];
  const [, iExpand] = ids[1];
  if (iRight !== -1 && iOnly !== -1 && iExpand !== -1 && iMove !== -1) {
    if (!(iRight < iOnly && iOnly < iExpand && iExpand < iMove)) {
      violations.push(
        `(B) ${F}: порядок в controls-right нарушен. Нужно: «Только здесь», затем «Разворачивать группы», затем «Перенести всё содержимое в…» (UI-SPEC §11.1).`,
      );
    }
  }
  if (!code.includes('Разворачивать группы')) {
    violations.push(`(B) ${F}: нет подписи «Разворачивать группы».`);
  }

  // (C) запросы places_contents.
  const calls = [];
  for (const m of code.matchAll(/'places_contents'\s*,\s*\{/g)) {
    const braceOpen = code.indexOf('{', m.index);
    calls.push(extractBalanced(code, braceOpen, '{', '}') ?? '');
  }
  if (calls.length !== 2) {
    violations.push(
      `(C) ${F}: запросов places_contents ${calls.length}, ожидалось ровно два (основной и счётчик массового переноса).`,
    );
  }
  let bulk = 0;
  let main = 0;
  for (const body of calls) {
    if (/\bnested\s*:\s*true\b/.test(body)) {
      bulk++;
      if (!/\bexpandGroups\s*:\s*true\b/.test(body)) {
        violations.push(
          `(C) ${F}: запрос счётчика массового переноса (nested: true) не шлёт литерал \`expandGroups: true\` — подтверждение должно называть физическое число.`,
        );
      }
    } else {
      main++;
      if (!/(?:^|[\s,{])expandGroups\s*(?:,|$)/.test(body) || /\bexpandGroups\s*:/.test(body)) {
        violations.push(
          `(C) ${F}: основной запрос places_contents должен передавать режим как \`expandGroups\` (проп), без литерала.`,
        );
      }
    }
  }
  if (calls.length === 2 && (bulk !== 1 || main !== 1)) {
    violations.push(
      `(C) ${F}: ожидался один запрос с \`nested: true\` (счётчик переноса) и один основной; найдено ${bulk} и ${main}.`,
    );
  }

  // (D) строка группы.
  const branch = ifBranch(code, "{#if row.kind === 'group'}");
  if (branch === null) {
    violations.push(
      `(D) ${F}: не найдена ветка \`{#if row.kind === 'group'}\` — антивакуум: без неё гейт ничего не проверяет о строке группы.`,
    );
  } else {
    if (/\bopenView\s*\(/.test(branch)) {
      violations.push(
        `(D) ${F}: ветка строки группы вызывает openView( — «Просмотр» для группы не открывается никогда (D-19).`,
      );
    }
    if (/role\s*=\s*"button"/.test(branch)) {
      violations.push(
        `(D) ${F}: ветка строки группы содержит role="button" — тело строки группы инертно (R-03).`,
      );
    }
    // Шеврон — единственное исключение из «одиночный клик по телу инертен»: его
    // тег вырезается, всё остальное обязано дать ровно один onclick (кнопка-имя).
    const clicks = count(stripChevronTags(branch), /\bonclick\s*=/g);
    if (clicks !== 1) {
      violations.push(
        `(D) ${F}: в ветке строки группы вне шеврона ${clicks} onclick=, нужен ровно один (кнопка-имя); одиночный клик по телу строки инертен (R-03).`,
      );
    }
    const opens = count(branch, /\bopenGroup\s*\(/g);
    if (opens < 1) {
      violations.push(
        `(D) ${F}: openGroup( не вызван в ветке строки группы — у кнопки-имени нет клавиатурного пути в «Группы».`,
      );
    }
    const dbl = /\bondblclick\s*=\s*\{/.exec(branch);
    if (dbl === null) {
      violations.push(
        `(D) ${F}: у строки группы нет ondblclick — двойной клик обязан вести в «Группы» (D-19).`,
      );
    } else {
      const value = extractBalanced(branch, branch.indexOf('{', dbl.index), '{', '}') ?? '';
      let ok = /\bopenGroup\s*\(/.test(value);
      if (!ok) {
        const handler = /\b([A-Za-z_]\w*)\s*\(/.exec(value);
        const body = handler ? functionBody(code, handler[1]) : null;
        ok = body !== null && /\bopenGroup\s*\(/.test(body) && !/\bopenView\s*\(/.test(body);
      }
      if (!ok) {
        violations.push(
          `(D) ${F}: ondblclick строки группы не ведёт в openGroup( (прямо или через обработчик без openView().`,
        );
      }
    }
  }
  const openGroup = functionBody(code, 'openGroup');
  if (openGroup === null) {
    violations.push(`(D) ${F}: не найдена функция openGroup.`);
  } else {
    if (!openGroup.includes('#/groups?id=')) {
      violations.push(`(D) ${F}: openGroup не ведёт в \`#/groups?id=…\`.`);
    }
    if (/\bopenView\s*\(/.test(openGroup)) {
      violations.push(`(D) ${F}: openGroup вызывает openView(.`);
    }
  }
}

/** Текст без тегов `<RowChevron … />` (граница — первый `/>`; `=>` его не содержит). */
function stripChevronTags(text) {
  let out = '';
  let i = 0;
  for (;;) {
    const at = text.indexOf('<RowChevron', i);
    if (at === -1) return out + text.slice(i);
    out += text.slice(i, at);
    const end = text.indexOf('/>', at);
    if (end === -1) return out;
    i = end + 2;
  }
}

/** Содержимое тега `<RowChevron … />` (все вхождения). */
function chevronTags(text) {
  const tags = [];
  let i = 0;
  for (;;) {
    const at = text.indexOf('<RowChevron', i);
    if (at === -1) return tags;
    const end = text.indexOf('/>', at);
    if (end === -1) return tags;
    tags.push(text.slice(at, end));
    i = end + 2;
  }
}

/** Тело сниппета `{#snippet name(...)} … {/snippet}` или null. */
function snippetBody(code, name) {
  const at = code.search(new RegExp(`\\{#snippet\\s+${name}\\s*\\(`));
  if (at === -1) return null;
  const from = code.indexOf('}', at) + 1;
  const end = code.indexOf('{/snippet}', from);
  return end === -1 ? null : code.slice(from, end);
}

const STATE_WRITE =
  /\b(?:membersByGroup|loadedKeyByGroup|expandedGroupIds|rowsGen)\s*(?:=(?!=)|\+=)/;

function scanExpansion(raw, violations) {
  const code = stripComments(raw);
  const F = CONTENTS_FILE;
  const branch = ifBranch(code, "{#if row.kind === 'group'}") ?? '';

  // (E) шеврон.
  const allTags = chevronTags(code);
  if (allTags.length !== 1) {
    violations.push(
      `(E) ${F}: <RowChevron найден ${allTags.length} раз, нужен ровно один (в строке группы; второго уровня раскрытия нет).`,
    );
  }
  const inBranch = chevronTags(branch);
  if (inBranch.length !== 1) {
    violations.push(
      `(E) ${F}: в ветке строки группы ${inBranch.length} шевронов, нужен ровно один.`,
    );
  } else if (!/\bonclick\s*=\s*\{[^}]*\btoggleGroup\s*\(/.test(inBranch[0])) {
    violations.push(`(E) ${F}: onclick шеврона не вызывает toggleGroup(.`);
  }
  if (/tr-row-chevron/.test(code)) {
    violations.push(
      `(E) ${F}: собственная реализация шеврона (tr-row-chevron) — нужен общий RowChevron.`,
    );
  }
  const toggle = functionBody(code, 'toggleGroup');
  if (toggle === null || !/\bexpandedGroupIds\s*=/.test(toggle)) {
    violations.push(`(E) ${F}: toggleGroup не меняет expandedGroupIds (или функции нет).`);
  }

  // (F) загрузка состава.
  const callRe = /apiCall\s*(?:<[^>]*>)?\s*\(\s*'places_group_members'/g;
  const callCount = count(code, callRe);
  if (callCount !== 1) {
    violations.push(`(F) ${F}: вызовов places_group_members ${callCount}, нужен ровно один.`);
  }
  const load = functionBody(code, 'loadMembers');
  if (load === null) {
    violations.push(`(F) ${F}: не найдена функция loadMembers.`);
  } else {
    if (count(load, callRe) !== 1) {
      violations.push(`(F) ${F}: apiCall('places_group_members') стоит не внутри loadMembers.`);
    }
    const thenAt = load.indexOf('.then(');
    const firstWrite = load.search(/\b(?:membersByGroup|loadedKeyByGroup)\s*=(?!=)/);
    if (thenAt === -1 || firstWrite === -1 || firstWrite < thenAt) {
      violations.push(
        `(F) ${F}: в loadMembers запись состава стоит не после \`.then(\` (после await) — риск effect_update_depth_exceeded.`,
      );
    }
  }
  let effectCallsLoader = false;
  for (const m of code.matchAll(/\$effect(?:\.pre)?\s*\(/g)) {
    const body = extractBalanced(code, code.indexOf('(', m.index), '(', ')') ?? '';
    if (count(body, callRe) > 0) {
      violations.push(
        `(F) ${F}: apiCall('places_group_members') внутри $effect — запрос стоит только в loadMembers.`,
      );
    }
    if (/\bloadMembers\s*\(/.test(body)) {
      effectCallsLoader = true;
      if (STATE_WRITE.test(body)) {
        violations.push(
          `(F) ${F}: $effect, зовущий loadMembers, пишет состояние состава — писать можно только после await.`,
        );
      }
    }
  }
  if (!effectCallsLoader) {
    violations.push(
      `(F) ${F}: ни один $effect не вызывает loadMembers( — раскрытие не загрузит состав.`,
    );
  }

  // (G) смена режима.
  const change = functionBody(code, 'handleExpandGroupsChange');
  if (change === null) {
    violations.push(`(G) ${F}: не найдена функция handleExpandGroupsChange.`);
  } else {
    if (!/\bif\s*\(\s*v\s*\)\s*expandedGroupIds\s*=\s*new Set\(\s*\)/.test(change)) {
      violations.push(
        `(G) ${F}: handleExpandGroupsChange не обнуляет expandedGroupIds при включении режима.`,
      );
    }
    if (!/\bonExpandGroupsChange\s*\(\s*v\s*\)/.test(change)) {
      violations.push(`(G) ${F}: handleExpandGroupsChange не вызывает onExpandGroupsChange(v).`);
    }
  }
  const boxAt = code.indexOf('id="place-contents-expand-groups"');
  const boxStart = boxAt === -1 ? -1 : code.lastIndexOf('<Checkbox', boxAt);
  const boxEnd = boxAt === -1 ? -1 : code.indexOf('>', boxAt);
  const box = boxStart === -1 || boxEnd === -1 ? '' : code.slice(boxStart, boxEnd);
  if (!/\bonchange\s*=\s*\{handleExpandGroupsChange\}/.test(box)) {
    violations.push(
      `(G) ${F}: чекбокс place-contents-expand-groups подключён не к handleExpandGroupsChange.`,
    );
  }

  // (H) один уровень.
  const cells = snippetBody(code, 'rowCells');
  if (cells === null) {
    violations.push(`(H) ${F}: не объявлен сниппет rowCells.`);
  } else if (/<RowChevron/.test(cells)) {
    violations.push(
      `(H) ${F}: сниппет rowCells содержит шеврон — строки состава раскрываться не должны.`,
    );
  }
  if (count(code, /\{@render\s+rowCells\s*\(/g) < 2) {
    violations.push(
      `(H) ${F}: rowCells используется меньше двух раз (обычные строки и строки состава).`,
    );
  }
  if (!/\{@render\s+rowCells\s*\(\s*member\s*\)/.test(branch)) {
    violations.push(`(H) ${F}: состав группы в ветке группы рисуется не через rowCells(member).`);
  }
}

function readFile(root, rel, violations) {
  const abs = path.join(root, rel);
  try {
    return fs.readFileSync(abs, 'utf8');
  } catch {
    violations.push(
      `(A) ${rel}: файл не читается (${abs}); отсутствие файла — нарушение, не SKIP.`,
    );
    return null;
  }
}

/** Чистая функция сканирования: принимает уже прочитанные исходники, fs не трогает. */
function scanSource({ page, contents }, violations) {
  if (page !== null) scanPage(page, violations);
  if (contents !== null) {
    scanContents(contents, violations);
    scanExpansion(contents, violations);
  }
}

function checkRepo(root, violations) {
  const page = readFile(root, PAGE_FILE, violations);
  const contents = readFile(root, CONTENTS_FILE, violations);
  scanSource({ page, contents }, violations);
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

/** Перестановка: блок «Только здесь» и блок «Разворачивать группы» меняются местами. */
function swapCheckboxBlocks(source) {
  const re = (id) => new RegExp(`<Checkbox\\b[^>]*?id="${id}"[^>]*>[\\s\\S]*?</Checkbox>`, 'g');
  const a = source.match(re('place-contents-only-here')) ?? [];
  const b = source.match(re('place-contents-expand-groups')) ?? [];
  if (a.length !== 1 || b.length !== 1) {
    throw new Error(`мутант swap: блоков чекбоксов ${a.length} и ${b.length}, нужно по одному`);
  }
  const joined = source.split(a[0] + '\n');
  if (joined.length !== 2 || !joined[1].trimStart().startsWith(b[0])) {
    throw new Error('мутант swap: блок «Разворачивать группы» не стоит сразу за «Только здесь»');
  }
  const indent = joined[1].slice(0, joined[1].length - joined[1].trimStart().length);
  return (
    joined[0] +
    b[0] +
    '\n' +
    indent +
    a[0] +
    '\n' +
    joined[1].slice(indent.length + b[0].length + 1)
  );
}

function runSelfTest() {
  let page;
  let contents;
  try {
    page = fs.readFileSync(path.join(UI_ROOT, PAGE_FILE), 'utf8');
    contents = fs.readFileSync(path.join(UI_ROOT, CONTENTS_FILE), 'utf8');
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник не читается: ${e.message}`);
    return false;
  }
  const clean = [];
  scanSource({ page, contents }, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники обязаны проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  let mutants;
  try {
    mutants = [
      {
        name: "(1) чтение режима: `=== '1'` заменено на `!== '0'`",
        src: {
          page: mutate(
            page,
            "localStorage.getItem(EXPAND_GROUPS_STORAGE_KEY) === '1'",
            "localStorage.getItem(EXPAND_GROUPS_STORAGE_KEY) !== '0'",
            'read-not-one',
          ),
          contents,
        },
        rules: ['A'],
      },
      {
        name: '(2) handleShowBlockedContents сбрасывает режим',
        src: {
          page: mutate(
            page,
            '    contentsResetToken += 1;\n',
            '    contentsResetToken += 1;\n    expandGroups = false;\n',
            'reset-in-blocked',
          ),
          contents,
        },
        rules: ['A'],
      },
      {
        name: '(3) в PlaceContents вернулось локальное `let expandGroups = $state(false)`',
        src: {
          page,
          contents: mutate(
            contents,
            '  // Phase 41.2 (D-19): a group row has no',
            '  let expandGroups = $state(false);\n  // Phase 41.2 (D-19): a group row has no',
            'local-state',
          ),
        },
        rules: ['B'],
      },
      {
        name: '(4) ПЕРЕСТАНОВКА (swap): «Разворачивать группы» стоит перед «Только здесь»',
        src: { page, contents: swapCheckboxBlocks(contents) },
        rules: ['B'],
      },
      {
        name: '(5) счётчик массового переноса шлёт `expandGroups`, а не литерал true',
        src: {
          page,
          contents: mutate(
            contents,
            '      expandGroups: true,\n',
            '      expandGroups,\n',
            'bulk-literal',
          ),
        },
        rules: ['C'],
      },
      {
        name: '(5б) основной запрос шлёт литерал `expandGroups: true`',
        src: {
          page,
          contents: mutate(
            contents,
            '{ rootId, nested, expandGroups }',
            '{ rootId, nested, expandGroups: true }',
            'main-literal',
          ),
        },
        rules: ['C'],
      },
      {
        name: '(6) в ячейку строки группы добавлен onclick={() => openView(row)}',
        src: {
          page,
          contents: mutate(
            contents,
            '<td class="cell cell-static"><span class="tr-mono">—</span></td>',
            '<td class="cell cell-static" onclick={() => openView(row)}><span class="tr-mono">—</span></td>',
            'group-cell-openview',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(6б) openView( спрятан в {:else} ВНУТРИ ветки группы (глубина, а не первое {:else})',
        src: {
          page,
          contents: mutate(
            contents,
            '<td class="cell cell-static">—</td>',
            '{#if onlyHere}<td class="cell cell-static">—</td>{:else}<td class="cell cell-static" onclick={() => openView(row)}>—</td>{/if}',
            'nested-else-openview',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(6в) ячейка строки группы получила role="button"',
        src: {
          page,
          contents: mutate(
            contents,
            '<td class="cell cell-static" title={row.name}>',
            '<td class="cell cell-static" role="button" title={row.name}>',
            'group-role-button',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(6г) двойной клик по строке группы снят',
        src: {
          page,
          contents: mutate(
            contents,
            '            ondblclick={(e) => handleGroupRowDblClick(e, row)}\n',
            '',
            'no-dblclick',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(6д) ветка группы переименована (антивакуум: гейт не молчит)',
        src: {
          page,
          contents: mutate(
            contents,
            "{#if row.kind === 'group'}",
            "{#if row.kind === 'groupx'}",
            'no-branch',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(7) запись в localStorage перенесена в $effect',
        src: {
          page: mutate(
            page,
            '  function handleShowBlockedContents(): void {',
            '  $effect(() => {\n    persistExpandGroups(expandGroups);\n  });\n\n  function handleShowBlockedContents(): void {',
            'persist-in-effect',
          ),
          contents,
        },
        rules: ['A'],
      },
      {
        name: '(8) PlacesPage не передаёт {expandGroups} в PlaceContents',
        src: { page: mutate(page, '              {expandGroups}\n', '', 'no-prop'), contents },
        rules: ['A'],
      },
      {
        name: '(9) шеврон снят с ветки группы (состав раскрыть нечем)',
        src: {
          page,
          contents: mutate(
            contents,
            `              <RowChevron
                expanded={expandedGroupIds.has(row.id)}
                onclick={() => toggleGroup(row.id)}
                label={expandedGroupIds.has(row.id) ? 'Свернуть состав' : 'Развернуть состав'}
              />
`,
            '',
            'no-chevron',
          ),
        },
        rules: ['E'],
      },
      {
        name: '(10) apiCall(places_group_members) перенесён из loadMembers в $effect',
        src: {
          page,
          contents: mutate(
            mutate(
              contents,
              'loadMembers(r.id, key);',
              "void apiCall<PlaceContentDto[]>('places_group_members', { groupId: r.id });",
              'call-into-effect',
            ),
            "apiCall<PlaceContentDto[]>('places_group_members', { groupId })",
            'Promise.resolve([] as PlaceContentDto[])',
            'call-out-of-loader',
          ),
        },
        rules: ['F'],
      },
      {
        name: '(10б) ПЕРЕСТАНОВКА: запись состава вынесена из .then() ПЕРЕД запросом (до await)',
        src: {
          page,
          contents: mutate(
            mutate(
              contents,
              '        membersByGroup = { ...membersByGroup, [groupId]: members };\n',
              '',
              'write-out-of-then',
            ),
            "    apiCall<PlaceContentDto[]>('places_group_members', { groupId })",
            "    membersByGroup = { ...membersByGroup, [groupId]: [] };\n    apiCall<PlaceContentDto[]>('places_group_members', { groupId })",
            'write-before-await',
          ),
        },
        rules: ['F'],
      },
      {
        name: '(10в) $effect пишет состояние состава в своём теле',
        src: {
          page,
          contents: mutate(
            contents,
            '    const generation = rowsGen;\n',
            '    const generation = rowsGen;\n    membersByGroup = {};\n',
            'effect-writes-state',
          ),
        },
        rules: ['F'],
      },
      {
        name: '(10г) эффект больше не зовёт loadMembers (мёртвый эффект)',
        src: {
          page,
          contents: mutate(contents, 'loadMembers(r.id, key);', 'void key;', 'dead-effect'),
        },
        rules: ['F'],
      },
      {
        name: '(11) из handleExpandGroupsChange убрано обнуление expandedGroupIds',
        src: {
          page,
          contents: mutate(contents, '    if (v) expandedGroupIds = new Set();\n', '', 'no-reset'),
        },
        rules: ['G'],
      },
      {
        name: '(12) чекбокс подключён напрямую к onExpandGroupsChange, минуя обработчик',
        src: {
          page,
          contents: mutate(
            contents,
            'onchange={handleExpandGroupsChange}',
            'onchange={onExpandGroupsChange}',
            'checkbox-bypass',
          ),
        },
        rules: ['G'],
      },
      {
        name: '(13) шеврон добавлен в общий сниппет rowCells (второй уровень раскрытия)',
        src: {
          page,
          contents: mutate(
            contents,
            '<span class="tr-mono">{row.inventory_or_code ?? \'—\'}</span>',
            '<RowChevron expanded={false} onclick={() => {}} /><span class="tr-mono">{row.inventory_or_code ?? \'—\'}</span>',
            'chevron-in-cells',
          ),
        },
        rules: ['E', 'H'],
      },
      {
        name: '(13б) состав группы рисуется не через rowCells(member)',
        src: {
          page,
          contents: mutate(
            contents,
            '{@render rowCells(member)}',
            '{@render rowCells(row)}',
            'members-not-via-snippet',
          ),
        },
        rules: ['H'],
      },
      {
        name: '(14) ослабленное правило D: тело строки группы получило второй onclick (toggleGroup, без openView)',
        src: {
          page,
          contents: mutate(
            contents,
            '<td class="cell cell-static" title={row.name}>',
            '<td class="cell cell-static" title={row.name} onclick={() => toggleGroup(row.id)}>',
            'body-click-toggle',
          ),
        },
        rules: ['D'],
      },
      {
        name: '(14б) onclick шеврона ведёт в «Просмотр» (openView) вместо toggleGroup',
        src: {
          page,
          contents: mutate(
            contents,
            '                onclick={() => toggleGroup(row.id)}\n',
            '                onclick={() => openView(row)}\n',
            'chevron-openview',
          ),
        },
        rules: ['D', 'E'],
      },
      {
        name: '(14в) ondblclick строки группы ведёт в «Просмотр»',
        src: {
          page,
          contents: mutate(
            contents,
            'ondblclick={(e) => handleGroupRowDblClick(e, row)}',
            'ondblclick={() => openView(row)}',
            'dblclick-openview',
          ),
        },
        rules: ['D'],
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

  // Зелёный контроль: слова из запретных правил внутри КОММЕНТАРИЯ не краснят.
  try {
    const commented = mutate(
      contents,
      '  // Phase 41.2 (D-19): a group row has no',
      '  // раньше был let expandGroups = $state(false) и openView(row) в группе\n  // Phase 41.2 (D-19): a group row has no',
      'comment-ok',
    );
    const v = [];
    scanSource({ page, contents: commented }, v);
    const ok = v.length === 0;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — запретные слова в комментарии не краснят (ожидалось 0, получено ${v.length})`,
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
    `${TAG} [selftest] PASS — реальные исходники чисты, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-places-expand-groups.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  checkRepo(root, violations);
  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
