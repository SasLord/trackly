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
//   INV-9 (41.11, AR-24) — АРГУМЕНТ каждого вызова `notifyPlaceContentChanged(...)`
//           не пустой литерал и содержит идентификатор. Пустой список в
//           `placeContentEvents.svelte.ts` — тихий no-op (`if (placeIds.length
//           === 0) return`), поэтому мутант `notifyPlaceContentChanged([])`
//           формально «инвалидирует», фактически ничего, и INV-7 (он смотрит
//           только на НАЛИЧИЕ вызова в обработчике) его пропускал зелёным.
//           Проверяемая формулировка (D-24a): «не пустой литерал И есть
//           идентификатор» — буквальное «аргумент — параметр обработчика»
//           статически непроверяемо (нужен разбор областей видимости) и
//           отвергнуто. Литерал ЛЮБОГО вида (`[1, 2]`) тоже красный: id мест
//           не бывают константой. Защита от вакуумности: найденных вызовов
//           обязано быть не меньше `MIN_NOTIFY_CALLS`.
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
//   node scripts/check-place-tree-invalidation.mjs --src=<dir>  # проверить копию (<dir>/src/...)
//   node scripts/check-place-tree-invalidation.mjs --selftest   # мутанты по РЕАЛЬНЫМ файлам (в памяти)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-place-tree-invalidation]';

const TREE = 'src/features/places/PlaceTree.svelte';
const STORE = 'src/lib/stores/placeContentEvents.svelte.ts';
// INV-7 (registry-driven, 40.1 exhaustive sweep) scans this whole subtree —
// see `checkCompleteness` below — instead of a fixed file list.
const FEATURES_DIR = 'src/features';
const NOTIFY = 'notifyPlaceContentChanged(';
// INV-9: сегодня в `src/features` 25 вызовов; порог с запасом. Падение ниже
// порога — нарушение гейта, а не молчаливое «0 нарушений» (вакуумный PASS,
// если вызовы разъехались мимо обхода).
const MIN_NOTIFY_CALLS = 20;

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

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
 * INV-7 (41.12, INT-15): включена ли граница идентификатора слева у литеральных
 * маркеров. Переключатель существует ТОЛЬКО ради selftest: кейсы границы
 * «красный-первым» выключают её и обязаны увидеть ложное срабатывание — иначе
 * фикстуры `foo.devices.delete(` / `mydevices.delete(` вакуумны и не доказывают,
 * что правило границы держит чистоту. В рабочем запуске всегда `true`.
 */
let boundaryEnabled = true;

/**
 * Индексы вхождений маркера с границей идентификатора слева (41.12).
 *
 * Литеральный маркер `devices.delete(` — подстрока `mydevices.delete(` и
 * `foo.devices.delete(`; без границы они дали бы ложное INV-7. Граница
 * применяется ТОЛЬКО к маркерам, начинающимся с символа идентификатора
 * (`[\w$]`): вхождение пропускается, если предыдущий символ — `[\w$.]`.
 * Маркеры, начинающиеся с кавычки (`'places_move_subtree_contents'`), и
 * module-маркеры (`${local}.${method}(`, локальное имя резолвится из импорта)
 * границей не затрагиваются.
 */
