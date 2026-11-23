#!/usr/bin/env node
// [check-act-group-picker] Структурный гейт пикера позиции акта с группами
// (ActFormItemsTable.svelte, фаза 41.2, план 10).
//
// Почему он существует: пикер получил три независимых источника подсказок, и
// главный инвариант — фильтр статуса «на складе» действует ТОЛЬКО на складской
// запрос. Стоит статусу просочиться в запрос групп или членов групп, как
// устройство работающего АРМ снова исчезнет из пикера (GAM-01), а компиляционные
// гейты (svelte-check, eslint, build) этого не увидят: объект фильтра типизирован
// одинаково с любым status_id. Так же слеп компилятор к порядку «лимит 100 до
// вставки» и к тому, что количество позиции из группы фиксировано 1.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он не доказывает, что пикер работает в живом
// приложении (потеря состояния строк при вставке, effect_update_depth_exceeded) —
// это проверяется живой проверкой фазы.
//
// Правила (над текстом файла без комментариев):
//   (A) в Dropdown передан getGroupSection; четыре константы секций на месте и
//       SECTION_WORKING стоит в SECTION_ORDER.
//   (B) тело fetchStorage/fetchGroupHits/fetchMembers: складской listGrouped
//       несёт `status_id: storageStatusId`; запрос членов devices.list (в теле
//       fetchMembers) несёт `group_members_only: true` и `status_id: null` и не
//       упоминает storageStatusId; groups.search не несёт status_id вовсе. Нижний
//       порог антивакуума: все запросы обязаны быть найдены.
//   (E) D-34 фазы 41.4: тело fetchWorking найдено; его devices.list несёт
//       `status_id: workingStatusId` и `group_members_only: false`, не упоминает
//       storageStatusId; workingStatusId получен через statusIdByCode('в_работе');
//       члены групп отсеиваются через groups.forDevices(.
//   (C) pickGroupHit вызывает groups.actExpansion(; проверка лимита
//       (MAX_ACT_ITEMS) стоит ВЫШЕ первого onChange( в теле функции; константа
//       MAX_ACT_ITEMS равна 100; строка единицы создаётся через groupRow с
//       `quantity: 1`.
//   (F) D-32: fetchMembers отфильтровывает «Списано» по `status_code !== STATUS_WRITTEN_OFF`.
//   (D) FormItemRow и маркер `complectation_at_time !== undefined` (в двух ячейках) на месте
//       (D-12: сохранённая семантика RETAINED-позиций не тронута).
//
// Self-test (`--selftest`): чистый РЕАЛЬНЫЙ файл проходит; затем мутанты по
// реальному исходнику через mutate() с проверкой ровно одного вхождения якоря
// (иначе мутант мог бы снять соседнее правило). Порядок проверки лимита защищён
// мутантом-ПЕРЕСТАНОВКОЙ (swap), а не удалением: удаление проходит и гейт,
// поставленный ПОСЛЕ того, что он охраняет.
//
// Zero-dependency: node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-act-group-picker.mjs              # проверить репозиторий
//   node scripts/check-act-group-picker.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-act-group-picker.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-group-picker]';
const FILE = 'src/features/acts/ActFormItemsTable.svelte';

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

