#!/usr/bin/env node
// [check-operation-modal-autofill] Постоянный гейт против регрессий
// автозаполнения полей в `OperationModal.svelte` (Phase 40, планы 40-31 и
// 40-35, требования UAT3-01 / UAT4-02 / UAT4-03 / UAT5-01).
//
// Почему он существует: каждый из четырёх инвариантов ниже — это записанный
// дефект, который прошёл `svelte-check`, `eslint`, `pnpm build` и весь набор
// Rust-тестов и был пойман ТОЛЬКО живым прогоном приложения (см. 40-31-SUMMARY,
// 40-35-SUMMARY). Сегодня их держат только длинные комментарии в исходнике —
// то есть ничего. Гейт превращает «мы это уже проходили» в падающую проверку.
//
// Проверяемые инварианты:
//   INV-1 (40-35 split-effect) — автозаполнение `from_refill` (место, через
//           `operationDefaultPlace`) и `to_refill` (три поля, через
//           `toRefillLastSend`) живут в ДВУХ РАЗНЫХ `$effect`, каждый гейтован
//           своим `op`. Один эффект с внутренним ветвлением по `op` — это
//           состояние до 40-35: бэкенд-контракты разошлись (40-33 удалил ветку
//           to_refill из `operation_default_place`, она падает в
//           AppError::Validation), и общий эффект снова начнёт звать не тот
//           эндпоинт.
//   INV-2 (WR-01 per-field guard) — каждое автозаполняемое поле пишется, только
//           если оно ЕЩЁ ПУСТО В МОМЕНТ РАЗРЕШЕНИЯ ПРОМИСА: проверка пустоты
//           стоит внутри `.then(...)`, отдельная на каждое поле (три разных
//           условия, не одно комбинированное), и НЕ вынесена перед запросом.
//           Иначе ответ сервера затирает то, что оператор успел ввести руками,
//           пока запрос был в полёте (UAT4-02 п.2 живой проверки).
//   INV-3 (WR-01 ручной выбор) — каждый обработчик `<PlacePicker onChange>`,
//           присваивающий `placeId`, обязан сбрасывать `placeAutofilled = false`
//           (иначе следующий автозаполняющий проход сочтёт ручной выбор своим
//           и перезапишет его). Таких обработчика ДВА — симметрия групп
//           to_refill/install и return_to_stock/from_refill, решение 40-31.
//   INV-4 (UAT5-01 cross-effect clobber) — DEC-B-очистка в install-эффекте
//           (`placeId = null; placeAutofilled = false`) обязана быть гейтована
//           `op === 'install'`. Без этого гейта ветка раннего выхода
//           install-эффекта просыпается на изменение `placeAutofilled` при
//           ЛЮБОЙ операции и мгновенно затирает только что подставленное место
//           заправки обратно в null — ровно тот дефект «холодное открытие
//           работает, повторное — нет», который стоил раунда живой UAT
//           (40-31-SUMMARY, UAT3-01b).
//
// Гейт СТРУКТУРНЫЙ (как check-place-path-short.mjs / check-print-idempotency.mjs):
// читает исходник и разбирает его скобочным балансом, НЕ выполняет код и НЕ
// доказывает правильность рантайма рун — поведение по-прежнему проверяется
// живым прогоном. Его задача — громко падать, когда выстраданный инвариант
// молча убрали при рефакторинге.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-operation-modal-autofill.mjs              # проверить репозиторий
//   node scripts/check-operation-modal-autofill.mjs --src=<dir>  # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-operation-modal-autofill]';

const TARGET = 'src/features/cartridges/OperationModal.svelte';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/**
 * Затирает пробелами (сохраняя длину и переводы строк) все комментарии: HTML,
 * блочные и строчные. Критично именно здесь: doc-комментарии этого файла
 * ДОСЛОВНО цитируют проверяемые условия (`if (preFillPrinterId === undefined &&
 * placeAutofilled) { placeId = null; ... }`), поэтому без вычистки комментариев
 * гейт «проходил» бы по объяснению при удалённом коде.
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

  const lines = out.join('').split('\n');
  let offset = 0;
  for (const line of lines) {
    const idx = line.indexOf('//');
    if (idx >= 0) {
      const before = line.slice(0, idx);
      const quotes = (before.match(/['"`]/g) ?? []).length;
      if (quotes % 2 === 0) blank(offset + idx, offset + line.length);
    }
    offset += line.length + 1;
  }

  return out.join('');
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

/** Тексты всех блоков `$effect(...)`. */
function effectBlocks(src) {
  const out = [];
  const re = /\$effect\s*\(/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    const parens = balancedParens(src, m.index + m[0].length - 1);
    if (parens !== null) out.push(parens.text);
  }
  return out;
}

