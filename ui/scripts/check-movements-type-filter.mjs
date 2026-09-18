#!/usr/bin/env node
// [check-movements-type-filter] Постоянный гейт против регрессии фильтра
// «Тип устройства» в отчёте «Перемещения» (HST-04, Phase 40.1, BLOCKER-2 аудита v1.4).
//
// Почему он существует: BLOCKER-2 уже один раз был ошибочно засчитан — бэкенд
// (`report_service.rs`, `query_movements_inner`) применял `filter.type_id`,
// `ReportsPage.svelte` передавал проп `typeId`, но `ReportFilters.svelte`
// принимал его как `typeId: _typeId` и контрол НИКОГДА не рендерил. Ни
// svelte-check, ни eslint, ни `pnpm build` такую «мёртвую проводку» не видят.
// Поэтому связка проверяется структурно по всей цепочке:
//
//   INV-1 — в ветке `{#if reportDomain === 'movements'}` ReportFilters.svelte
//           элемент с `id="report-type-filter"` — это кастомный `<Dropdown>`
//           (D-09, повторяющееся требование пользователя), а не нативный
//           `<select>` и не обёртка `<Select>`; Dropdown импортирован.
//   INV-2 — `onPickGroup` этого Dropdown вызывает `onFilterChange({ type_id })`
//           с отображением пустого id → `null` («Все типы», D-11) и иначе
//           `Number(id)`; список `groups` содержит опцию с `id: ''`.
//   INV-3 — ReportFilters.svelte реально читает пропы `typeId` и `deviceTypes`
//           (не алиасит их в `_typeId`/`_deviceTypes` — ровно форма исходного
//           дефекта BLOCKER-2).
//   INV-4 — ReportsPage.svelte передаёт в `<ReportFilters ...>`
//           `typeId={filter.type_id ...}` и `deviceTypes={...}`, а
//           `onFilterChange` вливает частичный фильтр в `filter`.
//   INV-5 — JS-зеркало `currentColumns()` (ReportsPage.svelte) и Rust
//           `columns_for`/`column_labels_for` (tauri_cmds/reports.rs) скрывают
//           ОДИН И ТОТ ЖЕ ключ колонки (и ту же подпись) для «Перемещений», оба
//           — под условием фильтра по типу: в JS `filter.type_id != null`, в
//           Rust `omit_type_column`, который в обоих экспорт-хелперах
//           (`build_reports_export_csv`/`build_reports_export_pdf`) вычисляется
//           как `report_type == "movements" && filter.type_id.is_some()`.
//           Иначе экран и CSV/PDF разойдутся молча (см. memory «JS-зеркало
//           Rust-формулы: нужен гейт-фикстура»).
//
// Гейт СТРУКТУРНЫЙ (по образцу check-report-type-parity.mjs): читает исходники
// и разбирает регулярками/скобочным балансом, НЕ рендерит компонент. Сужение
// строк самим бэкендом доказывают интеграционные тесты
// `crates/trackly-app/tests/report_movements.rs`
// (`report_movements_type_filter_narrows_rows`,
// `report_movements_place_and_type_filters_combine_with_and`).
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-movements-type-filter.mjs              # проверить репозиторий
//   node scripts/check-movements-type-filter.mjs --src=<dir>  # проверить копию (самотест)
//     <dir> — копия каталога ui/; Rust-файл ищется в <dir>/../crates/... (та же
//     раскладка, что в репозитории), либо явно через --rust=<file>.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-movements-type-filter]';

