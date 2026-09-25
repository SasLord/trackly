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
//   INV-7 (40.1, аудит 2026-09-17, WARNING-1; REGISTRY-DRIVEN с 2026-09-18,
//           40.1-audit-gap-closure exhaustive sweep) — ПРОДЮСЕРЫ
//           `notifyPlaceContentChanged` покрывают ВСЕ клиентские write-site
//           смены места. Три раунда верификации подряд каждый нашёл ЕЩЁ ОДИН
//           непокрытый экран (WR-01 CartridgesPage, WR-05 RequestDetail,
//           затем целый класс «Акты» — ActsPage) — во всех трёх случаях
//           write-site тихо не звал функцию, и ничего в гейте это не ловило,
//           потому что гейт проверял фиксированный СПИСОК файлов/функций, а
//           не «какие вообще экраны меняют место». С этого коммита INV-7 —
//           РЕЕСТР place-mutating компонентов/прямых вызовов
//           (`MUTATING_COMPONENTS`/`FORWARDING_COMPONENTS`/
//           `DIRECT_CALL_MARKERS`, см. доккомментарий над `checkCompleteness`
//           ниже) + скан ВСЕГО дерева `ui/src/features` на каждое использование
//           зарегистрированного компонента/вызова — так что НОВЫЙ экран,
//           который рендерит уже известный компонент без инвалидации, ловится
//           автоматически, без ручного добавления в список. Полный перечень
//           write-site (12 на момент этого коммита, локализация на каждое
//           ИСПОЛЬЗОВАНИЕ отдельно — не на файл) и явные blind spot'ы этого
//           подхода — в доккомментарии над `checkCompleteness`.
//   INV-8 (CR-02, ревью 40.4) — компонент, который КОММИТИТ серверную мутацию
//           РАНЬШЕ, чем пользователь закрывает попап (шаг «результат»
//           CSV-импорта), не даёт закрыть себя в обход инвалидирующего пропа:
//           обработчик, который этот попап передаёт в `onClose` своего
//           `<Modal>`, обязан звать инвалидирующий проп (`onImported`). INV-7
//           этого класса дыр НЕ ВИДИТ — он проверяет только ПОТРЕБИТЕЛЯ
//           (`onImported={...}` в `DevicesPage`), а Escape/подложка/«×»
//           ходили мимо потребителя вовсе, и гейт выдавал «PASS — 0
//           нарушений» с открытой дырой. Реестр —
//           `COMMIT_BEFORE_DISMISS_COMPONENTS`.
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
// INV-7 (registry-driven, 40.1 exhaustive sweep) scans this whole subtree —
// see `checkCompleteness` below — instead of a fixed file list.
const FEATURES_DIR = 'src/features';
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
 * от явно заданного индекса начала (`<tagName`) до закрывающего `>` тега
 * (включая self-closing `/>`). Балансирует ФИГУРНЫЕ скобки, не круглые —
 * атрибуты Svelte-тега вида `onChanged={() => { ... }}` сами содержат
 * вложенные `{}`, и наивный поиск первого `>` попал бы внутрь `{() => {...}}`.
 * Возвращает `null`, если баланс не закрылся до конца файла.
 */
function tagBlockAt(src, start) {
  let depth = 0;
  for (let i = start; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') depth--;
    else if (src[i] === '>' && depth === 0) return src.slice(start, i + 1);
  }
  return null;
}

/**
 * INV-7 (registry-driven): EVERY `<tagName ...>`/`<tagName ... />` usage in
 * `src`, not just the first — a page can (and does, e.g. `ActsPage.svelte`'s
 * two `<ActFormModal>` usages) render the same component more than once.
 * Word-boundary safe via a lookahead on whitespace/`/`/`>` so `<Foo` does not
 * spuriously match `<FooBar`.
 */
function allTagBlocks(src, tagName) {
  const out = [];
  const re = new RegExp(`<${tagName}(?=[\\s/>])`, 'g');
  let m;
  while ((m = re.exec(src)) !== null) {
    const block = tagBlockAt(src, m.index);
    if (block !== null) out.push(block);
  }
  return out;
}

