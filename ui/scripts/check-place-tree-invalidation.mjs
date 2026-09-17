#!/usr/bin/env node
// [check-place-tree-invalidation] Постоянный гейт против регрессий инвалидации
// счётчиков содержимого в дереве «Места» (Phase 40, план 40-32, UAT3-03).
//
// Почему он существует: первая реализация инвалидации прошла `svelte-check`,
// `eslint`, `pnpm build` и весь набор тестов — и ВЕШАЛА реактивность всей
// страницы (`effect_update_depth_exceeded`), потому что `$effect` инвалидации
// одновременно ЧИТАЛ `statsCache` реактивно и БЕЗУСЛОВНО его перезаписывал:
// каждая запись создаёт новую ссылку на объект, эффект видит собственную
// зависимость изменившейся и перезапускается. Дефект пойман только живым
// прогоном и исправлен отдельным коммитом 5042e674 (см. 40-32-SUMMARY).
// Сегодня инвариант держит только комментарий в исходнике — то есть ничего.
//
// Проверяемые инварианты:
//   INV-1 (5042e674) — в `$effect` инвалидации ВСЕ обращения к `statsCache`
//           находятся внутри `untrack(...)`. Реактивное чтение возвращает
//           бесконечный цикл эффекта и заморозку страницы.
//   INV-2 (5042e674) — обратная запись `statsCache = ...` в этом эффекте
//           УСЛОВНА (выполняется, только если что-то реально удалено): прогон
//           без изменений не должен создавать новую ссылку на объект. Вторая
//           половина той же защиты, держится отдельно от первой.
//   INV-3 (40-32)   — инвалидируются не только сообщённые id, но и ВСЕ их
//           предки (`ancestorsAndSelf`), потому что счётчик — итог по
//           поддереву: изменение глубоко в дереве обесценивает счётчики всей
//           цепочки вверх. Сама `ancestorsAndSelf` обязана идти по `parent_id`
//           в цикле, а не возвращать один элемент.
//   INV-4 (40-32)   — эффект СОХРАНЯЕТ реактивную зависимость от store:
//           хотя бы одно чтение `placeContentEventsStore` остаётся ВНЕ
//           `untrack`. Если убрать untrack «слишком хорошо» и спрятать туда
//           и store, эффект перестанет просыпаться вовсе — молчаливая смерть
//           инвалидации, симметричная бесконечному циклу.
//   INV-5 (соседний эффект) — ленивый догруз статистики сохраняет гейт
//           `statsCache[id] !== undefined`: он имеет ту же форму «читает и
//           пишет один и тот же $state» и сходится ТОЛЬКО благодаря этому
//           гейту (зафиксировано комментарием в исходнике).
//   INV-6 (store)   — `notifyPlaceContentChanged` монотонно увеличивает `seq`
//           и записывает `placeIds`. Без инкремента `seq` повторное событие с
//           тем же списком id не разбудит подписчика (эффект отсекает `seq === 0`
//           и просыпается именно на изменение `seq`).
//   INV-7 (40.1, аудит 2026-09-17, WARNING-1) — ПРОДЮСЕРЫ `notifyPlaceContentChanged`
//           покрывают все клиентские write-site смены места, а не только массовый
//           перенос (`PlaceContents.svelte`'s `handleMoveConfirm`). WR-01 из
//           аудита 2026-08-27 уже закрывался «наполовину» — новый write-site
//           тихо не звал функцию, и ничего в проекте это не ловило. Проверка
//           КАЖДЫЙ write-site РАЗДЕЛЬНО, а не «хотя бы один вызов где-то в
//           репозитории»: `PlaceContents.svelte` и (с 40.1, gap closure WR-01)
//           `CartridgesPage.svelte` содержат ПО ДВА разных вызова каждый —
//           файл-уровневая проверка не отличила бы удаление одного из них от
//           присутствия другого (mutation_test_anchor_must_be_unique). Поэтому
//           для `PlaceContents.svelte` проверка локализована на блок разметки
//           `<PlaceEntityViewModal ...>` (правка через «Просмотр»), а для
//           `CartridgesPage.svelte` — на ТЕЛА функций `handleFormSuccess`
//           (форма «Редактировать») и `handleOperationSuccess` (lifecycle-
//           операции install/return_to_stock/to_refill/from_refill/write_off,
//           WR-01 gap closure 2026-09-18) РАЗДЕЛЬНО; `DevicesPage.svelte`
//           остаётся файл-уровневой проверкой — в нём ровно один вызов.
//
// Гейт СТРУКТУРНЫЙ (как check-place-path-short.mjs / check-print-idempotency.mjs):
// читает исходники и разбирает их скобочным балансом, НЕ выполняет код и НЕ
// доказывает отсутствие цикла в рантайме — это по-прежнему проверяется живым
// прогоном. Его задача — громко падать, когда выстраданный фикс молча убрали.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-place-tree-invalidation.mjs              # проверить репозиторий
//   node scripts/check-place-tree-invalidation.mjs --src=<dir>  # проверить копию (самотест гейта)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-place-tree-invalidation]';