const REPORT_FILTERS = 'src/features/reports/ReportFilters.svelte';
const REPORTS_PAGE = 'src/features/reports/ReportsPage.svelte';
const REPORTS_RS = '../crates/trackly-app/src/tauri_cmds/reports.rs';

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/** Индекс парной закрывающей скобки для открывающей `open` (учёт вложенности). */
function matchingClose(src, openIdx, openCh = '{', closeCh = '}') {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === openCh) depth++;
    else if (src[i] === closeCh) {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/** Тело функции (JS `function name(` или Rust `fn name(`) по имени. */
function functionBody(src, fnName, keyword = 'function') {
  const m = src.match(new RegExp(`${keyword}\\s+${fnName}\\s*[(<]`));
  if (!m) return null;
  const open = src.indexOf('{', m.index + m[0].length);
  if (open < 0) return null;
  const close = matchingClose(src, open);
  return close < 0 ? null : src.slice(open + 1, close);
}

/**
 * Открывающий тег компонента/элемента, начинающийся в `start` (`<Name ...>`),
 * с учётом вложенных `{}` в выражениях атрибутов.
 */
function openingTagAt(src, start) {
  let depth = 0;
  for (let i = start; i < src.length; i++) {
    const c = src[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return src.slice(start, i + 1);
  }
  return null;
}

/** Все открывающие теги с заданным именем. */
function openingTags(src, tagName) {
  const out = [];
  const re = new RegExp(`<${tagName}[\\s>/]`, 'g');
  let m;
  while ((m = re.exec(src)) !== null) {
    const tag = openingTagAt(src, m.index);
    if (tag) out.push(tag);
  }
  return out;
}

/** Выражение атрибута `name={...}` внутри тега (с учётом вложенных `{}`). */
function attrExpression(tagSrc, name) {
  const m = tagSrc.match(new RegExp(`\\s${name}\\s*=\\s*\\{`));
  if (!m) return null;
  const open = m.index + m[0].length - 1;
  const close = matchingClose(tagSrc, open);
  return close < 0 ? null : tagSrc.slice(open + 1, close).trim();
}

/** Ветка `{#if reportDomain === 'movements'}` … `{:else}` / `{/if}` (без вложенных if). */
function movementsBranch(src) {
  const start = src.search(/\{#if\s+reportDomain\s*===\s*'movements'\s*\}/);
  if (start < 0) return null;
  let depth = 0;
  const re = /\{(#if|:else|\/if)\b[^}]*\}/g;
  re.lastIndex = start;
  let m;
  while ((m = re.exec(src)) !== null) {
    if (m[1] === '#if') depth++;
    else if (m[1] === '/if') {
      depth--;
      if (depth === 0) return src.slice(start, m.index);
    } else if (depth === 1) {
      return src.slice(start, m.index);
    }
  }
  return null;
}

/** Тег (любого имени), несущий `id="report-type-filter"`, внутри фрагмента. */
function tagWithTypeFilterId(fragment) {
  const re = /<([A-Za-z][\w.]*)[\s>]/g;
  let m;
  const found = [];
  while ((m = re.exec(fragment)) !== null) {
    const tag = openingTagAt(fragment, m.index);
    if (tag && /\sid\s*=\s*"report-type-filter"/.test(tag)) found.push({ name: m[1], tag });
  }
  return found;
}

// ---------------------------------------------------------------------------
// Проверки
// ---------------------------------------------------------------------------

function checkFilters(filtersSrc, violations) {
  const branch = movementsBranch(filtersSrc);
  if (branch === null) {
    violations.push(
      `INV-1: ветка {#if reportDomain === 'movements'} не найдена в ${REPORT_FILTERS}. ` +
        'Компонент переструктурирован — обнови гейт осознанно, а не удаляй проверку.',
    );
    return;
  }

  if (!/import\s+Dropdown\s+from\s+'\$lib\/components\/Dropdown\.svelte'/.test(filtersSrc)) {
    violations.push(
      `INV-1: ${REPORT_FILTERS} не импортирует кастомный Dropdown из $lib/components/Dropdown.svelte.`,
    );
  }

  const carriers = tagWithTypeFilterId(branch);
  if (carriers.length === 0) {
    violations.push(
      `INV-1 (регресс BLOCKER-2): в ветке movements ${REPORT_FILTERS} нет элемента с ` +
        'id="report-type-filter" — контрол «Тип устройства» не рендерится.',
    );
    return;
  }
  const nonDropdown = carriers.filter((c) => c.name !== 'Dropdown');
  if (nonDropdown.length > 0) {
    violations.push(
      `INV-1 (регресс D-09): id="report-type-filter" в ${REPORT_FILTERS} несёт ` +
        `<${nonDropdown.map((c) => c.name).join('>, <')}>, а должен — только кастомный <Dropdown> ` +
        '(не нативный <select>, не обёртка <Select>).',
    );
  }
  const dropdown = carriers.find((c) => c.name === 'Dropdown');
  if (!dropdown) return;

  // INV-2: onPickGroup → onFilterChange({ type_id: X === '' ? null : Number(X) })
  const pick = attrExpression(dropdown.tag, 'onPickGroup');
  if (pick === null) {
    violations.push(
      `INV-2: у <Dropdown id="report-type-filter"> в ${REPORT_FILTERS} нет onPickGroup={...} — ` +
        'выбор типа никуда не уходит.',
    );
  } else {
    const call = pick.match(
      /onFilterChange\s*\??\.?\s*\(\s*\{\s*type_id\s*:\s*([\s\S]+?)\s*,?\s*\}\s*\)/,
    );
    if (!call) {
      violations.push(
        `INV-2: onPickGroup в ${REPORT_FILTERS} не вызывает onFilterChange({ type_id: … }): ${pick}`,
      );
    } else {
      const v = call[1];
      const eqForm =
        /^([\w.]+)\s*===\s*''\s*\?\s*null\s*:\s*Number\(\s*([\w.]+)\s*\)$/.exec(v) ??
        /^([\w.]+)\s*!==\s*''\s*\?\s*Number\(\s*([\w.]+)\s*\)\s*:\s*null$/.exec(v);
      if (!eqForm || eqForm[1] !== eqForm[2]) {
        violations.push(
          `INV-2 (регресс D-11): type_id в onPickGroup ${REPORT_FILTERS} должен отображать пустой ` +
            `id «Все типы» в null и иначе Number(id) по одному и тому же значению; найдено: ${v}`,
        );
      }
    }
  }

  const groups = attrExpression(dropdown.tag, 'groups');
  const groupsName = groups && /^\w+$/.test(groups) ? groups : null;
  const groupsDef = groupsName
    ? filtersSrc.match(new RegExp(`const\\s+${groupsName}\\b[^=]*=\\s*[\\s\\S]*?\\]\\s*\\)?;`))
    : null;
  if (!groupsDef || !/\bid\s*:\s*''/.test(groupsDef[0])) {
    violations.push(
      `INV-2 (регресс D-11): список groups у <Dropdown id="report-type-filter"> в ${REPORT_FILTERS} ` +
        "не содержит опции с id: '' («Все типы») — фильтр нельзя сбросить.",
    );
  }

  // INV-3: typeId / deviceTypes читаются, а не алиасятся в _typeId/_deviceTypes.
  const destructure = filtersSrc.match(/const\s*\{([\s\S]*?)\}\s*:\s*Props\s*=\s*\$props\(\)/);
  if (!destructure) {
    violations.push(
      `INV-3: деструктуризация пропов (: Props = $props()) не найдена в ${REPORT_FILTERS}.`,
    );
  } else {
    for (const prop of ['typeId', 'deviceTypes']) {
      const aliased = new RegExp(`\\b${prop}\\s*:\\s*_`).test(destructure[1]);
      const present = new RegExp(`\\b${prop}\\b`).test(destructure[1]);
      if (!present || aliased) {
        violations.push(
          `INV-3 (регресс BLOCKER-2): проп ${prop} в ${REPORT_FILTERS} ` +
            (aliased ? `снова «принят и не используется» (${prop}: _…)` : 'не деструктурирован') +
            '.',
        );
      }
    }
  }
}

function checkPage(pageSrc, violations) {
  const tags = openingTags(pageSrc, 'ReportFilters');
  if (tags.length === 0) {
    violations.push(`INV-4: <ReportFilters ...> не найден в ${REPORTS_PAGE}.`);
    return;
  }
  for (const tag of tags) {
    const typeId = attrExpression(tag, 'typeId');
    if (typeId === null || !/^filter\.type_id\b/.test(typeId)) {
      violations.push(
        `INV-4: <ReportFilters> в ${REPORTS_PAGE} не передаёт typeId={filter.type_id ...} ` +
          `(найдено: ${typeId ?? 'нет атрибута'}).`,
      );
    }
    if (attrExpression(tag, 'deviceTypes') === null) {
      violations.push(`INV-4: <ReportFilters> в ${REPORTS_PAGE} не передаёт deviceTypes={...}.`);
    }
    const onChange = attrExpression(tag, 'onFilterChange');
    if (
      onChange === null ||
      !/filter\s*=\s*\{\s*\.\.\.filter\s*,\s*\.\.\.\w+\s*\}/.test(onChange)
    ) {
      violations.push(
        `INV-4: onFilterChange у <ReportFilters> в ${REPORTS_PAGE} не вливает частичный фильтр ` +
          `в filter (ожидалось filter = { ...filter, ...f }); найдено: ${onChange ?? 'нет атрибута'}.`,
      );
    }
  }
}

function checkColumnParity(pageSrc, rustSrc, violations) {
  // --- JS: currentColumns() → ветка movements → omit под filter.type_id ---
  const cc = functionBody(pageSrc, 'currentColumns');
  if (cc === null) {
    violations.push(`INV-5: функция currentColumns не найдена в ${REPORTS_PAGE}.`);
    return;
  }
  const mvIf = cc.match(/if\s*\(\s*activeDomain\s*===\s*'movements'\s*\)\s*\{/);
  const mvBody = mvIf
    ? cc.slice(mvIf.index + mvIf[0].length - 1, matchingClose(cc, mvIf.index + mvIf[0].length - 1))
    : null;
  const jsOmit = mvBody
    ? mvBody.match(
        /if\s*\(\s*filter\.type_id\s*!==?\s*null[^)]*\)\s*\{[^}]*?\.key\s*!==\s*'([^']+)'/,
      )
    : null;
  if (!jsOmit) {
    violations.push(
      `INV-5: в currentColumns() ${REPORTS_PAGE} (ветка movements) не найдено скрытие колонки ` +
        "под условием filter.type_id != null (… c.key !== '<ключ>').",
    );
  }

  // --- Rust: columns_for / column_labels_for ---
  const armOf = (body) => {
    if (body === null) return null;
    const i = body.indexOf('"movements" =>');
    if (i < 0) return null;
    const open = body.indexOf('{', i);
    const close = matchingClose(body, open);
    return close < 0 ? null : body.slice(open, close + 1);
  };
  const colsArm = armOf(functionBody(rustSrc, 'columns_for', 'fn'));
  const labelsArm = armOf(functionBody(rustSrc, 'column_labels_for', 'fn'));
  const retainRe = /if\s+omit_type_column\s*\{\s*\w+\.retain\(\|(\w+)\|\s*\*\1\s*!=\s*"([^"]+)"\)/;
  const rsCol = colsArm ? colsArm.match(retainRe) : null;
  const rsLabel = labelsArm ? labelsArm.match(retainRe) : null;
  if (!rsCol) {
    violations.push(
      `INV-5: в columns_for (${REPORTS_RS}) ветка "movements" не скрывает колонку под ` +
        'if omit_type_column { cols.retain(|c| *c != "<ключ>") }.',
    );
  }
  if (!rsLabel) {
    violations.push(
      `INV-5: в column_labels_for (${REPORTS_RS}) ветка "movements" не скрывает подпись под ` +
        'if omit_type_column { labels.retain(|l| *l != "<подпись>") }.',
    );
  }

  if (jsOmit && rsCol && jsOmit[1] !== rsCol[2]) {
    violations.push(
      `INV-5 (расхождение экрана и экспорта): currentColumns() в ${REPORTS_PAGE} скрывает ` +
        `'${jsOmit[1]}', а columns_for в ${REPORTS_RS} — "${rsCol[2]}".`,
    );
  }
  if (jsOmit && rsLabel) {
    const mapEntry = pageSrc.match(
      new RegExp(`\\{\\s*key:\\s*'${jsOmit[1]}'\\s*,\\s*label:\\s*'([^']+)'`),
    );
    if (!mapEntry || mapEntry[1] !== rsLabel[2]) {
      violations.push(
        `INV-5 (расхождение подписи): колонка '${jsOmit[1]}' в COLUMNS_MAP ${REPORTS_PAGE} ` +
          `подписана '${mapEntry ? mapEntry[1] : '?'}', а column_labels_for скрывает "${rsLabel[2]}".`,
      );
    }
  }
  // --- Rust: omit_type_column обусловлен filter.type_id в обоих экспорт-хелперах ---
  for (const fn of ['build_reports_export_csv', 'build_reports_export_pdf']) {
    const body = functionBody(rustSrc, fn, 'fn');
    if (body === null) {
      violations.push(`INV-5: функция ${fn} не найдена в ${REPORTS_RS}.`);
      continue;
    }
    const cond =
      /let\s+omit_type_column\s*=\s*report_type\s*==\s*"movements"\s*&&\s*filter\.type_id\.is_some\(\)\s*;/;
    if (!cond.test(body)) {
      violations.push(
        `INV-5: в ${fn} (${REPORTS_RS}) omit_type_column не вычисляется как ` +
          'report_type == "movements" && filter.type_id.is_some() — экспорт разойдётся с экраном.',
      );
    }
    for (const helper of ['columns_for', 'column_labels_for']) {
      if (
        !new RegExp(`\\b${helper}\\(\\s*&report_type\\s*,\\s*omit_type_column\\s*\\)`).test(body)
      ) {
        violations.push(
          `INV-5: ${fn} (${REPORTS_RS}) не передаёт omit_type_column в ${helper}(&report_type, …).`,
        );
      }
    }
  }
}

// ---------------------------------------------------------------------------

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-movements-type-filter.mjs [--src=<ui-dir>] [--rust=<reports.rs>]`,
    );
    process.exit(0);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const argRust = args.find((a) => a.startsWith('--rust='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const rustPath = argRust
    ? path.resolve(process.cwd(), argRust.slice('--rust='.length))
    : path.resolve(SRC_ROOT, REPORTS_RS);

  const read = (p, label) => {
    try {
      return fs.readFileSync(p, 'utf8');
    } catch {
      console.error(`${TAG} FAIL — не удалось прочитать ${label} (${p}).`);
      process.exit(1);
    }
  };
  const filtersSrc = read(path.join(SRC_ROOT, REPORT_FILTERS), REPORT_FILTERS);
  const pageSrc = read(path.join(SRC_ROOT, REPORTS_PAGE), REPORTS_PAGE);
  const rustSrc = read(rustPath, REPORTS_RS);

  const violations = [];
  checkFilters(filtersSrc, violations);
  checkPage(pageSrc, violations);
  checkColumnParity(pageSrc, rustSrc, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений фильтра «Тип устройства» отчёта «Перемещения».`,
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