function markerIndices(code, marker) {
  const all = indicesOf(code, marker);
  if (!boundaryEnabled || !/^[\w$]/.test(marker)) return all;
  return all.filter((i) => i === 0 || !/[\w$.]/.test(code[i - 1]));
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
 *     звать `notifyPlaceContentChanged(`. Поля записи (41.12):
 *       `minSites`    — сколько мест маркера обязан найти скан по всем файлам;
 *                       меньше — «запись реестра устарела» (маркер переименован
 *                       или переехал в SKIP-able файл, реестр молча не охраняет).
 *       `notifyAfter` — вызов `notifyPlaceContentChanged(` обязан стоять ПОСЛЕ
 *                       маркера в теле функции (счётный гейт слеп к порядку).
 *                       Только у `devices.delete(` / `cartridges.delete(`.
 *     Литеральный маркер, начинающийся с символа идентификатора, ищется с
 *     границей слева (`markerIndices`): `mydevices.delete(` и
 *     `foo.devices.delete(` его не срабатывают.
 *
 * ---- Полный перечень write-site на момент этого коммита (13) ----
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
 *  11. DeviceRowMenu.svelte (прежде DeviceContextMenu.svelte, удалён в
 *      41.1) — `<PlaceEntityViewModal onChanged={handleViewChanged}>`
 *      (правка через «Просмотр» из меню строки списка «Устройства»,
 *      WR-08 — тоже найден ЭТИМ гейтом, не раундами верификации;
 *      `handleViewChanged` буквально отбрасывал аргумент).
 *  12. PlaceContents.svelte — `handleMoveConfirm` (массовый перенос,
 *      DIRECT_CALL_MARKERS, покрыт с Phase 40).
 *  13. GroupFormModal.svelte — `handleSubmit` (`groups.create`, создание
 *      группы от устройства, DIRECT_CALL_MARKERS, фаза 41.1 план 17).
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
 *   4. (закрыт INV-9 в части «пустой/литеральный аргумент») Гейт
 *      СТРУКТУРНЫЙ/текстовый, как и весь остальной файл (см. заголовок).
 *      Раньше он не отличал `notifyPlaceContentChanged([])` от настоящего
 *      вызова; теперь INV-9 краснит пустой и литеральный аргумент. НЕ закрыто
 *      и закрыто быть не может: что аргумент содержит ПРАВИЛЬНЫЕ
 *      старое/новое место (`changedPlaceIds.slice(0, 0)` INV-9 пройдёт). Это
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
    minSites: 1,
  },
  { marker: 'acts.delete(', label: 'мягкое удаление/undo акта (acts.delete)', minSites: 1 },
  // 41.12 (INT-15 аудита v1.4), перечень от серверных мутаций: device delete_soft и
  // cartridge delete рассылают place_ids, а клиентский вызов `notifyPlaceContentChanged`
  // — единственный путь обновления счётчиков при оборванном сокете. Маркеры литеральные,
  // а не module-маркеры: `devices` импортируется из нескольких путей. `notifyAfter` —
  // позиционное правило: вызов инвалидации обязан стоять ПОСЛЕ мутации (счётный гейт
  // слеп к порядку: вызов до `await` проходил бы зелёным и инвалидировал бы до успеха).
  // Только эти две записи: у старых записей порядок осознанно не проверяется (см.
  // задокументированное исключение у `groups.addDevices` в GroupContentsTable.svelte).
  {
    marker: 'devices.delete(',
    label: 'удаление устройства (devices.delete)',
    minSites: 1,
    notifyAfter: true,
  },
  {
    marker: 'cartridges.delete(',
    label: 'удаление картриджа (cartridges.delete)',
    minSites: 1,
    notifyAfter: true,
  },
  // Phase 41 (группы): у группы с местом устройства живут в месте группы, поэтому
  // перенос группы, добавление устройств в состав и вложение группы меняют
  // place_id у членов. Перечень взят от серверных мутаций (groups_move,
  // groups_add_devices, groups_set_parent — все отдают changed_place_ids), а не от
  // экранов: groups.move( появится в странице раздела (план 41-23).
  // Маркеры групп объявлены как `module`+`exportName`+`method`, а НЕ литеральной
  // подстрокой `groups.setParent(`. Причина — ревью фазы 41 (W-F01): `GroupsPage.svelte`
  // импортирует `{ groups as groupsApi }`, литеральная подстрока такой вызов не видит, и
  // удаление notifyPlaceContentChanged из `removeFromParent()` давало PASS. Локальное имя
  // биндинга резолвится из импорта в КАЖДОМ файле, поэтому ловятся и алиас, и прямое имя.
  {
    module: '$lib/api/groups',
    exportName: 'groups',
    method: 'move',
    minSites: 1,
    label: 'перенос группы (groups.move)',
  },
  {
    module: '$lib/api/groups',
    exportName: 'groups',
    method: 'addDevices',
    minSites: 4,
    label: 'добавление в состав (groups.addDevices)',
  },
  {
    module: '$lib/api/groups',
    exportName: 'groups',
    method: 'setParent',
    minSites: 3,
    label: 'вложение группы (groups.setParent)',
  },
  // Фаза 41.1 (план 17), перечень от серверных мутаций, W7: создание группы ОТ УСТРОЙСТВА
  // (быстрое действие в меню строки) берёт место устройства и рассылает place_ids, а для
  // teardown ещё и меняет статус устройства. Без маркера гейт «перечисляй от серверных
  // мутаций, а не от экранов» (урок 40.1: три раунда верификации по одному экрану) эту
  // мутацию не видел бы. Единственный вызов лежит в GroupFormModal.handleSubmit.
  {
    module: '$lib/api/groups',
    exportName: 'groups',
    method: 'create',
    minSites: 1,
    label:
      'создание группы (groups.create; от устройства рассылает place_ids и меняет статус для teardown)',
  },
];