/**
 * INV-7 (registry-driven): extracts a Svelte attribute's raw value text from
 * a tag-block string. Handles both `name={value}` (balanced-brace extraction
 * — `value` can itself be an inline arrow function with nested `{}`) and the
 * Svelte shorthand `{name}` (value === name, used by e.g. `DevicesPage.svelte`'s
 * `{onSaved}`). Returns `null` if the attribute is absent from this usage.
 */
function attrValue(tagText, attrName) {
  const eqMatch = new RegExp(`(?:^|[\\s])${attrName}=\\{`).exec(tagText);
  if (eqMatch !== null) {
    const openIdx = eqMatch.index + eqMatch[0].length - 1;
    const braces = balancedBraces(tagText, openIdx);
    if (braces !== null) return braces.text.trim();
  }
  if (new RegExp(`(?:^|[\\s])\\{${attrName}\\}`).test(tagText)) return attrName;
  return null;
}

/** Balanced `{...}` content, mirrors `balancedParens` but for curly braces —
 * needed for `attrValue`'s `name={...}` extraction (the value itself may
 * contain nested `{}`, e.g. an inline arrow function body). */
function balancedBraces(src, openIdx) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === '{') depth++;
    else if (src[i] === '}') {
      depth--;
      if (depth === 0) return { text: src.slice(openIdx + 1, i), start: openIdx, end: i };
    }
  }
  return null;
}

/**
 * INV-7 (registry-driven): `const NAME = (...) => { ... }` fallback for
 * `functionBody` (below) — every handler in this codebase today is declared
 * `function NAME(...) {}`, but resolving the arrow-const form too means a
 * future handler written that way doesn't silently fall through to a
 * "could not resolve" violation for a benign reason.
 */
function constArrowBody(src, name) {
  const m = src.match(
    new RegExp(`const\\s+${name}\\s*=\\s*(?:async\\s*)?\\([^)]*\\)[^=]*=>\\s*\\{`),
  );
  if (!m) return null;
  const open = m.index + m[0].length - 1;
  const braces = balancedBraces(src, open);
  return braces === null ? null : braces.text;
}

/**
 * INV-7 (registry-driven): resolves a callback prop's VALUE TEXT (as
 * returned by `attrValue`) into a checkable body:
 *   - a bare identifier (`handleFoo`, incl. the shorthand's synthesized
 *     `attrName`) -> the same-file `function`/const-arrow declaration body.
 *   - an inline function/arrow expression (`(x) => {...}`) -> the expression
 *     text itself.
 * Returns `null` when resolution fails (member expression, imported/bound
 * handler, etc.) — callers MUST treat `null` as a VIOLATION, never a silent
 * skip (blind spot #2 in the doc-comment above `checkCompleteness`).
 */
function resolveCallbackBody(fileCode, valueText) {
  const trimmed = valueText.trim();
  if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(trimmed)) {
    const body = functionBody(fileCode, trimmed) ?? constArrowBody(fileCode, trimmed);
    return body === null ? null : { kind: 'named', name: trimmed, body };
  }
  if (/=>/.test(trimmed) || /^(?:async\s+)?function\b/.test(trimmed)) {
    return { kind: 'inline', name: null, body: trimmed };
  }
  return null;
}

/**
 * INV-7 (registry-driven): start index of the `{` innermost-enclosing `idx`,
 * scanning backward with brace-depth tracking — same technique as
 * `enclosingIfCondition` below, generalized to "any enclosing block" rather
 * than specifically an `if`. Returns `-1` if `idx` is at top level.
 */
function nearestEnclosingBraceStart(src, idx) {
  let depth = 0;
  for (let i = idx - 1; i >= 0; i--) {
    const c = src[i];
    if (c === '}') depth++;
    else if (c === '{') {
      if (depth === 0) return i;
      depth--;
    }
  }
  return -1;
}