const TREE = 'src/features/places/PlaceTree.svelte';
const STORE = 'src/lib/stores/placeContentEvents.svelte.ts';
// INV-7 producer write-sites (WARNING-1, audit 2026-09-17).
const CONTENTS = 'src/features/places/PlaceContents.svelte';
const CARTRIDGES_PAGE = 'src/features/cartridges/CartridgesPage.svelte';
const DEVICES_PAGE = 'src/features/devices/DevicesPage.svelte';
const NOTIFY = 'notifyPlaceContentChanged(';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/**
 * Затирает пробелами (сохраняя длину и переводы строк) все комментарии: HTML,
 * блочные и строчные. Обязательно: комментарии в обоих файлах дословно
 * цитируют проверяемый код (`{ ...statsCache }`, `statsCache = next`), и без
 * вычистки гейт «проходил» бы по объяснению при удалённом коде.
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
      if (depth === 0) return { text: src.slice(openIdx + 1, i), start: openIdx, end: i };
    }
  }
  return null;
}

/**
 * INV-7: срез разметки Svelte-тега `<${tagName} ... />` или `<${tagName} ...>`,
 * от `<tagName` до закрывающего `>` тега (включая self-closing `/>`).
 * Балансирует ФИГУРНЫЕ скобки, не круглые — атрибуты Svelte-тега вида
 * `onChanged={() => { ... }}` сами содержат вложенные `{}`, и наивный поиск
 * первого `>` попал бы внутрь `{() => {...}}`. Возвращает `null`, если тег не
 * найден (компонент удалён/переименован — гейт должен явно упасть на этом,
 * не молча пройти).
 */
