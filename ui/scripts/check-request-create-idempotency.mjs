#!/usr/bin/env node
// [check-request-create-idempotency] Гейт идемпотентной отправки формы заявки (фаза 43, план 09; D-11/D-12, DLV-03).
//
// Структурный: поведение «одно нажатие после сбоя -> один результат» он не доказывает —
// svelte-check и build его не видят; живая проверка в плане 12.
//
// Правила (RequestFormModal.svelte):
//  [I1] newClientId и sendWithRetry импортируются из '@trackly/transport/sendWithRetry';
//  [I2] объявление `clientRequestId` стоит в тексте ДО `async function handleSubmit`;
//  [I3] вызов `requests.create(` находится внутри `sendWithRetry(`, его объект содержит
//       `clientRequestId`;
//  [I4] в теле handleSubmit нет `newClientId(` (генерация в обработчике дала бы каждому
//       нажатию новый id и свела идемпотентность на нет).
//
// Usage:
//   node scripts/check-request-create-idempotency.mjs
//   node scripts/check-request-create-idempotency.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FORM = path.resolve(__dirname, '../src/features/requests/RequestFormModal.svelte');
const TAG = '[check-request-create-idempotency]';

/** @returns {string[]} нарушения */
export function check(form) {
  const bad = [];
  const f = stripComments(form);

  // [I1]
  const imp = /import \{([^}]*)\} from '@trackly\/transport\/sendWithRetry';/.exec(f);
  if (!imp || !/\bnewClientId\b/.test(imp[1]) || !/\bsendWithRetry\b/.test(imp[1]))
    bad.push("[I1] нет импорта newClientId и sendWithRetry из '@trackly/transport/sendWithRetry'");

  const hs = f.indexOf('async function handleSubmit');
  if (hs < 0) return [...bad, '[I2] нет async function handleSubmit'];

  // [I2]
  const decl = /\blet clientRequestId\b/.exec(f);
  if (!decl || decl.index > hs)
    bad.push('[I2] объявление clientRequestId отсутствует или стоит после handleSubmit');

  // тело handleSubmit: до закрывающего `</script>` (функция последняя в скрипте)
  const end = f.indexOf('</script>', hs);
  const body = f.slice(hs, end < 0 ? undefined : end);

  // [I3]
  const swr = body.indexOf('sendWithRetry(');
  const create = body.indexOf('requests.create(');
  if (swr < 0 || create < 0 || swr > create) {
    bad.push('[I3] requests.create( не обёрнут в sendWithRetry(');
  } else {
    const obj = body.slice(create, body.indexOf('\n      );', create) + 1 || undefined);
    if (!/\bclientRequestId\b/.test(obj)) bad.push('[I3] requests.create без clientRequestId');
  }

  // [I4]
  if (/\bnewClientId\(/.test(body)) bad.push('[I4] newClientId( вызывается внутри handleSubmit');

  return bad;
}

function selftest() {
  const real = fs.readFileSync(FORM, 'utf8');
  const count = (s, x) => s.split(x).length - 1;
  const mut = (name, edits, expect) => ({ name, edits, expect });
  const mutants = [
    mut(
      'id генерируется внутри handleSubmit',
      [
        [
          '    submitting = true;\n    try {\n      if (requestType',
          '    submitting = true;\n    const regen = newClientId();\n    try {\n      if (requestType',
        ],
      ],
      'I4',
    ),
    mut(
      'requests.create без clientRequestId',
      [['          clientRequestId,\n        }),', '        }),']],
      'I3',
    ),
    mut(
      'без sendWithRetry',
      [['await sendWithRetry(() =>', 'await Promise.resolve().then(() =>']],
      'I3',
    ),
    mut(
      'объявление id после handleSubmit',
      [
        ['  let clientRequestId = newClientId();\n', ''],
        ['</script>', '  let clientRequestId = newClientId();\n</script>'],
      ],
      'I2',
    ),
    mut(
      'импорт newClientId убран',
      [['import { newClientId, sendWithRetry } from', 'import { sendWithRetry } from']],
      'I1',
    ),
  ];

  const base = check(real);
  if (base.length > 0) {
    for (const b of base) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  let missed = 0;
  for (const m of mutants) {
    let src = real;
    for (const [from, to] of m.edits) {
      if (count(src, from) !== 1) {
        console.error(`${TAG} FAIL: selftest: якорь «${from}» не уникален (${count(src, from)})`);
        process.exit(1);
      }
      src = src.replace(from, to);
    }
    const bad = check(src);
    const ok = bad.some((b) => b.startsWith(`[${m.expect}]`));
    console.log(`${TAG} мутант «${m.name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${m.expect}]`);
    if (!ok) missed++;
  }
  if (missed > 0) process.exit(1);
  console.log(`${TAG} selftest OK — ${mutants.length} мутантов красные`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const bad = check(fs.readFileSync(FORM, 'utf8'));
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}
