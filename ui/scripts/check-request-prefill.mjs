#!/usr/bin/env node
// [check-request-prefill] Гейт предзаполнения формы заявки (фаза 41.6, EMP-03 / D-12 / D-13).
//
// Структурный: рантайм (что поле реально заполнилось, что тост один) он не доказывает —
// svelte-check и build этого не видят; живая проверка в UAT.
//
// Правила (RequestFormModal.svelte):
//  [P1] в Props объявлен `prefill`.
//  [P2] ПОРЯДОК: в блоке открытия `if (isOpen && !_wasOpen) {` и сброс типа/принтера/
//       устройства, и ВСЕ применения prefill (`requestType = pf.type`, вызовы
//       applyRepairPrefill / applyPrinterPrefill) стоят ПОЗЖЕ сброса — по индексам, не по
//       счёту. Иначе сброс затирает предзаполнение.
//  [P3] обе ветки типа применяют prefill: блок зовёт оба apply*-помощника, а помощники
//       пишут `repairChoice = String(` и `printerDeviceId = deviceId`.
//  [P4] каждый успешный pushToast формы стоит под охраной `if (!prefill)` (D-13: в режиме
//       prefill единственный тост показывает страница).
//  [P5] onSuccess репайр-ветки несёт `joined`.
//  [P6] RequestsPage не передаёт `prefill=` (прежнее поведение без prefill).
//
// Usage:
//   node scripts/check-request-prefill.mjs
//   node scripts/check-request-prefill.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIR = path.resolve(__dirname, '../src/features/requests');
const TAG = '[check-request-prefill]';

/** @returns {string[]} нарушения */
export function check({ form, page }) {
  const bad = [];
  const f = stripComments(form);
  const p = stripComments(page);

  // [P1]
  const props = /interface Props \{([\s\S]*?)\n {2}\}/.exec(f);
  if (!props || !/\bprefill\?:/.test(props[1])) bad.push('[P1] в Props нет prefill?:');

  // [P2] + [P3]
  const ob = /if \(isOpen && !_wasOpen\) \{([\s\S]*?)\n {4}\}\n/.exec(f);
  if (!ob) {
    bad.push('[P2] нет блока if (isOpen && !_wasOpen)');
  } else {
    const b = ob[1];
    const resets = [
      b.indexOf("requestType = 'cartridge_replace';"),
      b.indexOf('printerDeviceId = null;'),
      b.indexOf('repairChoice = null;'),
    ];
    if (resets.some((i) => i < 0))
      bad.push('[P2] блок открытия не сбрасывает тип/принтер/устройство');
    const lastReset = Math.max(...resets);
    const applies = {
      'requestType = pf.type': b.search(/requestType = pf\.type/),
      'applyRepairPrefill(': b.indexOf('applyRepairPrefill('),
      'applyPrinterPrefill(': b.indexOf('applyPrinterPrefill('),
    };
    for (const [name, idx] of Object.entries(applies)) {
      if (idx < 0) bad.push(`[P3] блок открытия не содержит «${name}»`);
      else if (idx < lastReset)
        bad.push(`[P2] «${name}» стоит РАНЬШЕ сброса: сброс затрёт предзаполнение`);
    }
    if (!/=\s*prefill\b/.test(b)) bad.push('[P2] блок открытия не читает prefill');
  }
  const rep = /async function applyRepairPrefill\([^)]*\)[^{]*\{([\s\S]*?)\n {2}\}\n/.exec(f);
  if (!rep || !/repairChoice = String\(/.test(rep[1]))
    bad.push('[P3] applyRepairPrefill не пишет repairChoice = String(');
  const prn = /async function applyPrinterPrefill\([^)]*\)[^{]*\{([\s\S]*?)\n {2}\}\n/.exec(f);
  if (!prn || !/printerDeviceId = deviceId/.test(prn[1]))
    bad.push('[P3] applyPrinterPrefill не пишет printerDeviceId = deviceId');

  // [P4]
  const toasts = [...f.matchAll(/pushToast\(\s*'success'/g)];
  if (toasts.length < 2) bad.push(`[P4] успешных pushToast меньше двух (${toasts.length})`);
  for (const m of toasts) {
    const before = f.slice(Math.max(0, m.index - 60), m.index);
    if (!/if \(!prefill\)\s*\{?\s*$/.test(before))
      bad.push('[P4] успешный pushToast без охраны if (!prefill)');
  }

  // [P5]
  if (!/onSuccess\(\{[^})]*joined/.test(f)) bad.push('[P5] onSuccess не несёт joined');

  // [P6]
  if (/\bprefill\s*=/.test(p)) bad.push('[P6] RequestsPage передаёт prefill=');

  return bad;
}

function load() {
  const rd = (n) => fs.readFileSync(path.join(DIR, n), 'utf8');
  return { form: rd('RequestFormModal.svelte'), page: rd('RequestsPage.svelte') };
}

function selftest() {
  const real = load();
  const count = (s, n) => s.split(n).length - 1;
  const fail = (msg) => {
    console.error(`${TAG} FAIL: selftest: ${msg}`);
    process.exit(1);
  };
  const mut = (key, find, repl) => {
    if (count(real[key], find) !== 1)
      fail(`якорь «${find}» не уникален (${count(real[key], find)})`);
    return { ...real, [key]: real[key].replace(find, repl) };
  };
  // Перестановка (не удаление): блок prefill переносится ВЫШЕ сброса.
  const moved = (() => {
    const start = real.form.indexOf('      const pf = prefill;');
    const endMark = '        void focusDescription(pf.type);\n      }\n';
    const end = real.form.indexOf(endMark);
    if (start < 0 || end < 0 || count(real.form, '      const pf = prefill;') !== 1)
      fail('якоря блока prefill не найдены');
    const chunk = real.form.slice(start, end + endMark.length);
    const without = real.form.slice(0, start) + real.form.slice(end + endMark.length);
    const anchor = '      openInstanceCounter += 1;\n';
    if (count(without, anchor) !== 1) fail('якорь openInstanceCounter += 1 не уникален');
    // chunk использует printersDone, объявленную ниже — для структурного гейта это неважно.
    return { ...real, form: without.replace(anchor, anchor + chunk) };
  })();

  const results = [['real', check(real).length === 0]];
  const mutants = {
    'prefill-before-reset': moved,
    'no-cartridge-branch': mut(
      'form',
      'void applyPrinterPrefill(pf.deviceId, printersDone, instance);',
      '',
    ),
    'toast-unguarded': mut(
      'form',
      "if (!prefill) pushToast('success', 'Заявка отправлена');",
      "pushToast('success', 'Заявка отправлена');",
    ),
    'no-joined': mut('form', '{ ...res.request, joined: res.joined }', 'res.request'),
    'page-passes-prefill': mut(
      'page',
      '<RequestFormModal',
      '<RequestFormModal prefill={{ type: "repair", deviceId: 1 }}',
    ),
  };
  for (const [n, m] of Object.entries(mutants)) {
    const caught = check(m).length > 0;
    console.log(`${TAG} mutant ${n}: ${caught ? 'caught' : 'MISSED'}`);
    results.push([n, caught]);
  }
  const failed = results.filter(([, ok]) => !ok).map(([n]) => n);
  if (failed.length > 0) fail(`гейт не отличает плохие фикстуры: ${failed.join(', ')}`);
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