/** Тела всех коллбэков `.then(...)` в тексте. */
function thenBodies(src) {
  const out = [];
  const re = /\.then\s*\(/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    const parens = balancedParens(src, m.index + m[0].length - 1);
    if (parens === null) continue;
    const arrow = parens.text.indexOf('=>');
    if (arrow < 0) continue;
    const body = blockAfter(parens.text, arrow + 2);
    if (body !== null) out.push(body);
  }
  return out;
}

/**
 * Условие ближайшего охватывающего `if` для позиции `idx`: сперва форма без
 * блока (`if (cond) stmt;` на той же строке), затем — `if (cond) { ... idx ... }`.
 * Возвращает null, если ближайший охватывающий блок открыт не через `if`.
 */
function enclosingIfCondition(src, idx) {
  const lineStart = src.lastIndexOf('\n', idx) + 1;
  const prefix = src.slice(lineStart, idx);
  const lastIf = prefix.lastIndexOf('if (');
  if (lastIf >= 0) {
    const parens = balancedParens(prefix, lastIf + 3);
    if (parens !== null && prefix.slice(parens.end + 1).trim() === '') {
      return parens.text.replace(/\s+/g, ' ').trim();
    }
  }

  let depth = 0;
  for (let i = idx - 1; i >= 0; i--) {
    const c = src[i];
    if (c === '}') depth++;
    else if (c === '{') {
      if (depth === 0) {
        let j = i - 1;
        while (j >= 0 && /\s/.test(src[j])) j--;
        if (src[j] !== ')') return null;
        let d = 0;
        let open = -1;
        for (let k = j; k >= 0; k--) {
          if (src[k] === ')') d++;
          else if (src[k] === '(') {
            d--;
            if (d === 0) {
              open = k;
              break;
            }
          }
        }
        if (open < 0) return null;
        let w = open - 1;
        while (w >= 0 && /\s/.test(src[w])) w--;
        if (!/\bif$/.test(src.slice(0, w + 1))) return null;
        return src
          .slice(open + 1, j)
          .replace(/\s+/g, ' ')
          .trim();
      }
      depth--;
    }
  }
  return null;
}

/** Индексы присваиваний `name = ...` (не сравнений) в тексте. */
function assignmentIndices(src, name) {
  const out = [];
  const re = new RegExp(`\\b${name}\\s*=(?!=)`, 'g');
  let m;
  while ((m = re.exec(src)) !== null) out.push(m.index);
  return out;
}

