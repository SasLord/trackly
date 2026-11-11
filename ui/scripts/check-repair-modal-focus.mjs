#!/usr/bin/env node
// [check-repair-modal-focus] Гейт: начальный фокус ремонтных попапов
// (src/features/requests/Repair*Modal.svelte) не должен стоять на PlacePicker.
//
// Почему: PlacePicker.handleFocus() раскрывает дерево при ЛЮБОМ фокусе, а Modal
// переводит фокус на элемент из `initialFocus`. Если там id PlacePicker, список
// «Куда переместить» раскрывается сам при открытии попапа, хотя значение уже
// подставлено (UAT 41.4, пункт 10). svelte-check и build этого не видят.
//
// Допустимо: кнопка футера, текстовое поле, отсутствие initialFocus.
// Гейт структурный (чтение исходников), рантайм фокуса не доказывает.
//
// Usage:
//   node scripts/check-repair-modal-focus.mjs
//   node scripts/check-repair-modal-focus.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIR = path.resolve(__dirname, '../src/features/requests');
const TAG = '[check-repair-modal-focus]';

/** id, назначенные компонентам PlacePicker в файле. */
function placePickerIds(src) {
  const ids = new Set();
  for (const m of src.matchAll(/<PlacePicker\b[^>]*>/g)) {
    const id = m[0].match(/\bid\s*=\s*(?:"([^"]+)"|'([^']+)'|\{\s*['"`]([^'"`]+)['"`]\s*\})/);
    if (id) ids.add(id[1] ?? id[2] ?? id[3]);
  }
  return ids;
}

/** Нарушения для одного файла: список строк. */
export function check(name, raw) {
  const src = stripComments(raw);
  const ids = placePickerIds(src);
  const bad = [];
  for (const m of src.matchAll(/\binitialFocus\s*=\s*(\{[^}]*\}|"[^"]*"|'[^']*')/g)) {
    const value = m[1];
    for (const id of ids) {
      if (new RegExp(`#${id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?![\\w-])`).test(value)) {
        bad.push(`${name}: initialFocus указывает на PlacePicker #${id}`);
      }
    }
  }
  return bad;
}

function selftest() {
  const badSrc = `<Modal initialFocus="#rep-x-place"><PlacePicker id="rep-x-place" /></Modal>`;
  const badTern = `<Modal initialFocus={m ? '#rep-x-place' : '#notes'}><PlacePicker id="rep-x-place" /></Modal>`;
  const good = `<Modal initialFocus=".modal-footer .btn-secondary"><PlacePicker id="rep-x-place" /></Modal>`;
  const goodNone = `<Modal><PlacePicker id="rep-x-place" /></Modal>`;
  const commented = `<!-- initialFocus="#rep-x-place" --><Modal initialFocus="#n"><PlacePicker id="rep-x-place" /></Modal>`;
  const ok =
    check('bad', badSrc).length === 1 &&
    check('badTern', badTern).length === 1 &&
    check('good', good).length === 0 &&
    check('goodNone', goodNone).length === 0 &&
    check('commented', commented).length === 0;
  if (!ok) {
    console.error(`${TAG} FAIL: selftest не отличает плохие фикстуры от хороших`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const files = fs
    .readdirSync(DIR)
    .filter((f) => /^Repair.*Modal\.svelte$/.test(f))
    .sort();
  if (files.length === 0) {
    console.error(`${TAG} FAIL: не найдено ни одного Repair*Modal.svelte`);
    process.exit(1);
  }
  const bad = files.flatMap((f) => check(f, fs.readFileSync(path.join(DIR, f), 'utf8')));
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK (${files.length} файлов)`);
}