/** Тело `[async] function name(...) { ... }` или null. */
function functionBody(code, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  // Тип результата может содержать собственные `{`: тело открывает та скобка,
  // после которой сразу перенос строки.
  const braceOpen = code.indexOf('{\n', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

/** Аргументы всех вызовов `callee(` в тексте (содержимое скобок). */
function callArgs(code, callee) {
  const out = [];
  let from = 0;
  for (;;) {
    const at = code.indexOf(callee, from);
    if (at === -1) break;
    const parenOpen = at + callee.length - 1;
    const args = extractBalanced(code, parenOpen, '(', ')');
    if (args !== null) out.push(args);
    from = at + callee.length;
  }
  return out;
}

const SERVER_FILE = '../crates/trackly-app/src/services/act_service.rs';

/** Серверный исходник (корень репозитория — всегда рядом с ui/, не из --src). */
function readServerSource() {
  try {
    return fs.readFileSync(path.join(UI_ROOT, SERVER_FILE), 'utf8');
  } catch {
    return null;
  }
}

/**
 * B-IN-05: клиентская константа MAX_ACT_ITEMS и серверный потолок `p.items.len() > N`
 * (create и update) обязаны совпадать. Правило счёта: клиент суммирует quantity других
 * строк (согласован с update, который считает единицы; create считает строки, поэтому
 * клиент строже create — так задумано).
 */
function scanServerCeiling(rawClient, server, violations) {
  if (server === null) {
    violations.push(
      `(C) ${SERVER_FILE}: файл не читается — потолок 100 позиций не сверен с клиентом (отсутствие файла — нарушение, не SKIP).`,
    );
    return;
  }
  const client = /\bconst\s+MAX_ACT_ITEMS\s*=\s*(\d+)\b/.exec(stripComments(rawClient));
  const ceilings = [...stripComments(server).matchAll(/\bp\.items\.len\(\)\s*>\s*(\d+)\b/g)].map(
    (m) => Number(m[1]),
  );
  if (ceilings.length < 2) {
    violations.push(
      `(C) ${SERVER_FILE}: найдено потолков \`p.items.len() > N\`: ${ceilings.length}, нужно не меньше двух (create и update) — антивакуум.`,
    );
  }
  if (client && ceilings.some((n) => n !== Number(client[1]))) {
    violations.push(
      `(C) ${FILE}: MAX_ACT_ITEMS=${client[1]} расходится с серверным потолком [${ceilings.join(', ')}] в act_service.rs (B-IN-05).`,
    );
  }
}

export function scanSource(raw, violations, server = readServerSource()) {
  scanServerCeiling(raw, server, violations);
  const code = stripComments(raw);

  // (A) секции в одном Dropdown.
  if (!/<Dropdown\b[\s\S]*?getGroupSection\s*=/.test(code)) {
    violations.push(
      `(A) ${FILE}: в <Dropdown> не передан getGroupSection — секции «Группы» / «Устройства со склада» / «Устройства в составе групп» не будут разделены (D-01).`,
    );
  }
  for (const lit of [
    "'Группы'",
    "'Устройства со склада'",
    "'Устройства в работе'",
    "'Устройства в составе групп'",
  ]) {
    if (!code.includes(lit)) {
      violations.push(`(A) ${FILE}: нет константы секции ${lit}.`);
    }
  }
  if (!/\bSECTION_ORDER\s*=\s*\[[^\]]*\bSECTION_WORKING\b[^\]]*\]/.test(code)) {
    violations.push(
      `(A) ${FILE}: SECTION_WORKING не входит в SECTION_ORDER — секция не покажется.`,
    );
  }

  // (B) источники подсказок и фильтр статуса.
  const grouped = callArgs(code, 'devices.listGrouped(');
  const membersFn = functionBody(code, 'fetchMembers');
  const workingFn = functionBody(code, 'fetchWorking');
  const listed = membersFn === null ? [] : callArgs(membersFn, 'devices.list(');
  const listedWorking = workingFn === null ? [] : callArgs(workingFn, 'devices.list(');
  const listedAll = callArgs(code, 'devices.list(');
  const searched = callArgs(code, 'groups.search(');
  if (
    grouped.length !== 1 ||
    listed.length !== 1 ||
    listedWorking.length !== 1 ||
    listedAll.length !== 3 ||
    searched.length !== 1
  ) {
    violations.push(
      `(B) ${FILE}: антивакуум — найдено запросов listGrouped=${grouped.length}, devices.list(fetchMembers)=${listed.length}, devices.list(fetchWorking)=${listedWorking.length}, devices.list(всего)=${listedAll.length}, groups.search=${searched.length}; нужно 1/1/1/3/1 (иначе гейт ничего не охраняет).`,
    );
  }
  for (const body of grouped) {
    if (!/\bstatus_id\s*:\s*storageStatusId\b/.test(body)) {
      violations.push(
        `(B) ${FILE}: складской запрос listGrouped не несёт \`status_id: storageStatusId\` — прежнее поведение складского поиска изменено (D-02).`,
      );
    }
  }
  for (const body of listed) {
    if (!/\bgroup_members_only\s*:\s*true\b/.test(body)) {
      violations.push(`(B) ${FILE}: запрос членов групп не несёт \`group_members_only: true\`.`);
    }
    if (!/\bstatus_id\s*:\s*null\b/.test(body)) {
      violations.push(
        `(B) ${FILE}: запрос членов групп не несёт \`status_id: null\` — устройство работающего АРМ станет недостижимо (D-02).`,
      );
    }
    if (/storageStatusId/.test(body)) {
      violations.push(
        `(B) ${FILE}: запрос членов групп упоминает storageStatusId — фильтр статуса «на складе» просочился в источник членов (D-02).`,
      );
    }
  }
  for (const body of searched) {
    if (/status_id|storageStatusId/.test(body)) {
      violations.push(
        `(B) ${FILE}: groups.search( несёт фильтр статуса — группа любого статуса обязана быть достижима (D-02).`,
      );
    }
  }

  // (E) источник «Устройства в работе» (D-34 фазы 41.4).
  for (const body of listedWorking) {
    if (!/\bstatus_id\s*:\s*workingStatusId\b/.test(body)) {
      violations.push(
        `(E) ${FILE}: запрос fetchWorking не несёт \`status_id: workingStatusId\` — устройства «В работе» не попадут в пикер (D-34).`,
      );
    }
    if (!/\bgroup_members_only\s*:\s*false\b/.test(body)) {
      violations.push(`(E) ${FILE}: запрос fetchWorking не несёт \`group_members_only: false\`.`);
    }
    if (/storageStatusId/.test(body)) {
      violations.push(
        `(E) ${FILE}: запрос fetchWorking упоминает storageStatusId — фильтр «на складе» просочился в источник «в работе» (D-34).`,
      );
    }
  }
  if (workingFn !== null) {
    if (!workingFn.includes('groups.forDevices(')) {
      violations.push(
        `(E) ${FILE}: fetchWorking не отсеивает членов групп через groups.forDevices( — член группы показался бы без предупреждения о выносе из состава (D-34).`,
      );
    }
    if (!/\bworkingStatusId\s*=\s*statusIdByCode\(\s*'в_работе'\s*\)/.test(workingFn)) {
      violations.push(
        `(E) ${FILE}: workingStatusId в fetchWorking не получен через statusIdByCode('в_работе') (D-09: id статуса по code).`,
      );
    }
  } else {
    violations.push(`(E) ${FILE}: не найдена функция fetchWorking — антивакуум (D-34).`);
  }

  // (F) D-32: источник участников групп не предлагает «Списано» отдельным выбором —
  // фильтр по КОДУ через STATUS_WRITTEN_OFF (литерал статуса запрещён словарным гейтом).
  if (membersFn !== null) {
    if (!/\bstatus_code\s*!==\s*STATUS_WRITTEN_OFF\b/.test(membersFn)) {
      violations.push(
        `(F) ${FILE}: fetchMembers не отфильтровывает устройства «Списано» (\`status_code !== STATUS_WRITTEN_OFF\`) — списанное предлагалось бы отдельным выбором (D-32).`,
      );
    }
  } else {
    violations.push(`(F) ${FILE}: не найдена функция fetchMembers — антивакуум (D-32).`);
  }
  if (
    !/\bimport\s*\{[^}]*\bSTATUS_WRITTEN_OFF\b[^}]*\}\s*from\s*'\.\.\/devices\/deviceRowMenu'/.test(
      code,
    )
  ) {
    violations.push(
      `(F) ${FILE}: STATUS_WRITTEN_OFF не импортирован из ../devices/deviceRowMenu (D-32).`,
    );
  }

  // (C) разворот группы.
  if (!/\bconst\s+MAX_ACT_ITEMS\s*=\s*100\b/.test(code)) {
    violations.push(`(C) ${FILE}: нет \`const MAX_ACT_ITEMS = 100\` (серверный потолок позиций).`);
  }
  const pick = functionBody(code, 'pickGroupHit');
  if (pick === null) {
    violations.push(`(C) ${FILE}: не найдена функция pickGroupHit — антивакуум разворота.`);
  } else {
    if (!pick.includes('groups.actExpansion(')) {
      violations.push(
        `(C) ${FILE}: pickGroupHit не зовёт groups.actExpansion( — клиент не должен зеркалить правило разворота (T-41.2-44).`,
      );
    }
    const limitAt = pick.indexOf('MAX_ACT_ITEMS');
    const changeAt = pick.indexOf('onChange(');
    if (limitAt === -1) {
      violations.push(`(C) ${FILE}: pickGroupHit не проверяет лимит MAX_ACT_ITEMS (R-11).`);
    } else if (changeAt === -1) {
      violations.push(`(C) ${FILE}: pickGroupHit не вызывает onChange( — вставка строк потеряна.`);
    } else if (limitAt > changeAt) {
      violations.push(
        `(C) ${FILE}: проверка лимита MAX_ACT_ITEMS стоит ПОСЛЕ первого onChange( — частичная вставка возможна (R-11).`,
      );
    }
    // B-WR-01: первая встреча MAX_ACT_ITEMS — это вычисление `free`, оно стоит выше
    // onChange( при любой судьбе отклоняющей ветки. Поэтому проверяем сам отказ:
    // условие `fresh.length > free`, блок которого завершается `return`, стоит ниже
    // вычисления `free` и выше первого onChange(.
    const freeAt = pick.search(/\bconst\s+free\s*=[^;]*\bMAX_ACT_ITEMS\b/);
    const guard = /\bif\s*\(\s*fresh\.length\s*>\s*free\s*\)\s*\{/.exec(pick);
    if (freeAt === -1) {
      violations.push(
        `(C) ${FILE}: pickGroupHit не вычисляет \`const free = ... MAX_ACT_ITEMS ...\` — нечем ограничить вставку (R-11).`,
      );
    }
    if (guard === null) {
      violations.push(
        `(C) ${FILE}: pickGroupHit не содержит отказа \`if (fresh.length > free) {\` — вставка свыше лимита позиций возможна (R-11).`,
      );
    } else {
      const braceOpen = guard.index + guard[0].length - 1;
      const block = extractBalanced(pick, braceOpen, '{', '}');
      if (block === null || !/\breturn\s*;/.test(block)) {
        violations.push(
          `(C) ${FILE}: блок отказа по лимиту в pickGroupHit не завершается \`return;\` — после тоста вставка продолжится (R-11).`,
        );
      }
      if (block === null || !block.includes('pushToast(')) {
        violations.push(
          `(C) ${FILE}: блок отказа по лимиту в pickGroupHit не сообщает пользователю (pushToast) — молчаливый отказ (R-11).`,
        );
      }
      if (freeAt !== -1 && guard.index < freeAt) {
        violations.push(
          `(C) ${FILE}: отказ по лимиту стоит ВЫШЕ вычисления free — сравнение идёт со старым значением (R-11).`,
        );
      }
      if (changeAt !== -1 && guard.index > changeAt) {
        violations.push(
          `(C) ${FILE}: отказ \`fresh.length > free\` стоит ПОСЛЕ первого onChange( — частичная вставка возможна (R-11).`,
        );
      }
    }
  }
  const row = functionBody(code, 'groupRow');
  if (row === null) {
    violations.push(`(C) ${FILE}: не найдена функция groupRow — антивакуум строки единицы.`);
  } else if (!/\bquantity\s*:\s*1\b/.test(row)) {
    violations.push(
      `(C) ${FILE}: groupRow не фиксирует \`quantity: 1\` — количество позиции из группы фиксировано (D-02).`,
    );
  }

  // (D) FormItemRow и маркер RETAINED-позиций.
  if (!/\bexport\s+interface\s+FormItemRow\b/.test(code)) {
    violations.push(`(D) ${FILE}: нет \`export interface FormItemRow\` (D-12).`);
  }
  // Маркер стоит в двух местах (ячейка устройства и ячейка количества): снятие
  // любого из них ломает различие retained/fresh строк.
  const markers = code.split('complectation_at_time !== undefined').length - 1;
  if (markers < 2) {
    violations.push(
      `(D) ${FILE}: маркер \`complectation_at_time !== undefined\` найден ${markers} раз, нужно не меньше двух (ячейки устройства и количества) — семантика RETAINED-позиций тронута (D-12).`,
    );
  }
}