/**
 * INV-7 (registry-driven, `DIRECT_CALL_MARKERS`): walks outward from `idx`
 * through enclosing `{...}` blocks (capped at 50 hops — generous, no
 * realistic file nests that deep) until it finds one immediately preceded by
 * a `function NAME(...)` header, then returns that function's FULL body via
 * `functionBody`. A marker matched at an arbitrary point inside a handler
 * must resolve to the ENCLOSING NAMED function, not just the nearest `{}`
 * (which could be an `if`/`.then()` callback/etc nested inside it).
 */
function enclosingFunction(src, idx) {
  let cursor = idx;
  for (let hop = 0; hop < 50; hop++) {
    const braceStart = nearestEnclosingBraceStart(src, cursor);
    if (braceStart < 0) return null;
    const header = src.slice(0, braceStart);
    const m = header.match(
      /(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*\([^)]*\)\s*(?::[^{]*)?\s*$/,
    );
    if (m) {
      const body = functionBody(src, m[1]);
      return body === null ? null : { name: m[1], body };
    }
    cursor = braceStart;
  }
  return null;
}

/**
 * INV-7 (registry-driven): every `.svelte` file under `dir`, recursively —
 * the scan surface for `checkCompleteness` (all of `ui/src/features`, not a
 * fixed file list).
 */
function walkSvelteFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walkSvelteFiles(full));
    else if (entry.isFile() && entry.name.endsWith('.svelte')) out.push(full);
  }
  return out;
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

/**
 * Тело функции по имени (учитывает вложенные `{}`).
 *
 * INV-7 fix (registry-driven sweep, 2026-09-18): the naive "first `{` after
 * the match" approach breaks when the parameter list itself contains an
 * inline TS object-type annotation with its OWN `{}` — e.g.
 * `function onSaved(result?: { typeId: number; placeId: number | null }) {`
 * (`DevicesPage.svelte`, `PlaceEntityViewModal.svelte`'s `handleDeviceEditSaved`)
 * — the old code would stop at the param-list's own `{`, slicing out the
 * TYPE ANNOTATION as if it were the function body and silently reporting
 * "no `notifyPlaceContentChanged(`" even though the real body (past the
 * closing `)`) has it. Fixed by first balancing the PARAMETER LIST's own
 * parens (`balancedParens`) and only then searching for the body's `{`
 * AFTER that closing `)` — skips any return-type annotation too.
 */
