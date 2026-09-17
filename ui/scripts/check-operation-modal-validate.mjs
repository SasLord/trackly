#!/usr/bin/env node
// [check-operation-modal-validate] Постоянный гейт против регрессии
// «клиент снова блокирует установку картриджа в принтер без места»
// (Phase 40, план 40-23, HST-01 / UAT3-01).
//
// Почему он существует: `validate()` в `OperationModal.svelte` до плана 40-23
// требовал `placeId` одной общей веткой `op === 'install' || op === 'to_refill'`.
// Планы 40-21/40-22 сделали сервер источником правды: `transition_in_tx`
// принимает `place_id: None` при установке в принтер (D-13 auto-resolve места
// принтера + шаг 5a обратной записи в `devices.place_id`). План 40-23 сузил
// клиентское требование до ЛЕГАСИ-пути (принтер не выбран,
// `effectivePrinterId === undefined`) и оставил `to_refill` безусловным —
// у него принтерного контекста нет вообще.
//
// Обе половины ломаются молча и по отдельности:
//   — если из install-условия выпадет `effectivePrinterId === undefined`,
//     форма снова начнёт блокировать submit в сценарии, который сервер
//     принимает (UAT-40 test 5 «картридж не едет за принтером»);
//   — если к `to_refill` припишут принтерное условие (например, «за компанию»,
//     при рефакторинге общей ветки), место при отправке на заправку станет
//     необязательным и уедет в БД пустым.
// Ни один существующий гейт этого не видит: `svelte-check`/`eslint`/`pnpm build`
// доказывают компилируемость, а Rust-тесты кончаются на границе DTO и вообще
// не знают про клиентскую валидацию.
//
// Гейт СТРУКТУРНЫЙ (по образцу check-place-path-short.mjs /
// check-print-idempotency.mjs): читает исходник и разбирает условия `if`
// скобочным балансом, НЕ рендерит компонент и не выполняет код. Он не
// доказывает, что форма ведёт себя правильно, — только то, что выстраданное
// условие не убрали при рефакторинге.
//
// Проверяемые инварианты:
//   INV-1 (40-23) — в теле `validate()` есть условие, требующее `placeId` для
//                   `op === 'install'`, и оно содержит `effectivePrinterId ===
//                   undefined` (требование ограничено легаси-путём).
//   INV-2 (40-23) — требование `placeId` для `op === 'to_refill'` живёт в
//                   ОТДЕЛЬНОЙ ветке (условие упоминает to_refill и не
//                   упоминает install) и БЕЗУСЛОВНО: внутренняя проверка —
//                   ровно `placeId === null`, без принтерных конъюнктов.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-operation-modal-validate.mjs              # проверить репозиторий
//   node scripts/check-operation-modal-validate.mjs --src=<dir>  # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-operation-modal-validate]';

const TARGET = 'src/features/cartridges/OperationModal.svelte';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/**
 * Затирает пробелами (сохраняя длину и переводы строк) все комментарии:
 * HTML (`<!-- -->`), блочные (`/* *\/`) и строчные (`//`). Без этого гейт
 * «проходил» бы по объяснению инварианта в doc-комментарии при мёртвом коде:
 * комментарии в этом файле дословно цитируют проверяемые условия.
 */
function stripComments(src) {
  const out = src.split('');
  const blank = (from, to) => {
    for (let i = from; i < to && i < src.length; i++) if (out[i] !== '\n') out[i] = ' ';
  };

  for (const re of [/<!--[\s\S]*?-->/g, /\/\*[\s\S]*?\*\//g]) {
    let m;
    while ((m = re.exec(src)) !== null) blank(m.index, m.index + m[0].length);
  }

  let joined = out.join('');
  const lines = joined.split('\n');
  let offset = 0;
  for (const line of lines) {
    const idx = line.indexOf('//');
    if (idx >= 0) {
      const before = line.slice(0, idx);
      const quotes = (before.match(/['"`]/g) ?? []).length;
      // Нечётное число кавычек слева — `//` внутри строкового литерала.
      if (quotes % 2 === 0) blank(offset + idx, offset + line.length);
    }
    offset += line.length + 1;
  }

  joined = out.join('');
  return joined;
}

/** Тело функции по имени (учитывает вложенные `{}`). */
function functionBody(src, fnName) {
  const m = src.match(new RegExp(`(?:async\\s+)?function\\s+${fnName}\\s*\\(`));
  if (!m) return null;
  const open = src.indexOf('{', m.index + m[0].length);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return src.slice(open + 1, i);
    }
  }
  return null;
}

/** Балансное содержимое скобок, начиная с индекса открывающей `(`. */
function balancedParens(src, openIdx) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === '(') depth++;
    else if (src[i] === ')') {
      depth--;
      if (depth === 0) return { text: src.slice(openIdx + 1, i), end: i };
    }
  }
  return null;
}

/** Блок `{...}`, следующий сразу за индексом `from` (пропускает пробелы). */
function blockAfter(src, from) {
  let i = from;
  while (i < src.length && /\s/.test(src[i])) i++;
  if (src[i] !== '{') return null;
  let depth = 0;
  for (let j = i; j < src.length; j++) {
    if (src[j] === '{') depth++;
    else if (src[j] === '}') {
      depth--;
      if (depth === 0) return src.slice(i + 1, j);
    }
  }
  return null;
}

