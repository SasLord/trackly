#!/usr/bin/env node
// [check-request-flow] Гейт UI раунда 2 фазы 41.4 (D-35 / D-36 / D-37).
//
// Структурный: рантайм (выбор созданной заявки, переход по ссылке) он не доказывает —
// svelte-check и build его тоже не видят; живая проверка в UAT.
//
// Правила:
//  [A] RequestDetail: mergedIntoId() отдаёт relatedRequestId только для
//      'repair_merged_into'; в истории Button variant="link" внутри
//      {#if mergedIntoId(...)} зовёт openRequest(...); onMerged зовёт openRequest,
//      а не openRepairRequest напрямую.
//  [B] RequestDetail: .description-text :global(.field-value) { white-space: pre-wrap },
//      класс description-text стоит на обёртке DetailField label="Описание" (обе ветки).
//  [C] RequestFormModal: каждый onSuccess( с аргументом, в ветке repair — res.request.
//      RequestsPage: onSuccess формы присваивает selectedId из аргумента и не зовёт
//      handleStatusFilterChange; RequestDetail получает onOpenRequest, ставящий selectedId.
//  [D] RepairSendModal: receiverName предзаполняется из lastReceiverName.
//
// Usage:
//   node scripts/check-request-flow.mjs
//   node scripts/check-request-flow.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIR = path.resolve(__dirname, '../src/features/requests');
const TAG = '[check-request-flow]';

/** @returns {string[]} нарушения */
export function check({ detail, form, page, send }) {
  const bad = [];
  const d = stripComments(detail);
  const f = stripComments(form);
  const p = stripComments(page);
  const s = stripComments(send);

  // [A]
  const fn = /function mergedIntoId\([^)]*\)[^{]*\{([\s\S]*?)\n {2}\}/.exec(d);
  if (!fn) bad.push('[A] нет функции mergedIntoId');
  else if (!/'repair_merged_into'/.test(fn[1]) || !/relatedRequestId/.test(fn[1]))
    bad.push('[A] mergedIntoId не привязана к repair_merged_into и relatedRequestId');
  const blk = /\{#if mergedIntoId\([^)]*\)[^}]*\}([\s\S]*?)\{\/if\}/.exec(d);
  if (!blk) bad.push('[A] нет {#if mergedIntoId(...)} в истории');
  else if (!/<Button[^>]*variant="link"[^>]*onclick=\{\(\) => openRequest\(\w+\)\}/.test(blk[1]))
    bad.push('[A] ссылка в истории не Button variant="link" с openRequest(id)');
  const merged = /onMerged=\{\(targetId\) => \{([\s\S]*?)\}\}/.exec(d);
  if (!merged) bad.push('[A] нет onMerged');
  else if (!/openRequest\(targetId\)/.test(merged[1]) || /openRepairRequest/.test(merged[1]))
    bad.push('[A] onMerged должен звать openRequest(targetId), не openRepairRequest');
  if (!/function openRequest\(/.test(d) || !/onOpenRequest/.test(d))
    bad.push('[A] нет openRequest/onOpenRequest');
  if (/\{@html/.test(d)) bad.push('[A] {@html} запрещён');

  // [B]
  if (!/\.description-text :global\(\.field-value\) \{[^}]*white-space:\s*pre-wrap/.test(d))
    bad.push('[B] нет .description-text :global(.field-value) с white-space: pre-wrap');
  const wraps = d.match(
    /<div class="field-wide description-text">\s*<DetailField label="Описание"/g,
  );
  if ((wraps?.length ?? 0) !== 2)
    bad.push(`[B] description-text на обёртке «Описание»: ${wraps?.length ?? 0} из 2`);

  // [C]
  const calls = f.match(/onSuccess\([^)]*\)/g) ?? [];
  const real = calls.filter((c) => !/onSuccess: /.test(c));
  if (real.length < 2) bad.push('[C] RequestFormModal: меньше двух вызовов onSuccess(...)');
  if (real.some((c) => c === 'onSuccess()'))
    bad.push('[C] RequestFormModal: onSuccess() без аргумента');
  if (!/onSuccess\(res\.request\)/.test(f))
    bad.push('[C] ветка repair не передаёт onSuccess(res.request)');
  const h = /<RequestFormModal[\s\S]*?onSuccess=\{\((\w+)\) => \{([\s\S]*?)\n {2}\}\}\s*\/>/.exec(
    p,
  );
  if (!h) bad.push('[C] RequestsPage: нет onSuccess={(created) => …} у RequestFormModal');
  else {
    if (!new RegExp(`selectedId = ${h[1]}\\.id`).test(h[2]))
      bad.push('[C] RequestsPage: onSuccess не присваивает selectedId = created.id');
    if (/handleStatusFilterChange/.test(h[2]))
      bad.push('[C] RequestsPage: onSuccess зовёт handleStatusFilterChange (сбросит выбор)');
  }
  if (!/<RequestDetail[\s\S]*?onOpenRequest=\{\((\w+)\) => \(selectedId = \1\)\}/.test(p))
    bad.push('[C] RequestsPage: RequestDetail без onOpenRequest, ставящего selectedId');

  // [D]
  if (!/receiverName = last\.lastReceiverName/.test(s))
    bad.push('[D] RepairSendModal не предзаполняет receiverName из lastReceiverName');
  return bad;
}

function load() {
  const rd = (n) => fs.readFileSync(path.join(DIR, n), 'utf8');
  return {
    detail: rd('RequestDetail.svelte'),
    form: rd('RequestFormModal.svelte'),
    page: rd('RequestsPage.svelte'),
    send: rd('RepairSendModal.svelte'),
  };
}

function selftest() {
  const real = load();
  const count = (s, n) => s.split(n).length - 1;
  const mut = (key, find, repl) => {
    if (count(real[key], find) !== 1) {
      console.error(
        `${TAG} FAIL: selftest: якорь «${find}» не уникален (${count(real[key], find)})`,
      );
      process.exit(1);
    }
    return { ...real, [key]: real[key].replace(find, repl) };
  };
  const results = [['real', check(real).length === 0]];
  const mutants = {
    'onSuccess-no-arg': mut('form', 'onSuccess(res.request)', 'onSuccess()'),
    'link-no-merged-cond': mut('detail', '{#if mergedIntoId(row.entry) !== null}', '{#if true}'),
    'no-pre-wrap': mut('detail', 'white-space: pre-wrap;', ''),
    'onMerged-direct': mut(
      'detail',
      '      openRequest(targetId);',
      '      openRepairRequest(targetId);',
    ),
    'no-lastReceiverName': mut('send', "receiverName = last.lastReceiverName ?? '';", ''),
    'filter-change-resets': mut(
      'page',
      '      filter = { ...filter, status: null };',
      '      handleStatusFilterChange({ ...filter, status: null });',
    ),
    'no-selectedId': mut('page', '      selectedId = created.id;', ''),
    'no-onOpenRequest': mut('page', 'onOpenRequest={(id) => (selectedId = id)}', ''),
  };
  for (const [n, m] of Object.entries(mutants)) results.push([n, check(m).length > 0]);
  const failed = results.filter(([, ok]) => !ok).map(([n]) => n);
  if (failed.length > 0) {
    console.error(`${TAG} FAIL: selftest не отличает плохие фикстуры: ${failed.join(', ')}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const bad = check(load());
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}