function readSource(root, violations) {
  const abs = path.join(root, FILE);
  try {
    return fs.readFileSync(abs, 'utf8');
  } catch {
    violations.push(
      `(A) ${FILE}: файл не читается (${abs}); отсутствие файла — нарушение, не SKIP.`,
    );
    return null;
  }
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

/**
 * Перестановка (swap): блок проверки лимита переносится ЗА вставку строк, то есть
 * ниже первого onChange( в pickGroupHit. Удаление проверки здесь не годится — оно
 * не доказывает, что гейт видит ПОРЯДОК.
 */
function swapLimitAfterOnChange(source) {
  const start = source.indexOf('async function pickGroupHit(');
  if (start === -1) throw new Error('мутант swap: нет pickGroupHit');
  const limitStart = source.indexOf('    // Занято позициями других строк', start);
  const limitEnd = source.indexOf('    // Первая единица заменяет строку idx', start);
  if (limitStart === -1 || limitEnd === -1 || limitEnd < limitStart) {
    throw new Error('мутант swap: не найден блок проверки лимита в pickGroupHit');
  }
  const block = source.slice(limitStart, limitEnd);
  if (source.split(block).length - 1 !== 1) {
    throw new Error('мутант swap: блок лимита неуникален');
  }
  const without = source.slice(0, limitStart) + source.slice(limitEnd);
  const anchor = '    onChange(next);\n    anchorHintIds = hints;';
  if (without.split(anchor).length - 1 !== 1) {
    throw new Error('мутант swap: якорь onChange(next) неуникален');
  }
  return without.replace(anchor, () => `    onChange(next);\n${block}    anchorHintIds = hints;`);
}

/** Мутант (13): блок отказа по лимиту вырезан целиком (якорь начала — ровно один). */
function deleteLimitBlock(source) {
  const anchor = '    if (fresh.length > free) {\n';
  if (source.split(anchor).length - 1 !== 1) {
    throw new Error('мутант (13): якорь `if (fresh.length > free) {` должен быть уникален');
  }
  const start = source.indexOf(anchor);
  const braceOpen = start + anchor.length - 2;
  const body = extractBalanced(source, braceOpen, '{', '}');
  if (body === null) throw new Error('мутант (13): блок не закрыт');
  const end = braceOpen + body.length + 2; // включая `}`
  return source.slice(0, start) + source.slice(end).replace(/^\n/, '');
}

function runSelfTest() {
  let raw;
  try {
    raw = fs.readFileSync(path.join(UI_ROOT, FILE), 'utf8');
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник не читается: ${e.message}`);
    return false;
  }
  const clean = [];
  scanSource(raw, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник обязан проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  let mutants;
  try {
    mutants = [
      {
        name: '(1) из складского запроса убран `status_id: storageStatusId`',
        src: mutate(
          raw,
          '        status_id: storageStatusId,\n',
          '        status_id: null,\n',
          'storage-no-status',
        ),
        rule: 'B',
      },
      {
        name: '(2) в запрос членов групп подставлен `status_id: storageStatusId`',
        src: mutate(
          raw,
          '          status_id: null,\n          state: null,\n          name_prefix: query,\n          include_deleted: false,\n          group_by_condition: false,\n',
          '          status_id: storageStatusId,\n          state: null,\n          name_prefix: query,\n          include_deleted: false,\n          group_by_condition: false,\n',
          'members-with-status',
        ),
        rule: 'B',
      },
      {
        name: '(3) из Dropdown убран `getGroupSection`',
        src: mutate(raw, '              getGroupSection={sectionOf}\n', '', 'no-section'),
        rule: 'A',
      },
      {
        name: '(4) в groups.search добавлен status_id',
        src: mutate(
          raw,
          'await groups.search(query, null)',
          'await groups.search(query, null, { status_id: 1 })',
          'search-status',
        ),
        rule: 'B',
      },
      {
        name: '(5) ПЕРЕСТАНОВКА (swap): проверка лимита перенесена ниже первого onChange( в pickGroupHit',
        src: swapLimitAfterOnChange(raw),
        rule: 'C',
      },
      {
        name: '(6) в строке единицы `quantity: 1` заменено на `quantity: base.quantity`',
        src: mutate(
          raw,
          '      quantity: 1,\n      has_serial: true,',
          '      quantity: base.quantity,\n      has_serial: true,',
          'row-quantity',
        ),
        rule: 'C',
      },
      {
        name: '(7) pickGroupHit не зовёт groups.actExpansion( (клиент зеркалит разворот)',
        src: mutate(
          raw,
          'await groups.actExpansion(hit.id)',
          'await Promise.resolve(null)',
          'no-expansion',
        ),
        rule: 'C',
      },
      {
        name: '(8) снят маркер RETAINED-позиций `complectation_at_time !== undefined`',
        src: mutate(
          raw,
          "{#if mode === 'edit' && row.complectation_at_time !== undefined}",
          "{#if mode === 'edit' && row.complectation_at_time != null}",
          'no-retained-marker',
        ),
        rule: 'D',
      },
      {
        name: '(9) в fetchWorking `status_id: workingStatusId` заменён на `status_id: storageStatusId`',
        src: mutate(
          raw,
          '              status_id: workingStatusId,\n',
          '              status_id: storageStatusId,\n',
          'working-storage-status',
        ),
        rule: 'E',
      },
      {
        name: '(10) из fetchWorking убран вызов groups.forDevices(',
        src: mutate(
          raw,
          'await groups.forDevices(found.map((d) => d.id))',
          'await Promise.resolve([] as { device_id: number }[])',
          'working-no-owners',
        ),
        rule: 'E',
      },
      {
        name: '(11) SECTION_WORKING убрана из SECTION_ORDER',
        src: mutate(
          raw,
          '[SECTION_GROUPS, SECTION_STORAGE, SECTION_WORKING, SECTION_MEMBERS]',
          '[SECTION_GROUPS, SECTION_STORAGE, SECTION_MEMBERS]',
          'working-not-in-order',
        ),
        rule: 'A',
      },
      {
        name: '(12) из fetchMembers убран фильтр «Списано» (D-32)',
        src: mutate(
          raw,
          'd.status_code !== STATUS_WRITTEN_OFF',
          'd.status_code !== undefined',
          'members-no-written-off-filter',
        ),
        rule: 'F',
      },
      {
        name: '(13) B-WR-01: удалён весь блок отказа `if (fresh.length > free) { ... return; }`',
        src: deleteLimitBlock(raw),
        rule: 'C',
      },
      {
        name: '(14) B-WR-01: после тоста отказа удалён `return;`',
        src: mutate(
          raw,
          'Удалите лишние позиции и попробуйте снова.`,\n      );\n      return;\n    }',
          'Удалите лишние позиции и попробуйте снова.`,\n      );\n    }',
          'limit-no-return',
        ),
        rule: 'C',
      },
      {
        name: '(15) B-IN-05: серверный потолок в create расходится (> 99)',
        src: raw,
        server: mutate(
          readServerSource() ?? '',
          'if p.items.len() > 100 {\n            return Err(AppError::Validation {\n                field: "items".into(),\n                message: "Максимум 100 позиций в одном акте".into(),\n            });\n        }\n        // T-03.1-02',
          'if p.items.len() > 99 {\n            return Err(AppError::Validation {\n                field: "items".into(),\n                message: "Максимум 100 позиций в одном акте".into(),\n            });\n        }\n        // T-03.1-02',
          'server-ceiling',
        ),
        rule: 'C',
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const m of mutants) {
    const v = [];
    scanSource(m.src, v, m.server);
    const hit = v.some((line) => line.startsWith(`(${m.rule})`));
    console.error(
      `${TAG} [selftest] ${hit ? 'PASS' : 'FAIL'} — ${m.name} (ожидалось ${m.rule}, получено ${v.length})`,
    );
    if (!hit) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  // Зелёный контроль: запретные слова в КОММЕНТАРИИ не краснят.
  try {
    const commented = mutate(
      raw,
      '  // Серверный потолок позиций в одном акте',
      '  // раньше: status_id: storageStatusId в groups.search( и onChange(next) до MAX_ACT_ITEMS\n  // Серверный потолок позиций в одном акте',
      'comment-ok',
    );
    const v = [];
    scanSource(commented, v);
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
    `${TAG} [selftest] PASS — реальный исходник чист, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-act-group-picker.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  const raw = readSource(root, violations);
  if (raw !== null) scanSource(raw, violations);
  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