/** Все условия `if (...)` в тексте вместе с телом соответствующего блока. */
function ifClauses(src) {
  const out = [];
  const re = /\bif\s*\(/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    const openIdx = m.index + m[0].length - 1;
    const parens = balancedParens(src, openIdx);
    if (parens === null) continue;
    out.push({
      condition: parens.text.replace(/\s+/g, ' ').trim(),
      body: blockAfter(src, parens.end + 1),
    });
  }
  return out;
}

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

function checkInv1(validateBody, violations) {
  const clauses = ifClauses(validateBody).filter(
    (c) => c.condition.includes("op === 'install'") && c.condition.includes('placeId === null'),
  );

  if (clauses.length === 0) {
    violations.push({
      inv: 'INV-1',
      message:
        "в теле validate() нет условия, объединяющего `op === 'install'` и `placeId === null`",
      hint:
        'Либо требование места для install удалили целиком, либо его СНОВА слили с to_refill в общую ' +
        "ветку (`op === 'install' || op === 'to_refill'` + вложенный `if (placeId === null)`) — это ровно " +
        'до-40-23 состояние: клиент блокирует установку в принтер без места, хотя сервер (планы 40-21/40-22, ' +
        'D-13) такой запрос принимает и сам проставляет место принтера. Обнови гейт вместе с рефакторингом ' +
        'осознанно, а не удаляй проверку.',
    });
    return;
  }

  for (const c of clauses) {
    if (!c.condition.includes('effectivePrinterId === undefined')) {
      violations.push({
        inv: 'INV-1',
        message: `условие \`if (${c.condition})\` в validate() не содержит \`effectivePrinterId === undefined\``,
        hint:
          'Требование «Место» для install обязано быть ограничено ЛЕГАСИ-путём (принтер не выбран). Без этого ' +
          'конъюнкта форма снова блокирует submit при установке в принтер без места — регресс UAT-40 test 5, ' +
          'закрытый планом 40-23 в паре с серверными 40-21/40-22.',
      });
    }
  }
}

function checkInv2(validateBody, violations) {
  const clauses = ifClauses(validateBody).filter(
    (c) => c.condition.includes("op === 'to_refill'") && !c.condition.includes("op === 'install'"),
  );

  if (clauses.length === 0) {
    violations.push({
      inv: 'INV-2',
      message: "в теле validate() нет отдельной ветки для `op === 'to_refill'`",
      hint:
        'Требование места для «Отправки на заправку» снова живёт в общей с install ветке — у to_refill нет ' +
        'принтерного контекста вообще, и любое ослабление install-условия немедленно ослабляет и его. Держи ' +
        'ветки раздельными (решение плана 40-23).',
    });
    return;
  }

  for (const c of clauses) {
    if (/effectivePrinterId|preFillPrinterId|selectedPrinterId/.test(c.condition)) {
      violations.push({
        inv: 'INV-2',
        message: `ветка to_refill в validate() гейтована принтерным контекстом: \`if (${c.condition})\``,
        hint:
          'У to_refill нет принтера в payload (легаси-путь D-08 применяется всегда) — место обязано требоваться ' +
          'безусловно, иначе отправка на заправку уедет с пустым местом.',
      });
      continue;
    }

    const body = c.body;
    if (body === null) {
      violations.push({
        inv: 'INV-2',
        message: "не удалось разобрать тело ветки `op === 'to_refill'` в validate()",
        hint: 'Структура ветки изменилась — обнови гейт осознанно, а не удаляй проверку.',
      });
      continue;
    }

    const inner = ifClauses(body).filter((i) => i.condition.includes('placeId === null'));
    if (inner.length === 0) {
      violations.push({
        inv: 'INV-2',
        message: "внутри ветки `op === 'to_refill'` нет проверки `placeId === null`",
        hint:
          'Место при «Отправке на заправку» перестало быть обязательным на клиенте — операция уедет с пустым ' +
          'местом, и автоподстановка следующей отправки (план 40-35, UAT4-02) тоже останется без источника.',
      });
      continue;
    }

    for (const i of inner) {
      if (i.condition !== 'placeId === null') {
        violations.push({
          inv: 'INV-2',
          message: `проверка места в ветке to_refill стала условной: \`if (${i.condition})\``,
          hint:
            'Требование `placeId` для to_refill обязано быть БЕЗУСЛОВНЫМ (ровно `placeId === null`, без ' +
            'дополнительных конъюнктов) — послабление, сделанное для install (принтер сам резолвит место), ' +
            'к to_refill неприменимо.',
        });
      }
    }
  }
}

// ---------------------------------------------------------------------------

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-operation-modal-validate.mjs [--src=<ui-dir>]`);
    process.exit(0);
  }

  const full = path.join(SRC_ROOT, TARGET);
  let source;
  try {
    source = fs.readFileSync(full, 'utf8');
  } catch {
    console.error(
      `${TAG} FAIL — не удалось прочитать ${TARGET}. Компонент переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
    );
    process.exit(1);
  }

  const code = stripComments(source);
  const validateBody = functionBody(code, 'validate');

  const violations = [];
  if (validateBody === null) {
    violations.push({
      inv: 'INV-0',
      message: 'не найдена функция validate() в ' + TARGET,
      hint: 'Валидацию переименовали/вынесли — гейт больше ничего не держит. Обнови гейт вместе с рефакторингом.',
    });
  } else {
    checkInv1(validateBody, violations);
    checkInv2(validateBody, violations);
  }

  for (const v of violations) {
    console.error(`${TAG} ${TARGET} — ${v.inv}: ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений инвариантов validate() в ${TARGET}. ` +
        'Этот класс дефекта уже уезжал в живую UAT (UAT-40 test 5) — не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