/** Открывающие теги `<PlacePicker ...>` с учётом вложенных `{}`. */
function placePickerTags(src) {
  const out = [];
  const re = /<PlacePicker[\s>]/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    let depth = 0;
    for (let i = m.index; i < src.length; i++) {
      const c = src[i];
      if (c === '{') depth++;
      else if (c === '}') depth--;
      else if (c === '>' && depth === 0) {
        out.push(src.slice(m.index, i + 1));
        break;
      }
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

/** Возвращает { fromEffect, toEffect } или null, если INV-1 нарушен. */
function checkInv1(effects, violations) {
  const withFrom = effects.filter((e) => e.includes("op === 'from_refill'"));
  const withTo = effects.filter((e) => e.includes("op === 'to_refill'"));
  const combined = effects.filter(
    (e) => e.includes("op === 'from_refill'") && e.includes("op === 'to_refill'"),
  );

  if (combined.length > 0) {
    violations.push({
      inv: 'INV-1',
      message: "найден один $effect, гейтованный сразу на 'from_refill' и 'to_refill'",
      hint:
        'Это состояние до плана 40-35: комбинированный эффект с ветвлением внутри. Бэкенд-контракты разошлись — ' +
        '40-33 удалил ветку to_refill из operation_default_place (теперь AppError::Validation) и завёл отдельный ' +
        'эндпоинт toRefillLastSend на три поля. Держи два независимых эффекта, каждый со своим гейтом по op.',
    });
    return null;
  }

  if (withFrom.length !== 1 || withTo.length !== 1) {
    violations.push({
      inv: 'INV-1',
      message: `ожидался ровно один $effect на каждый op автозаполнения, найдено from_refill=${withFrom.length}, to_refill=${withTo.length}`,
      hint: 'Автозаполнение переструктурировали — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
    return null;
  }

  const fromEffect = withFrom[0];
  const toEffect = withTo[0];

  if (!fromEffect.includes('operationDefaultPlace(')) {
    violations.push({
      inv: 'INV-1',
      message: "$effect для 'from_refill' не вызывает cartridges.operationDefaultPlace(",
      hint: 'Источник дефолтного места для «Получения с заправки» подменён — обнови гейт осознанно.',
    });
  }
  if (!toEffect.includes('toRefillLastSend(')) {
    violations.push({
      inv: 'INV-1',
      message: "$effect для 'to_refill' не вызывает cartridges.toRefillLastSend(",
      hint:
        'Три поля «Отправки на заправку» обязаны браться из ОДНОЙ записи (план 40-33/40-35). Вызов ' +
        "operationDefaultPlace с op='to_refill' на бэкенде теперь падает в AppError::Validation.",
    });
  }

  return { fromEffect, toEffect };
}

/** Одно поле: guard по пустоте стоит внутри .then и охватывает присваивание. */
function checkFieldGuard(thenBody, field, emptinessCheck, label, violations, conditions) {
  const assigns = assignmentIndices(thenBody, field);
  if (assigns.length === 0) {
    violations.push({
      inv: 'INV-2',
      message: `в .then(...) эффекта ${label} нет присваивания \`${field} = ...\``,
      hint: 'Автозаполнение поля удалено или переписано — обнови гейт вместе с рефакторингом осознанно.',
    });
    return;
  }
  for (const idx of assigns) {
    const cond = enclosingIfCondition(thenBody, idx);
    if (cond === null || !cond.includes(emptinessCheck)) {
      violations.push({
        inv: 'INV-2',
        message: `присваивание \`${field} = ...\` в .then(...) эффекта ${label} не защищено проверкой \`${emptinessCheck}\` (ближайшее условие: ${cond === null ? '— нет —' : '`' + cond + '`'})`,
        hint:
          'WR-01: подставлять значение можно, только пока поле ещё пусто В МОМЕНТ РАЗРЕШЕНИЯ ПРОМИСА. Без этой ' +
          'проверки поздний ответ сервера затирает то, что оператор успел ввести руками, пока запрос был в полёте.',
      });
    } else {
      conditions.push(cond);
    }
  }
}

function checkInv2(effects, violations) {
  const { fromEffect, toEffect } = effects;

  // (a) from_refill — одно поле.
  const fromThens = thenBodies(fromEffect);
  if (fromThens.length === 0) {
    violations.push({
      inv: 'INV-2',
      message: "в $effect для 'from_refill' не найден коллбэк .then(...)",
      hint: 'Гейт не может определить «момент разрешения промиса» — обнови гейт вместе с рефакторингом осознанно.',
    });
  } else {
    const conds = [];
    checkFieldGuard(fromThens[0], 'placeId', 'placeId === null', 'from_refill', violations, conds);
  }

  // (b) to_refill — три поля, три НЕЗАВИСИМЫХ условия.
  const toThens = thenBodies(toEffect);
  if (toThens.length === 0) {
    violations.push({
      inv: 'INV-2',
      message: "в $effect для 'to_refill' не найден коллбэк .then(...)",
      hint: 'Гейт не может определить «момент разрешения промиса» — обнови гейт вместе с рефакторингом осознанно.',
    });
  } else {
    const conds = [];
    checkFieldGuard(
      toThens[0],
      'givenByName',
      "givenByName === ''",
      'to_refill',
      violations,
      conds,
    );
    checkFieldGuard(
      toThens[0],
      'givenToName',
      "givenToName === ''",
      'to_refill',
      violations,
      conds,
    );
    checkFieldGuard(toThens[0], 'placeId', 'placeId === null', 'to_refill', violations, conds);
    if (conds.length === 3 && new Set(conds).size !== 3) {
      violations.push({
        inv: 'INV-2',
        message: 'три поля to_refill защищены ОДНИМ общим условием, а не тремя независимыми',
        hint:
          'Решение плана 40-35: частичная ручная правка одного поля (сделанная, пока запрос ещё в полёте) не должна ' +
          'блокировать автозаполнение двух остальных. Комбинированный guard именно это и делает.',
      });
    }
  }

  // (c) guard не вынесен ПЕРЕД запросом (hoisted) — тогда он проверял бы
  //     пустоту до ожидания, а не в момент разрешения.
  for (const [label, effect] of [
    ['from_refill', fromEffect],
    ['to_refill', toEffect],
  ]) {
    const callIdx = effect.indexOf('cartridges');
    if (callIdx < 0) continue;
    const pre = effect.slice(0, callIdx);
    const hoisted = [
      'placeId === null',
      'placeId !== null',
      "givenByName === ''",
      "givenToName === ''",
    ].filter((p) => pre.includes(p));
    if (hoisted.length > 0) {
      violations.push({
        inv: 'INV-2',
        message: `в $effect для '${label}' проверка пустоты (${hoisted.join(', ')}) вынесена ПЕРЕД запросом`,
        hint:
          'Guard обязан стоять внутри .then(...): состояние поля в момент ОТПРАВКИ запроса ничего не говорит о его ' +
          'состоянии в момент ответа — ровно в этот промежуток оператор и правит поле руками.',
      });
    }
  }
}

function checkInv3(code, violations) {
  const tags = placePickerTags(code);
  let placeIdHandlers = 0;

  for (const tag of tags) {
    const m = tag.match(/onChange\s*=\s*\{/);
    if (!m) continue;
    const open = m.index + m[0].length - 1;
    let depth = 0;
    let handler = null;
    for (let i = open; i < tag.length; i++) {
      if (tag[i] === '{') depth++;
      else if (tag[i] === '}') {
        depth--;
        if (depth === 0) {
          handler = tag.slice(open + 1, i);
          break;
        }
      }
    }
    if (handler === null) continue;
    if (assignmentIndices(handler, 'placeId').length === 0) continue;
    placeIdHandlers++;
    if (!/placeAutofilled\s*=\s*false/.test(handler)) {
      violations.push({
        inv: 'INV-3',
        message:
          'обработчик <PlacePicker onChange>, присваивающий placeId, не сбрасывает `placeAutofilled = false`',
        hint:
          'WR-01: ручной выбор места обязан снимать признак автозаполнения, иначе следующий проход автозаполнения ' +
          '(смена принтера или ответ сервера) сочтёт выбор оператора своим и перезапишет его.',
      });
    }
  }

  if (placeIdHandlers < 2) {
    violations.push({
      inv: 'INV-3',
      message: `обработчиков <PlacePicker onChange>, присваивающих placeId, найдено ${placeIdHandlers}, ожидалось не менее 2`,
      hint:
        'Их два по решению плана 40-31: блок to_refill/install и симметричный блок return_to_stock/from_refill. ' +
        'Если блок действительно объединили — обнови гейт осознанно, а не удаляй проверку.',
    });
  }
}

function checkInv4(effects, violations) {
  const installEffects = effects.filter(
    (e) => e.includes("op === 'install'") && e.includes('effectivePrinterId !== undefined'),
  );
  if (installEffects.length !== 1) {
    violations.push({
      inv: 'INV-4',
      message: `ожидался ровно один install-$effect (op === 'install' + effectivePrinterId !== undefined), найдено ${installEffects.length}`,
      hint: 'Install-эффект переструктурировали — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
    return;
  }

  const effect = installEffects[0];
  const resets = assignmentIndices(effect, 'placeAutofilled').filter((i) =>
    /placeAutofilled\s*=\s*false/.test(effect.slice(i, i + 40)),
  );
  if (resets.length === 0) {
    violations.push({
      inv: 'INV-4',
      message: 'в install-$effect нет DEC-B-очистки `placeAutofilled = false`',
      hint: 'Очистку удалили/перенесли — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
    return;
  }

  for (const idx of resets) {
    const cond = enclosingIfCondition(effect, idx);
    if (cond === null || !cond.includes("op === 'install'")) {
      violations.push({
        inv: 'INV-4',
        message: `DEC-B-очистка в install-$effect не гейтована \`op === 'install'\` (ближайшее условие: ${cond === null ? '— нет —' : '`' + cond + '`'})`,
        hint:
          'Это ровно дефект UAT3-01b/UAT5-01: ветка раннего выхода install-эффекта выполняется при ЛЮБОЙ операции, ' +
          'читает `placeAutofilled`, поэтому просыпается сразу после того, как эффект заправки подставил место, — и ' +
          'через несколько микрозадач возвращает placeId в null незаметно для оператора. Гонки тут нет, ломается ' +
          'каждый раз.',
      });
    }
  }
}

// ---------------------------------------------------------------------------

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-operation-modal-autofill.mjs [--src=<ui-dir>]`);
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
  const effects = effectBlocks(code);
  const violations = [];

  if (effects.length === 0) {
    violations.push({
      inv: 'INV-0',
      message: `в ${TARGET} не найдено ни одного $effect`,
      hint: 'Компонент переписан на другую модель реактивности — обнови гейт осознанно.',
    });
  } else {
    const pair = checkInv1(effects, violations);
    if (pair !== null) checkInv2(pair, violations);
    checkInv3(code, violations);
    checkInv4(effects, violations);
  }

  for (const v of violations) {
    console.error(`${TAG} ${TARGET} — ${v.inv}: ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений инвариантов автозаполнения в ${TARGET}. ` +
        'Каждый из них — уже случившийся дефект, пойманный только живым прогоном (40-31/40-35): не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