/**
 * Локальное имя, под которым `exportName` из `modulePath` виден в этом файле.
 *
 * Возвращает `null`, когда модуль не импортирован (значит вызова тут быть не может)
 * или импортирован без нужного экспорта. Поддерживает `{ x }`, `{ x as y }`,
 * многострочный список и одинарные/двойные кавычки в пути.
 */
function resolveLocalName(code, modulePath, exportName) {
  const quoted = modulePath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const importRe = new RegExp(`import\\s*\\{([^}]*)\\}\\s*from\\s*['"]${quoted}['"]`, 'g');
  let m;
  while ((m = importRe.exec(code)) !== null) {
    for (const rawSpec of m[1].split(',')) {
      const spec = rawSpec.trim();
      if (!spec) continue;
      const asMatch = spec.match(/^([A-Za-z_$][\w$]*)\s+as\s+([A-Za-z_$][\w$]*)$/);
      if (asMatch) {
        if (asMatch[1] === exportName) return asMatch[2];
        continue;
      }
      if (spec === exportName) return exportName;
    }
  }
  return null;
}

function checkCompleteness(files, violations, read) {
  // 41.12: число найденных мест каждого маркера по ВСЕМ файлам — для `minSites`.
  const siteCounts = new Map();
  for (const relPath of files) {
    const baseName = path.basename(relPath, '.svelte');
    const raw = read(relPath);
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

    for (const entry of DIRECT_CALL_MARKERS) {
      const { label } = entry;
      let marker = entry.marker;
      if (!marker) {
        const local = resolveLocalName(code, entry.module, entry.exportName);
        // Модуль не импортирован в этом файле — вызова быть не может, идём дальше.
        if (local === null) continue;
        marker = `${local}.${entry.method}(`;
      }
      for (const idx of markerIndices(code, marker)) {
        siteCounts.set(label, (siteCounts.get(label) ?? 0) + 1);
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
        if (entry.notifyAfter) {
          // Позиционное правило: вызов инвалидации — после ПОСЛЕДНЕГО вхождения
          // маркера в теле (мутация уже состоялась). Перестановка выше `await`
          // оставляет число вызовов прежним и счётный гейт её не видит.
          const inBody = markerIndices(enclosing.body, marker);
          const lastMarker = inBody.length > 0 ? inBody[inBody.length - 1] : -1;
          if (lastMarker < 0 || enclosing.body.indexOf(NOTIFY, lastMarker + marker.length) < 0) {
            violations.push({
              file: relPath,
              inv: 'INV-7',
              message: `${enclosing.name}() зовёт ${label}, но notifyPlaceContentChanged( стоит не после мутации (до await или отсутствует)`,
              hint: 'Инвалидация счётчиков обязана идти ПОСЛЕ успешного await серверной мутации: вызов до неё сбрасывает счётчики, пока запись ещё не состоялась, и следующая подгрузка вернёт старое число.',
            });
          }
        } else if (!enclosing.body.includes(NOTIFY)) {
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

  // 41.12 (T-41.12-17): запись реестра, у которой не осталось мест, молча перестаёт
  // охранять — маркер переименован или вызов переехал в SKIP-able файл.
  for (const entry of DIRECT_CALL_MARKERS) {
    if (entry.minSites === undefined) continue;
    const found = siteCounts.get(entry.label) ?? 0;
    if (found < entry.minSites) {
      violations.push({
        file: '(реестр DIRECT_CALL_MARKERS)',
        inv: 'INV-7',
        message: `запись реестра устарела: ${entry.label} — найдено ${found} мест, ожидалось не меньше ${entry.minSites}`,
        hint: 'Вызов переехал или переименован, а запись осталась: маркер в SKIP-able файле молчит навсегда. Обнови реестр осознанно.',
      });
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
// INV-9 (41.11, AR-24) — аргумент notifyPlaceContentChanged не пустой литерал
// ---------------------------------------------------------------------------

/** Пустой литерал списка: `[]`, `[ ]`, `new Array()`, `Array()`, `Array.of()`. */
const EMPTY_LITERAL_RE = /^(?:\[\s*\]|(?:new\s+)?Array\s*\(\s*\)|Array\s*\.\s*of\s*\(\s*\))$/;

/** Служебные слова, не считающиеся «идентификатором данных». */
const NON_DATA_WORDS_RE = /\b(?:new|Array|Set|of|from|true|false|null|undefined|NaN|Infinity)\b/g;

/**
 * Что не так с текстом аргумента, либо `null`, если всё в порядке (D-24a):
 *   - пустой после trim;
 *   - пустой литерал;
 *   - после вырезания строковых/числовых литералов и служебных слов не осталось
 *     ни одного идентификатора (`[1, 2]`, `[null]`, `Array.from(new Set([]))`).
 */
function badNotifyArg(argText) {
  const arg = argText.trim();
  if (arg === '') return 'аргумент отсутствует';
  if (EMPTY_LITERAL_RE.test(arg)) return 'аргумент — пустой литерал списка';
  const stripped = arg
    .replace(/'(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|`(?:[^`\\]|\\.)*`/g, ' ')
    .replace(/\b\d[\w.]*/g, ' ')
    .replace(NON_DATA_WORDS_RE, ' ');
  if (!/[A-Za-z_$]/.test(stripped)) return 'аргумент — литерал без единого идентификатора';
  return null;
}

/** Номер строки (с 1) для индекса в тексте — комментарии затёрты пробелами с
 * сохранением переводов строк, поэтому номер совпадает с исходником. */
const lineOf = (src, idx) => src.slice(0, idx).split('\n').length;

/**
 * Возвращает число найденных вызовов (для защиты от вакуумности).
 */
function checkNotifyArgs(files, violations, read) {
  let calls = 0;
  for (const relPath of files) {
    const code = stripComments(read(relPath));
    for (const idx of indicesOf(code, NOTIFY)) {
      const parens = balancedParens(code, idx + NOTIFY.length - 1);
      calls += 1;
      if (parens === null) {
        violations.push({
          file: relPath,
          inv: 'INV-9',
          message: `строка ${lineOf(code, idx)}: не удалось разобрать скобки вызова notifyPlaceContentChanged(`,
          hint: 'Скобки вызова не сбалансированы до конца файла — аргумент не проверить.',
        });
        continue;
      }
      const reason = badNotifyArg(parens.text);
      if (reason !== null) {
        violations.push({
          file: relPath,
          inv: 'INV-9',
          message: `строка ${lineOf(code, idx)}: notifyPlaceContentChanged(${parens.text.trim()}) — ${reason}`,
          hint: 'Пустой список — тихий no-op в placeContentEvents (инвалидации нет, а вызов «есть»): передавай идентификаторы изменённых мест (changed_place_ids ответа сервера или id старого/нового места).',
        });
      }
    }
  }
  if (calls < MIN_NOTIFY_CALLS) {
    violations.push({
      file: FEATURES_DIR,
      inv: 'INV-9',
      message: `найдено ${calls} вызовов notifyPlaceContentChanged( — меньше порога ${MIN_NOTIFY_CALLS}`,
      hint: 'Либо вызовы пропали (инвалидация снята), либо обход перестал их видеть — «0 нарушений» было бы вакуумным. Если число вызовов законно упало, обнови MIN_NOTIFY_CALLS осознанно.',
    });
  }
  return calls;
}

// ---------------------------------------------------------------------------
// Сборка нарушений (чистая: без process.exit — вызывается и из --selftest)
// ---------------------------------------------------------------------------

class GateReadError extends Error {}

/** Относительные пути всех `.svelte` под `<root>/src/features`. */
function listFeatureFiles(root) {
  const featuresDir = path.join(root, FEATURES_DIR);
  try {
    return walkSvelteFiles(featuresDir).map((f) =>
      path.relative(root, f).split(path.sep).join('/'),
    );
  } catch {
    throw new GateReadError(
      `не удалось обойти ${FEATURES_DIR}. Каталог переехал/удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
    );
  }
}

/** `read(rel)` обязан бросать GateReadError, если файла нет. */
function collectViolations(read, files) {
  const violations = [];
  checkTree(read(TREE), violations);
  checkStore(read(STORE), violations);
  checkCompleteness(files, violations, read);
  checkCommitBeforeDismiss(read, violations);
  const notifyCalls = checkNotifyArgs(files, violations, read);
  return { violations, notifyCalls };
}

function diskReader(root) {
  return (rel) => {
    try {
      return fs.readFileSync(path.join(root, rel), 'utf8');
    } catch {
      throw new GateReadError(
        `не удалось прочитать ${rel}. Файл переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверку.`,
      );
    }
  };
}

// ---------------------------------------------------------------------------
// --selftest: мутанты по РЕАЛЬНЫМ файлам репозитория, в памяти (D-25, D-26, D-32)
// ---------------------------------------------------------------------------

/**
 * Заменяет якорь `from` на `to` в `source`. D-26: якорь обязан встречаться
 * РОВНО один раз — иначе мутант попадает в соседнюю функцию, а selftest
 * «доказывает» не то, что заявлено (или не мутирует ничего).
 */
function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL — якорь «${label}» встречается ${n} раз (нужно ровно 1)`);
    process.exit(1);
  }
  return source.replace(from, () => to);
}

function selftest() {
  const disk = diskReader(UI_ROOT);
  const files = listFeatureFiles(UI_ROOT);
  const run = (overrides, onlyFiles = files) => {
    const read = (rel) => (overrides.has(rel) ? overrides.get(rel) : disk(rel));
    return collectViolations(read, onlyFiles);
  };

  const ROW_MENU = 'src/features/devices/DeviceRowMenu.svelte';
  const ROW_MENU_ANCHOR =
    'if (changedPlaceIds.length > 0) notifyPlaceContentChanged(changedPlaceIds);';
  const withRowMenuCall = (arg, label) =>
    new Map([
      [
        ROW_MENU,
        mutate(
          disk(ROW_MENU),
          ROW_MENU_ANCHOR,
          `if (changedPlaceIds.length > 0) notifyPlaceContentChanged(${arg});`,
          `${ROW_MENU}: ${label}`,
        ),
      ],
    ]);
  const swap = (rel, from, to, label) =>
    new Map([[rel, mutate(disk(rel), from, to, `${rel}: ${label}`)]]);

  // --- 41.12 (INT-15): удаление устройства / картриджа ---
  // Все якоря берутся из РЕАЛЬНЫХ исходников и проходят через `mutate` (ровно одно
  // вхождение). Многострочные якоря повторяют отступы файла.
  const CART_PAGE = 'src/features/cartridges/CartridgesPage.svelte';
  const ACTS_PAGE = 'src/features/acts/ActsPage.svelte';
  const NOTIFY_DELETED =
    'if (deletedPlaceId !== null) notifyPlaceContentChanged([deletedPlaceId]);';
  const DEV_DELETE = 'await devices.delete(dev.id, dev.version);';
  const CART_DELETE =
    'await cartridges.delete(confirmDeleteCartridge.id, confirmDeleteCartridge.version);';
  const deleteSites = [
    { rel: ROW_MENU, del: DEV_DELETE, short: 'DeviceRowMenu', removed: 'devices.remove' },
    { rel: CART_PAGE, del: CART_DELETE, short: 'CartridgesPage', removed: 'cartridges.remove' },
  ];

  // 3e: синтетические файлы. Фикстуры содержат РЕАЛЬНУЮ подстроку маркера
  // (`pickedDevices.delete(` с заглавной D подстроки `devices.delete(` не содержит и
  // для проверки границы не годится).
  const SYNTH_DIR = 'src/features/zz_synthetic';
  const boundaryFixtures = [
    { file: `${SYNTH_DIR}/Pick1.svelte`, call: 'foo.devices.delete(id)', fn: 'pick1' },
    { file: `${SYNTH_DIR}/Pick2.svelte`, call: 'mydevices.delete(id)', fn: 'pick2' },
    { file: `${SYNTH_DIR}/Pick3.svelte`, call: 'foo.cartridges.delete(id)', fn: 'pick3' },
    { file: `${SYNTH_DIR}/Pick4.svelte`, call: 'mycartridges.delete(id)', fn: 'pick4' },
  ];
  const synthSource = (body) => `<script lang="ts">\n${body}\n</script>\n`;
  const withSynth = (rel, body) => run(new Map([[rel, synthSource(body)]]), [...files, rel]);
  /** Прогон с ВЫКЛЮЧЕННОЙ границей идентификатора (красный-первым для 3e). */
  const withoutBoundary = (fn) => {
    boundaryEnabled = false;
    try {
      return fn();
    } finally {
      boundaryEnabled = true;
    }
  };

  const deleteCases = [];
  for (const { rel, del, short, removed } of deleteSites) {
    const lineAfterDel = (indent) => `${del}\n${indent}${NOTIFY_DELETED}`;
    const lineBeforeDel = (indent) => `${NOTIFY_DELETED}\n${indent}${del}`;
    deleteCases.push(
      {
        name: `3a: ${short} — вызов инвалидации удалён`,
        run: () => run(swap(rel, NOTIFY_DELETED, 'void deletedPlaceId;', 'вызов снят')),
        expectInv: ['INV-7'],
        exact: 1,
      },
      {
        name: `3b: ${short} — удалён МАРКЕР (вызов переименован; ловит только minSites)`,
        run: () =>
          run(
            swap(
              rel,
              del,
              del.replace(/\b(devices|cartridges)\.delete\(/, `${removed}(`),
              'маркер переименован',
            ),
          ),
        expectInv: ['INV-7'],
        exact: 1,
      },
      {
        name: `3c: ${short} — перестановка (инвалидация выше мутации, число вызовов прежнее)`,
        run: () =>
          run(swap(rel, lineAfterDel('      '), lineBeforeDel('      '), 'перестановка вызова')),
        expectInv: ['INV-7'],
        exact: 1,
      },
      {
        name: `3d: ${short} — пустой аргумент notifyPlaceContentChanged([])`,
        run: () =>
          run(
            swap(
              rel,
              NOTIFY_DELETED,
              'if (deletedPlaceId !== null) notifyPlaceContentChanged([]);',
              'аргумент -> []',
            ),
          ),
        expectInv: ['INV-9'],
        exact: 1,
      },
    );
  }
  for (const { file, call, fn } of boundaryFixtures) {
    const body = `function ${fn}(id) { ${call}; }`;
    deleteCases.push(
      {
        name: `3e: граница включена — ${call} не даёт ложного срабатывания`,
        run: () => withSynth(file, body),
        clean: true,
      },
      {
        name: `3e: граница ВЫКЛЮЧЕНА — ${call} даёт INV-7 (фикстура не вакуумна)`,
        run: () => withoutBoundary(() => withSynth(file, body)),
        expectInv: ['INV-7'],
        exact: 1,
      },
    );
  }
  deleteCases.push(
    {
      name: '3f: новый потребитель devices.delete( без инвалидации',
      run: () =>
        withSynth(
          `${SYNTH_DIR}/NewConsumer.svelte`,
          'async function removeDevice(d) { await devices.delete(d.id, d.version); }',
        ),
      expectInv: ['INV-7'],
      exact: 1,
    },
    {
      name: '3g: ActsPage — acts.delete( переименован (minSites существующего маркера)',
      run: () =>
        run(
          swap(
            ACTS_PAGE,
            'const changedPlaceIds = await acts.delete(act.id, act.version);',
            'const changedPlaceIds = await acts.remove(act.id, act.version);',
            'маркер acts.delete переименован',
          ),
        ),
      expectInv: ['INV-7'],
      exact: 1,
    },
    {
      // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
      // Блочно-первая вычистка стирала бы всё от `templates/*.html` до ближайшего
      // `*/` ниже — вместе с нарушением, и гейт печатал бы PASS.
      name: '3h: `/*` в строчном комментарии не прячет devices.delete( от скана',
      run: () =>
        withSynth(
          `${SYNTH_DIR}/CommentTrap.svelte`,
          [
            '// шаблоны лежат в templates/*.html',
            'async function removeDevice(d) { await devices.delete(d.id, d.version); }',
            '/* настоящий блочный комментарий */',
          ].join('\n'),
        ),
      expectInv: ['INV-7'],
      exact: 1,
    },
  );

  // expectInv — какие правила ОБЯЗАНЫ покраснеть; exact — точное число нарушений
  // (если задано); clean — ожидается ноль нарушений.
  const cases = [
    {
      name: 'контроль: реальное дерево чисто',
      run: () => run(new Map()),
      clean: true,
    },
    // --- D-24 / D-24a ---
    {
      name: 'A: DeviceRowMenu — notifyPlaceContentChanged([])',
      run: () => run(withRowMenuCall('[]', 'аргумент -> []')),
      expectInv: ['INV-9'],
      exact: 1,
    },
    {
      name: 'B: DeviceRowMenu — литерал любого вида [1, 2]',
      run: () => run(withRowMenuCall('[1, 2]', 'аргумент -> [1, 2]')),
      expectInv: ['INV-9'],
      exact: 1,
    },
    {
      name: 'C: DeviceRowMenu — вызов без аргумента',
      run: () => run(withRowMenuCall('', 'аргумент -> пусто')),
      expectInv: ['INV-9'],
      exact: 1,
    },
    {
      name: 'D: DeviceRowMenu — new Array()',
      run: () => run(withRowMenuCall('new Array()', 'аргумент -> new Array()')),
      expectInv: ['INV-9'],
      exact: 1,
    },
    {
      name: 'E: вакуум — обход видит один файл вместо дерева (порог MIN_NOTIFY_CALLS)',
      run: () => run(new Map(), [ROW_MENU]),
      expectInv: ['INV-9'],
    },
    // --- формы, которые обязаны ПРОХОДИТЬ (правило не краснит всё подряд) ---
    {
      name: 'ok: result.changed_place_ids',
      run: () => run(withRowMenuCall('result.changed_place_ids', 'форма ok 1')),
      clean: true,
    },
    {
      name: 'ok: [place.id, moveTargetId]',
      run: () => run(withRowMenuCall('[place.id, moveTargetId]', 'форма ok 2')),
      clean: true,
    },
    {
      name: 'ok: Array.from(new Set([place.id, ...ids]))',
      run: () => run(withRowMenuCall('Array.from(new Set([place.id, ...ids]))', 'форма ok 3')),
      clean: true,
    },
    // --- прежние инварианты: selftest не охраняет только INV-9 ---
    {
      name: 'INV-1: PlaceTree — чтение statsCache вне untrack',
      run: () =>
        run(
          swap(
            'src/features/places/PlaceTree.svelte',
            'untrack(() => {',
            'Reflect.apply(() => {',
            'untrack снят',
          ),
        ),
      expectInv: ['INV-1'],
    },
    {
      name: 'INV-6: store — seq перестал увеличиваться',
      run: () =>
        run(
          swap(
            STORE,
            'placeContentEventsStore.seq += 1;',
            'placeContentEventsStore.seq = placeContentEventsStore.seq;',
            'seq += 1 снят',
          ),
        ),
      expectInv: ['INV-6'],
    },
    {
      name: 'INV-7: RequestDetail — обработчик не зовёт notifyPlaceContentChanged',
      run: () =>
        run(
          swap(
            'src/features/requests/RequestDetail.svelte',
            'if (changedPlaceIds.length > 0) notifyPlaceContentChanged(changedPlaceIds);',
            'void changedPlaceIds;',
            'вызов инвалидации снят',
          ),
        ),
      expectInv: ['INV-7'],
    },
    {
      name: 'INV-8: DeviceImportCsvModal — закрытие после коммита минует onImported',
      run: () =>
        run(
          swap(
            'src/features/devices/DeviceImportCsvModal.svelte',
            'onImported(report.affected_place_ids);',
            'onClose();',
            'onImported снят из handleClose',
          ),
        ),
      expectInv: ['INV-8'],
    },
    ...deleteCases,
  ];

  let failed = 0;
  for (const c of cases) {
    const { violations, notifyCalls } = c.run();
    const invs = new Set(violations.map((v) => v.inv));
    let ok;
    let why;
    if (c.clean) {
      ok = violations.length === 0 && notifyCalls >= MIN_NOTIFY_CALLS;
      why = `ожидалось 0 нарушений, получено ${violations.length} (${[...invs].join(',')}), вызовов ${notifyCalls}`;
    } else {
      const missing = c.expectInv.filter((i) => !invs.has(i));
      ok = missing.length === 0 && (c.exact === undefined || violations.length === c.exact);
      why = `ожидалось ${c.expectInv.join(',')}${c.exact !== undefined ? ` ×${c.exact}` : ''}, получено ${violations.length} (${[...invs].join(',') || '—'})`;
    }
    if (!ok) {
      failed += 1;
      console.error(`${TAG} SELFTEST FAIL — ${c.name}: ${why}`);
    }
  }

  if (failed > 0) {
    console.error(`${TAG} SELFTEST FAIL — ${failed} из ${cases.length} кейсов не прошли`);
    process.exit(1);
  }
  console.error(`${TAG} SELFTEST PASS — ${cases.length} кейсов (мутанты по реальным файлам)`);
  process.exit(0);
}

// ---------------------------------------------------------------------------

function main() {
  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-place-tree-invalidation.mjs [--src=<ui-dir>] [--selftest]`,
    );
    process.exit(0);
  }
  if (process.argv.includes('--selftest')) {
    try {
      selftest();
    } catch (e) {
      if (e instanceof GateReadError) {
        console.error(`${TAG} SELFTEST FAIL — ${e.message}`);
        process.exit(1);
      }
      throw e;
    }
    return;
  }

  const argSrc = process.argv.slice(2).find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  let result;
  try {
    result = collectViolations(diskReader(SRC_ROOT), listFeatureFiles(SRC_ROOT));
  } catch (e) {
    if (e instanceof GateReadError) {
      console.error(`${TAG} FAIL — ${e.message}`);
      process.exit(1);
    }
    throw e;
  }
  const { violations, notifyCalls } = result;

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

  console.error(
    `${TAG} PASS — 0 нарушений (INV-9: проверено ${notifyCalls} вызовов notifyPlaceContentChanged)`,
  );
  process.exit(0);
}

main();