function functionBody(src, fnName) {
  const m = src.match(new RegExp(`(?:async\\s+)?function\\s+${fnName}\\s*\\(`));
  if (!m) return null;
  const parenOpen = m.index + m[0].length - 1;
  const params = balancedParens(src, parenOpen);
  if (params === null) return null;
  const open = src.indexOf('{', params.end + 1);
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
 * INV-7 (registry-driven, 40.1-audit-gap-closure exhaustive sweep,
 * 2026-09-18) — продюсеры `notifyPlaceContentChanged` покрывают ВСЕ
 * клиентские write-site смены места. Три раунда верификации подряд нашли
 * ЕЩЁ ОДИН непокрытый write-site (WR-01 CartridgesPage's lifecycle-операции,
 * WR-05 RequestDetail, затем целый класс «Акты» — ActsPage), и КАЖДЫЙ раз
 * root cause был один и тот же: гейт проверял фиксированный СПИСОК файлов/
 * функций, поэтому экран, которого не было в списке, был для гейта
 * невидим — не ложноотрицательный результат самой проверки, а дыра в том,
 * НА ЧТО она вообще смотрела.
 *
 * Эта секция заменяет список файлов РЕЕСТРОМ place-mutating «поверхностей»
 * и сканирует ВСЁ дерево `ui/src/features` на каждое использование
 * зарегистрированной поверхности — так НОВЫЙ экран, рендерящий уже известный
 * компонент без инвалидации, ловится автоматически, без ручного добавления
 * в список.
 *
 * ---- Реестр ----
 *   `MUTATING_COMPONENTS` — компоненты, чей success-колбэк стреляет DTO
 *     ПОСЛЕ серверной мутации места (OperationModal/CartridgeFormModal/
 *     DeviceFormModal/ActFormModal/ReturnModal). Резолвленный обработчик
 *     обязан звать `notifyPlaceContentChanged(` НАПРЯМУЮ — ЛИБО, если сам
 *     файл — это компонент из `FORWARDING_COMPONENTS` (см. ниже), может
 *     вместо этого форвардить через СВОЙ собственный проп (тот самый один
 *     хоп D-17's "consumer forwards up via callback").
 *   `FORWARDING_COMPONENTS` — компоненты, которые сами форвардят результат
 *     мутации через СВОЙ проп вместо прямого вызова (сегодня единственный:
 *     `PlaceEntityViewModal`'s `onChanged`). Каждое ИСПОЛЬЗОВАНИЕ такого
 *     компонента (в любом файле) обязано резолвить `forwardProp` в тело,
 *     которое ЗВОНИТ `notifyPlaceContentChanged(` — проверяется идентично
 *     `MUTATING_COMPONENTS`, просто под другим именем пропа.
 *   `DIRECT_CALL_MARKERS` — прямые (не через компонент) вызовы, меняющие
 *     place_id без обёртки `MUTATING_COMPONENTS` (bulk-move,
 *     `acts.delete`/soft-delete+undo). Функция, ОБРАМЛЯЮЩАЯ маркер, обязана
 *     звать `notifyPlaceContentChanged(`.
 *
 * ---- Полный перечень write-site на момент этого коммита (12) ----
 *   1. PlaceContents.svelte — `<PlaceEntityViewModal onChanged={...}>`
 *      (правка предмета через «Просмотр» со страницы «Места»).
 *   2. CartridgesPage.svelte — `handleFormSuccess` (форма «Редактировать»).
 *   3. CartridgesPage.svelte — `handleOperationSuccess` (lifecycle-операции,
 *      WR-01).
 *   4. DevicesPage.svelte — `onSaved` (создание/редактирование устройства).
 *   5. RequestDetail.svelte — `handleInstallSuccess` (установка картриджа
 *      через заявку, WR-05).
 *   6. ActsPage.svelte — `handleSaved` (создание акта, WR-06).
 *   7. ActsPage.svelte — `handleEditSaved` (редактирование акта, WR-06).
 *   8. ActsPage.svelte — `handleReturnSuccess` (создание/редактирование
 *      возврата, WR-06).
 *   9. ActsPage.svelte — `handleDelete` (мягкое удаление + undo-каскад,
 *      DIRECT_CALL_MARKERS, WR-06).
 *  10. PrinterDetail.svelte — `<DeviceFormModal onSaved={...}>` (правка
 *      «Данные устройства» со страницы принтера, WR-07 — найден ЭТИМ
 *      гейтом при построении реестра, ни одним из трёх раундов
 *      верификации).
 *  11. DeviceContextMenu.svelte — `<PlaceEntityViewModal onChanged=
 *      {handleViewChanged}>` (правка через «Просмотр» из кебаб-меню списка
 *      «Устройства», WR-08 — тоже найден ЭТИМ гейтом, не раундами
 *      верификации; `handleViewChanged` буквально отбрасывал аргумент).
 *  12. PlaceContents.svelte — `handleMoveConfirm` (массовый перенос,
 *      DIRECT_CALL_MARKERS, покрыт с Phase 40).
 *
 * ---- Что гейт ловит АВТОМАТИЧЕСКИ (без правки самого гейта) ----
 *   - Новый `.svelte`-файл под `ui/src/features`, рендерящий любой
 *     `MUTATING_COMPONENTS`/`FORWARDING_COMPONENTS` компонент с колбэком,
 *     не зовущим `notifyPlaceContentChanged(` (ни напрямую, ни — для
 *     forwarding-компонентов — через свой форвардящий проп).
 *   - Новое использование УЖЕ известного компонента в УЖЕ известном файле
 *     (второй `<ActFormModal>`, третий `<OperationModal>` и т.д.) —
 *     `allTagBlocks` находит КАЖДОЕ вхождение, не только первое.
 *   - Новый прямой вызов одного из `DIRECT_CALL_MARKERS` где угодно в
 *     дереве, чья обрамляющая именованная функция не зовёт notify.
 *
 * ---- Blind spot'ы (явно, без переоценки покрытия) ----
 *   1. Новый серверный мутирующий путь, НЕ заведённый через
 *      зарегистрированный компонент/маркер, невидим гейту, пока человек не
 *      добавит его в реестр — тот же класс пропуска, что и раньше, просто
 *      перенесённый на уровень выше: с «по UI-файлу» (меняется постоянно)
 *      на «по серверной мутирующей поверхности» (исчерпывающе
 *      перечислена в этом самом sweep'е, `40.1-03-SUMMARY.md`, и меняется
 *      НАМНОГО реже — новые экраны добавляются часто, новые
 *      place-мутирующие серверные точки — редко).
 *   2. Резолюция колбэка — ТОЛЬКО в пределах одного файла: обработчик,
 *      импортированный из другого модуля, bound-метод (`this.handleX`),
 *      member-expression (`obj.method`) или условно выбранный обработчик
 *      (`cond ? a : b`) не резолвятся — `resolveCallbackBody` вернёт
 *      `null`, и это ВСЕГДА нарушение (громкий fail), никогда молчаливый
 *      skip. Если это сработает на легитимном новом паттерне — расширяй
 *      `resolveCallbackBody`, не глуши проверку.
 *   3. Рекурсия `FORWARDING_COMPONENTS` захардкожена на ОДИН хоп (ровно тот
 *      единственный хоп, что реально существует в этой кодовой базе —
 *      `PlaceEntityViewModal` → его собственный потребитель). Второй
 *      уровень форвардинга потребует явного расширения этой секции, не
 *      пройдёт молча.
 *   4. Гейт СТРУКТУРНЫЙ/текстовый, как и весь остальной файл (см.
 *      заголовок) — доказывает, что ПРОВОДКА существует, не что аргументы
 *      `notifyPlaceContentChanged` — ПРАВИЛЬНЫЕ старое/новое место. Это
 *      по-прежнему требует живой проверки `cargo tauri dev` на каждый
 *      write-site, как и во всех раундах до этого.
 */

/** Компоненты, чей success-колбэк стреляет DTO после серверной мутации
 * места — резолвленный обработчик обязан звать `notifyPlaceContentChanged(`
 * напрямую (или форвардить, см. `FORWARDING_COMPONENTS`/`checkCompleteness`). */
const MUTATING_COMPONENTS = [
  { component: 'OperationModal', props: ['onSuccess'] },
  { component: 'CartridgeFormModal', props: ['onSuccess'] },
  { component: 'DeviceFormModal', props: ['onSaved'] },
  { component: 'ActFormModal', props: ['onSaved'] },
  { component: 'ReturnModal', props: ['onSuccess'] },
  { component: 'DeviceImportCsvModal', props: ['onImported'] },
];

/** Компоненты, форвардящие результат мутации через СВОЙ проп вместо
 * прямого вызова (D-17). Каждое ИХ использование где угодно тоже
 * проверяется — `forwardProp`'s handler обязан звать
 * `notifyPlaceContentChanged(`. */
const FORWARDING_COMPONENTS = [{ component: 'PlaceEntityViewModal', forwardProp: 'onChanged' }];

/** Прямые (не через компонент) маркеры вызова, меняющие place_id — их
 * ОБРАМЛЯЮЩАЯ именованная функция обязана звать
 * `notifyPlaceContentChanged(`. */
const DIRECT_CALL_MARKERS = [
  {
    marker: "'places_move_subtree_contents'",
    label: 'массовый перенос (places_move_subtree_contents)',
  },
  { marker: 'acts.delete(', label: 'мягкое удаление/undo акта (acts.delete)' },
];

function checkCompleteness(files, violations) {
  for (const filePath of files) {
    const relPath = path.relative(UI_ROOT, filePath).split(path.sep).join('/');
    const baseName = path.basename(filePath, '.svelte');
    const raw = fs.readFileSync(filePath, 'utf8');
    const code = stripComments(raw);
    // Grants "may forward instead of calling notifyPlaceContentChanged
    // directly" — ONLY inside the file that itself implements a
    // FORWARDING_COMPONENTS contract (e.g. PlaceEntityViewModal.svelte).
    const selfForward = FORWARDING_COMPONENTS.find((f) => f.component === baseName);

    const componentEntries = [
      ...MUTATING_COMPONENTS,
      ...FORWARDING_COMPONENTS.map((f) => ({ component: f.component, props: [f.forwardProp] })),
    ];

    for (const { component, props } of componentEntries) {
      for (const tag of allTagBlocks(code, component)) {
        const propName = props.find((p) => attrValue(tag, p) !== null);
        if (propName === undefined) {
          violations.push({
            file: relPath,
            inv: 'INV-7',
            message: `<${component} ...> usage не подключает ни один из [${props.join(', ')}] — невозможно проверить инвалидацию дерева мест`,
            hint: `Каждое использование <${component}> обязано подключить один из [${props.join(', ')}] к обработчику, инвалидирующему дерево мест — см. доккомментарий INV-7 выше.`,
          });
          continue;
        }
        const value = attrValue(tag, propName);
        const resolved = resolveCallbackBody(code, value);
        if (resolved === null) {
          violations.push({
            file: relPath,
            inv: 'INV-7',
            message: `<${component} ${propName}={${value}}> — не удалось резолвить тело обработчика (не inline-функция, не same-file объявление function/const-arrow)`,
            hint: 'Blind spot #2 в доккомментарии INV-7 выше: cross-file/bound/member-expression обработчики не резолвятся. Сделай обработчик inline, объяви его функцией в этом же файле, или расширь resolveCallbackBody в этом гейте.',
          });
          continue;
        }
        const callsNotify = resolved.body.includes(NOTIFY);
        // `?.()` optional-call syntax is common for prop-typed callbacks
        // (`onChanged?.(...)`, since the prop may be undefined) — a plain
        // `.includes(prop + '(')` substring check misses it (there's a `?.`
        // in between), so this is a small regex instead.
        const callsSelfForward =
          selfForward !== undefined &&
          new RegExp(`\\b${selfForward.forwardProp}\\s*(?:\\?\\.)?\\s*\\(`).test(resolved.body);
        if (!callsNotify && !callsSelfForward) {
          violations.push({
            file: relPath,
            inv: 'INV-7',
            message: `<${component} ${propName}={${value}}> — резолвленный обработчик не вызывает notifyPlaceContentChanged(${selfForward ? ` (и не форвардит через ${selfForward.forwardProp}()` : ''})`,
            hint: 'Success-колбэк этого компонента стреляет ПОСЛЕ серверной мутации места (см. реестр в доккомментарии выше) — обработчик обязан инвалидировать счётчики дерева мест (старое+новое место, дедуп, null отброшены), по образцу CartridgesPage.svelte handleOperationSuccess/handleFormSuccess.',
          });
        }
      }
    }

    for (const { marker, label } of DIRECT_CALL_MARKERS) {
      for (const idx of indicesOf(code, marker)) {
        const enclosing = enclosingFunction(code, idx);
        if (enclosing === null) {
          violations.push({
            file: relPath,
            inv: 'INV-7',
            message: `прямой вызов ${label} — не удалось резолвить обрамляющую именованную функцию`,
            hint: 'DIRECT_CALL_MARKERS требует, чтобы маркер лежал внутри `function NAME(...) { ... }` — inline top-level/module-scope вызовы этим гейтом не поддерживаются; оберни вызов в именованный обработчик.',
          });
          continue;
        }
        if (!enclosing.body.includes(NOTIFY)) {
          violations.push({
            file: relPath,
            inv: 'INV-7',
            message: `${enclosing.name}() зовёт ${label}, но не вызывает notifyPlaceContentChanged(`,
            hint: 'Это прямая серверная мутация place_id вне любой обёртки MUTATING_COMPONENTS — обрамляющий обработчик обязан сам инвалидировать счётчики дерева мест, по образцу PlaceContents.svelte handleMoveConfirm / ActsPage.svelte handleDelete.',
          });
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// INV-8 (CR-02, ревью 40.4) — «коммит раньше закрытия»
// ---------------------------------------------------------------------------

/**
 * Реестр попапов, которые коммитят серверную мутацию НЕ последним действием:
 * коммит происходит на промежуточном шаге, после него попап ещё живёт (экран
 * результата), и пользователь может закрыть его Escape / кликом по подложке /
 * «×» — минуя основную кнопку. Для таких попапов «успешный коммит» и
 * «закрытие» — РАЗНЫЕ события, поэтому инвалидация обязана висеть на
 * ЗАКРЫТИИ, а не на одной кнопке.
 *
 * Почему это отдельный инвариант, а не расширение INV-7: INV-7 смотрит на
 * ПОТРЕБИТЕЛЯ (`<DeviceImportCsvModal onImported={...}>` в `DevicesPage`) и
 * доказывает, что обработчик этого пропа инвалидирует дерево. Он ничего не
 * знает о том, ВСЕГДА ли попап до этого пропа доходит. В 40.4 `handleDone()`
 * (кнопка «Готово») звал `onImported`, а `handleClose()` (Escape/подложка/«×»,
 * живые на шаге 4 — то есть ПОСЛЕ `import_csv_commit`) звал только `onClose` —
 * и гейт выдавал «PASS — 0 нарушений» при устаревших счётчиках дерева,
 * устаревшем списке устройств и без тоста.
 *
 * Поля записи:
 *   `file`             — исходник самого попапа (не потребителя).
 *   `commitMarker`     — вызов, который коммитит мутацию. Если он исчез или
 *                        переименован, запись реестра устарела — падаем
 *                        громко, а не «проверяем» несуществующий путь.
 *   `commitStateVar`   — `$state`, по которому попап отличает «коммит
 *                        состоялся» от «ещё нет». Обработчик закрытия обязан
 *                        его читать: безусловный вызов инвалидирующего пропа
 *                        стрелял бы и при отмене на первом шаге.
 *   `invalidatingProp` — проп, ведущий к `notifyPlaceContentChanged` у
 *                        потребителя (его и проверяет INV-7).
 *   `dismissProp`      — проп `<Modal>`, через который приходят Escape,
 *                        подложка и «×» (см. `Modal.svelte`).
 *
 * Blind spot: гейт текстовый, как и весь остальной файл. Он доказывает, что
 * путь закрытия ВЕДЁТ к инвалидирующему пропу и что он смотрит на признак
 * коммита — не то, что условие расставлено верно. Живая проверка всех четырёх
 * способов закрытия по-прежнему обязательна.
 */
const COMMIT_BEFORE_DISMISS_COMPONENTS = [
  {
    file: 'src/features/devices/DeviceImportCsvModal.svelte',
    label: 'CSV-импорт устройств (N-3/40.4)',
    commitMarker: 'importCsvCommit(',
    commitStateVar: 'report',
    invalidatingProp: 'onImported',
    dismissProp: 'onClose',
  },
];

/** Вызов `name(...)` либо `name?.(...)` в тексте — проп-колбэки в этой
 * кодовой базе встречаются в обеих формах (ср. `callsSelfForward` выше). */
function callsCallback(body, name) {
  return new RegExp(`\\b${name}\\s*(?:\\?\\.)?\\s*\\(`).test(body);
}

function checkCommitBeforeDismiss(read, violations) {
  for (const entry of COMMIT_BEFORE_DISMISS_COMPONENTS) {
    const { file, label, commitMarker, commitStateVar, invalidatingProp, dismissProp } = entry;
    const code = stripComments(read(file));

    if (!code.includes(commitMarker)) {
      violations.push({
        file,
        inv: 'INV-8',
        message: `не найден коммит-вызов \`${commitMarker}\` — запись реестра COMMIT_BEFORE_DISMISS_COMPONENTS устарела`,
        hint: `Реестр INV-8 привязан к конкретному коммит-вызову (${label}). Если он переехал или переименован — обнови запись осознанно; молча «проверять» несуществующий путь гейт не должен.`,
      });
      continue;
    }

    const tags = allTagBlocks(code, 'Modal');
    if (tags.length === 0) {
      violations.push({
        file,
        inv: 'INV-8',
        message: 'не найдено ни одного `<Modal ...>` — не через что проверять путь закрытия',
        hint: 'Попап перестал рендерить <Modal> (или тег переименован) — пересмотри запись реестра INV-8 вместе с рефакторингом, не удаляй проверку.',
      });
      continue;
    }

    for (const tag of tags) {
      const value = attrValue(tag, dismissProp);
      if (value === null) {
        violations.push({
          file,
          inv: 'INV-8',
          message: `<Modal ...> не подключает \`${dismissProp}\` — Escape/подложка/«×» уходят в никуда`,
          hint: 'Modal.svelte зовёт этот проп на Escape, mousedown+mouseup по подложке и кнопке «×» — без него закрытие попапа не обрабатывается вовсе.',
        });
        continue;
      }
      const resolved = resolveCallbackBody(code, value);
      if (resolved === null) {
        violations.push({
          file,
          inv: 'INV-8',
          message: `<Modal ${dismissProp}={${value}}> — не удалось резолвить тело обработчика закрытия`,
          hint: 'Тот же blind spot #2, что и у INV-7: cross-file/bound/member-expression обработчики не резолвятся. Сделай обработчик inline или объяви его функцией в этом же файле.',
        });
        continue;
      }
      if (!callsCallback(resolved.body, invalidatingProp)) {
        violations.push({
          file,
          inv: 'INV-8',
          message: `<Modal ${dismissProp}={${value}}> — обработчик закрытия не зовёт \`${invalidatingProp}(\`: закрытие ПОСЛЕ коммита проходит мимо инвалидации дерева мест`,
          hint: `Это ровно дефект CR-02 (ревью 40.4): \`${commitMarker}\` уже записал строки в БД, а Escape/подложка/«×» закрывают попап без обновления списка и без notifyPlaceContentChanged. Закрытие обязано вести к \`${invalidatingProp}\`, когда коммит состоялся — см. handleClose в ${file}.`,
        });
        continue;
      }
      if (!new RegExp(`\\b${commitStateVar}\\b`).test(resolved.body)) {
        violations.push({
          file,
          inv: 'INV-8',
          message: `<Modal ${dismissProp}={${value}}> — обработчик закрытия зовёт \`${invalidatingProp}(\`, не глядя на \`${commitStateVar}\``,
          hint: `\`${commitStateVar}\` — признак состоявшегося коммита. Безусловный вызов ${invalidatingProp} стрелял бы и при отмене на первом шаге (ложный тост «Импорт завершён» и лишний refresh), поэтому ветка обязана быть условной.`,
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

  const violations = [];
  checkTree(treeSrc, violations);
  checkStore(storeSrc, violations);

  const featuresDir = path.join(SRC_ROOT, FEATURES_DIR);
  let files = [];
  try {
    files = walkSvelteFiles(featuresDir);
  } catch {
    console.error(
      `${TAG} FAIL — не удалось обойти ${FEATURES_DIR}. Каталог переехал/удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
    );
    process.exit(1);
  }
  checkCompleteness(files, violations);
  checkCommitBeforeDismiss(read, violations);

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