function tagBlock(src, tagName) {
  const start = src.indexOf(`<${tagName}`);
  if (start < 0) return null;
  let depth = 0;
  for (let i = start; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') depth--;
    else if (src[i] === '>' && depth === 0) return src.slice(start, i + 1);
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

/** Диапазоны [start, end) всех вызовов `untrack(...)` в тексте. */
function untrackRanges(src) {
  const out = [];
  const re = /\buntrack\s*\(/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    const parens = balancedParens(src, m.index + m[0].length - 1);
    if (parens !== null) out.push([m.index, parens.end + 1]);
  }
  return out;
}

const inRanges = (ranges, idx) => ranges.some(([a, b]) => idx >= a && idx < b);

/** Индексы всех вхождений подстроки. */
function indicesOf(src, needle) {
  const out = [];
  let i = src.indexOf(needle);
  while (i >= 0) {
    out.push(i);
    i = src.indexOf(needle, i + 1);
  }
  return out;
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

/**
 * Условие ближайшего охватывающего `if` для позиции `idx`: сперва форма без
 * блока (`if (cond) stmt;` на той же строке), затем — `if (cond) { ... idx ... }`.
 * Возвращает null, если ближайший охватывающий блок открыт НЕ через `if`
 * (например `for (...) {`, `untrack(() => {` или тело функции).
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

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

function checkTree(treeSrc, violations) {
  const code = stripComments(treeSrc);
  const effects = effectBlocks(code);

  if (!/import\s*\{[^}]*\buntrack\b[^}]*\}\s*from\s*'svelte'/.test(code)) {
    violations.push({
      file: TREE,
      inv: 'INV-1',
      message: "`untrack` не импортируется из 'svelte'",
      hint: 'Без untrack эффект инвалидации реактивно читает собственный кэш и уходит в бесконечный цикл (effect_update_depth_exceeded).',
    });
  }

  const invalidation = effects.filter((e) => e.includes('placeContentEventsStore'));
  if (invalidation.length !== 1) {
    violations.push({
      file: TREE,
      inv: 'INV-1',
      message: `ожидался ровно один $effect, читающий placeContentEventsStore, найдено ${invalidation.length}`,
      hint: 'Подписку на события инвалидации удалили или размножили — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
  } else {
    const effect = invalidation[0];
    const ranges = untrackRanges(effect);

    // INV-1 — все обращения к statsCache внутри untrack.
    const leaked = indicesOf(effect, 'statsCache').filter((i) => !inRanges(ranges, i));
    if (leaked.length > 0) {
      violations.push({
        file: TREE,
        inv: 'INV-1',
        message: `в $effect инвалидации есть ${leaked.length} обращений к statsCache ВНЕ untrack(...)`,
        hint:
          'Ровно дефект UAT3-03a: эффект читает statsCache реактивно и тут же его перезаписывает — новая ссылка на ' +
          'объект будит этот же эффект, Svelte 5 обрывает цикл ошибкой effect_update_depth_exceeded и замораживает ' +
          'реактивность всей страницы. Чтение кэша обязано идти через untrack().',
      });
    }

    // INV-2 — запись условна.
    const writes = indicesOf(effect, 'statsCache').filter((i) =>
      /^statsCache\s*=(?!=)/.test(effect.slice(i)),
    );
    if (writes.length === 0) {
      violations.push({
        file: TREE,
        inv: 'INV-2',
        message: 'в $effect инвалидации нет обратной записи `statsCache = ...`',
        hint: 'Инвалидация ничего не вычищает — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
      });
    }
    for (const idx of writes) {
      const cond = enclosingIfCondition(effect, idx);
      if (cond === null) {
        violations.push({
          file: TREE,
          inv: 'INV-2',
          message: 'обратная запись `statsCache = ...` в $effect инвалидации БЕЗУСЛОВНА',
          hint:
            'Вторая половина фикса 5042e674: прогон, в котором ничего не удалилось, не должен создавать новую ссылку ' +
            'на объект кэша. Безусловная запись возвращает самоподдерживающийся цикл эффекта.',
        });
      }
    }

    // INV-3 — предки.
    if (!effect.includes('ancestorsAndSelf(')) {
      violations.push({
        file: TREE,
        inv: 'INV-3',
        message: '$effect инвалидации не разворачивает сообщённые id через ancestorsAndSelf(',
        hint:
          'Счётчик узла — итог ПО ПОДДЕРЕВУ («Всего с вложенными»), поэтому изменение состава места обесценивает ' +
          'счётчики всех его предков. Инвалидация только самих сообщённых id оставляет цепочку вверх устаревшей — ' +
          'ровно тот симптом, с которого начался UAT3-03.',
      });
    }

    // INV-4 — реактивная зависимость от store сохранена.
    const storeReadsOutside = indicesOf(effect, 'placeContentEventsStore').filter(
      (i) => !inRanges(ranges, i),
    );
    if (storeReadsOutside.length === 0) {
      violations.push({
        file: TREE,
        inv: 'INV-4',
        message:
          'все чтения placeContentEventsStore в $effect инвалидации спрятаны внутрь untrack(...)',
        hint:
          'Тогда у эффекта не остаётся ни одной реактивной зависимости и он больше никогда не просыпается — ' +
          'инвалидация умирает молча, без единой ошибки в консоли. untrack предназначен ТОЛЬКО для чтения statsCache.',
      });
    }
  }

  // INV-3 (вторая половина) — ancestorsAndSelf реально идёт вверх по дереву.
  const ancestors = functionBody(code, 'ancestorsAndSelf');
  if (ancestors === null) {
    violations.push({
      file: TREE,
      inv: 'INV-3',
      message: 'функция ancestorsAndSelf не найдена',
      hint: 'Переименована/удалена — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
  } else if (!ancestors.includes('parent_id') || !/\b(while|for)\b/.test(ancestors)) {
    violations.push({
      file: TREE,
      inv: 'INV-3',
      message: 'ancestorsAndSelf не идёт вверх по цепочке parent_id в цикле',
      hint:
        'Если функция вернёт только сам узел, инвалидация формально «есть», но счётчики предков останутся ' +
        'устаревшими — дефект UAT3-03 воспроизводится, а гейт бы это пропустил.',
    });
  }

  // INV-5 — соседний ленивый эффект сохраняет свой сходящийся гейт.
  const lazy = effects.filter((e) => e.includes('places_subtree_stats'));
  if (lazy.length !== 1) {
    violations.push({
      file: TREE,
      inv: 'INV-5',
      message: `ожидался ровно один $effect ленивого догруза (places_subtree_stats), найдено ${lazy.length}`,
      hint: 'Догруз статистики переструктурировали — обнови гейт вместе с рефакторингом осознанно.',
    });
  } else if (!/statsCache\[[A-Za-z0-9_]+\]\s*!==\s*undefined/.test(lazy[0])) {
    violations.push({
      file: TREE,
      inv: 'INV-5',
      message: 'в ленивом $effect догруза статистики пропал гейт `statsCache[id] !== undefined`',
      hint:
        'Этот эффект тоже читает и пишет один и тот же $state и сходится ТОЛЬКО благодаря гейту: запись добавляет ' +
        'ключ, которого раньше не было, поэтому повторный проход его пропускает. Без гейта — тот же бесконечный ' +
        'цикл effect_update_depth_exceeded, что и в UAT3-03a.',
    });
  }
}

function checkStore(storeSrc, violations) {
  const code = stripComments(storeSrc);
  const body = functionBody(code, 'notifyPlaceContentChanged');
  if (body === null) {
    violations.push({
      file: STORE,
      inv: 'INV-6',
      message: 'функция notifyPlaceContentChanged не найдена',
      hint: 'Продюсер событий переименован/удалён — обнови гейт вместе с рефакторингом осознанно, а не удаляй проверку.',
    });
    return;
  }
  if (!/\bseq\s*(\+=\s*1|\+\+)/.test(body)) {
    violations.push({
      file: STORE,
      inv: 'INV-6',
      message: 'notifyPlaceContentChanged не увеличивает seq',
      hint:
        'Подписчик в PlaceTree просыпается именно на изменение seq (и отсекает начальное `seq === 0`). Без инкремента ' +
        'повторное событие с тем же списком placeIds не вызовет инвалидацию вовсе.',
    });
  }
  if (!/\bplaceIds\s*=(?!=)/.test(body)) {
    violations.push({
      file: STORE,
      inv: 'INV-6',
      message: 'notifyPlaceContentChanged не записывает placeIds',
      hint: 'Подписчик читает placeIds, чтобы понять, какие узлы (и их предков) вычищать.',
    });
  }
}

/**
 * INV-7 — продюсеры `notifyPlaceContentChanged` покрывают все клиентские
 * write-site смены места (WARNING-1, аудит 2026-09-17; расширено WR-01 gap
 * closure 2026-09-18). Стратегия проверки различается по файлу, намеренно —
 * это НЕ недосмотр/асимметрия по забывчивости:
 *
 *   1. `PlaceContents.svelte` содержит ДВА разных вызова `notifyPlaceContentChanged`
 *      (bulk-move в `handleMoveConfirm` и правка через `<PlaceEntityViewModal ...>`)
 *      — файл-уровневая проверка не отличила бы удаление одного от присутствия
 *      другого. Проверка ЛОКАЛИЗОВАНА на блок разметки тега
 *      `<PlaceEntityViewModal ...>` через `tagBlock`.
 *   2. `CartridgesPage.svelte` СТАЛА такой же после WR-01 gap closure: она
 *      теперь тоже содержит ДВА разных вызова — форма «Редактировать»
 *      (`handleFormSuccess`) и lifecycle-операции (`handleOperationSuccess`,
 *      install/return_to_stock/to_refill/from_refill/write_off). Файл-уровневая
 *      проверка (как раньше) больше не различила бы удаление вызова из
 *      `handleOperationSuccess`, пока `handleFormSuccess`'s вызов ещё цел —
 *      ровно класс дефекта mutation_test_anchor_must_be_unique. Проверка
 *      ЛОКАЛИЗОВАНА на ТЕЛО каждой функции через `functionBody`.
 *   3. `DevicesPage.svelte` содержит РОВНО ОДИН вызов (введён планом 40.1-03;
 *      до него — ноль вхождений в файле), так что файл-уровневая проверка
 *      (`indicesOf(code, NOTIFY).length === 0`) однозначно указывает на
 *      конкретный write-site — локализация до блока здесь не нужна.
 */
function checkProducers(contentsSrc, cartridgesSrc, devicesSrc, violations) {
  // Write-site 1 — правка предмета через PlaceEntityViewModal (PlaceContents.svelte).
  const contentsCode = stripComments(contentsSrc);
  const viewModalTag = tagBlock(contentsCode, 'PlaceEntityViewModal');
  if (viewModalTag === null) {
    violations.push({
      file: CONTENTS,
      inv: 'INV-7',
      message: 'тег <PlaceEntityViewModal ...> не найден',
      hint:
        'Компонент удалён/переименован/перенесён — обнови гейт вместе с рефакторингом осознанно, ' +
        'не удаляй проверку write-site 1 (правка предмета со страницы «Места», WARNING-1).',
    });
  } else if (!viewModalTag.includes(NOTIFY)) {
    violations.push({
      file: CONTENTS,
      inv: 'INV-7',
      message:
        'блок <PlaceEntityViewModal ...> не вызывает notifyPlaceContentChanged( — правка предмета ' +
        'со страницы «Места» больше не инвалидирует дерево',
      hint:
        'WR-01 (аудит 2026-08-27) уже закрывался «наполовину» — только массовый перенос ' +
        '(handleMoveConfirm) остаётся нетронутым, но этого НЕ достаточно: правка одного предмета ' +
        'через «Просмотр» → «Редактировать» тоже обязана звать notifyPlaceContentChanged, иначе ' +
        'счётчики дерева останутся устаревшими до перезагрузки страницы.',
    });
  }

  // Write-site 2 и 3 — CartridgesPage.svelte содержит ДВА разных write-site,
  // каждый проверяется отдельно по телу своей функции (WR-01 gap closure,
  // 40.1-audit-gap-closure, 2026-09-18).
  const cartridgesCode = stripComments(cartridgesSrc);
  for (const fnName of ['handleFormSuccess', 'handleOperationSuccess']) {
    const body = functionBody(cartridgesCode, fnName);
    if (body === null) {
      violations.push({
        file: CARTRIDGES_PAGE,
        inv: 'INV-7',
        message: `функция ${fnName} не найдена`,
        hint:
          'Переименована/удалена — обнови гейт вместе с рефакторингом осознанно, не удаляй ' +
          `проверку write-site (${fnName === 'handleFormSuccess' ? 'форма «Редактировать»' : 'lifecycle-операции install/return_to_stock/to_refill/from_refill/write_off'}, WARNING-1).`,
      });
    } else if (!body.includes(NOTIFY)) {
      violations.push({
        file: CARTRIDGES_PAGE,
        inv: 'INV-7',
        message: `${fnName} не вызывает notifyPlaceContentChanged(`,
        hint:
          fnName === 'handleOperationSuccess'
            ? 'WR-01 (аудит 40.1, 2026-09-18): lifecycle-операции (установка в принтер, возврат на ' +
              'склад, отправка/приём из заправки, списание) меняют place_id картриджа на сервере не ' +
              'реже (и чаще), чем форма «Редактировать» — без этого вызова счётчики дерева «Места» ' +
              'останутся устаревшими до перезагрузки страницы именно после самой частой операции.'
            : 'Правка места картриджа из формы «Редактировать» обязана инвалидировать счётчики ' +
              'дерева «Места», передавая и старое, и новое место (D-15), иначе счётчик ' +
              'места-источника останется завышенным.',
      });
    }
  }

  // Write-site 4 — DevicesPage.svelte (файл-уровневая проверка, см. doc-
  // комментарий функции выше: в файле ровно один вызов).
  const devicesCode = stripComments(devicesSrc);
  if (indicesOf(devicesCode, NOTIFY).length === 0) {
    violations.push({
      file: DEVICES_PAGE,
      inv: 'INV-7',
      message: 'notifyPlaceContentChanged( не вызывается нигде в файле',
      hint:
        'D-14 (аудит 2026-09-17) называет оба списка дословно — «Устройства»/«Картриджи»: правка ' +
        'места предмета из этого списка обязана инвалидировать счётчики дерева «Места», передавая ' +
        'и старое, и новое место (D-15), иначе счётчик места-источника останется завышенным.',
    });
  }
}

// ---------------------------------------------------------------------------

function main() {
  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(`${TAG} Usage: node scripts/check-place-tree-invalidation.mjs [--src=<ui-dir>]`);
    process.exit(0);
  }

  const read = (rel) => {
    try {
      return fs.readFileSync(path.join(SRC_ROOT, rel), 'utf8');
    } catch {
      console.error(
        `${TAG} FAIL — не удалось прочитать ${rel}. Файл переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
      );
      process.exit(1);
    }
  };

  const treeSrc = read(TREE);
  const storeSrc = read(STORE);
  const contentsSrc = read(CONTENTS);
  const cartridgesSrc = read(CARTRIDGES_PAGE);
  const devicesSrc = read(DEVICES_PAGE);

  const violations = [];
  checkTree(treeSrc, violations);
  checkStore(storeSrc, violations);
  checkProducers(contentsSrc, cartridgesSrc, devicesSrc, violations);

  for (const v of violations) {
    console.error(`${TAG} ${v.file} — ${v.inv}: ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений инвариантов инвалидации счётчиков. ` +
        'Этот класс дефекта уже вешал страницу целиком (UAT3-03a, коммит 5042e674) и не виден ни svelte-check, ни ' +
        'eslint, ни сборке — не «чинить» гейт, а вернуть инвариант.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
