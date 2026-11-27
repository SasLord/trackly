#!/usr/bin/env node
// [check-request-create-idempotency] Гейт идемпотентной отправки формы заявки (фаза 43, план 09; D-11/D-12, DLV-03).
//
// Структурный: поведение «одно нажатие после сбоя -> один результат» он не доказывает —
// svelte-check и build его не видят; живая проверка в плане 12.
//
// Правила (RequestFormModal.svelte):
//  [I1] newClientId и sendWithRetry импортируются из '@trackly/transport/sendWithRetry';
//  [I2] выдатчик `createRequestIdIssuer(` объявлен в тексте ДО `async function handleSubmit`;
//  [I3] вызов `requests.create(` находится внутри `sendWithRetry(`, его объект содержит
//       `clientRequestId`;
//  [I4] в теле handleSubmit нет `newClientId(` (генерация в обработчике дала бы каждому
//       нажатию новый id и свела идемпотентность на нет);
//  [I5] handleSubmit берёт id как `.idFor(requestSignature(...))` и передаёт его в requests.create
//       как `clientRequestId` (id держится, пока тело заявки не менялось; WR-02, уточнение D-12).
//
// Исполняющие кейсы (НАСТОЯЩИЙ requestIdPolicy.ts через стенд _rune-harness.mjs):
//  [Q1] без правок тела id тот же (генератор вызван один раз);
//  [Q2] после смены подписи (правка тела) id новый и далее стабилен;
//  [Q3] reset() (открытие формы) выдаёт новый id;
//  [Q4] подпись: равна для равных значений, меняется при смене любого поля тела, trim описания,
//       поля, не уходящие в тело данного типа, подпись не меняют.
//
// Usage:
//   node scripts/check-request-create-idempotency.mjs
//   node scripts/check-request-create-idempotency.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';
import { gateMain, installHarness, SRC, srcUrl } from './_rune-harness.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FORM_REL = 'features/requests/RequestFormModal.svelte';
const POLICY_REL = 'features/requests/requestIdPolicy.ts';
const FORM = path.resolve(__dirname, '../src', FORM_REL);
const TAG = '[check-request-create-idempotency]';

/** @returns {string[]} нарушения структурных правил */
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
  const decl = /\bcreateRequestIdIssuer\(/.exec(f);
  if (!decl || decl.index > hs)
    bad.push('[I2] объявление выдатчика createRequestIdIssuer( отсутствует или стоит после handleSubmit');

  // тело handleSubmit: до закрывающего `</script>` (функция последняя в скрипте)
  const end = f.indexOf('</script>', hs);
  const body = f.slice(hs, end < 0 ? undefined : end);

  // [I3]
  const swr = body.indexOf('sendWithRetry(');
  const create = body.indexOf('requests.create(');
  let obj = '';
  if (swr < 0 || create < 0 || swr > create) {
    bad.push('[I3] requests.create( не обёрнут в sendWithRetry(');
  } else {
    obj = body.slice(create, body.indexOf('\n      );', create) + 1 || undefined);
    if (!/\bclientRequestId\b/.test(obj)) bad.push('[I3] requests.create без clientRequestId');
  }

  // [I4]
  if (/\bnewClientId\(/.test(body)) bad.push('[I4] newClientId( вызывается внутри handleSubmit');

  // [I5]
  if (!/\.idFor\(\s*requestSignature\(/.test(body))
    bad.push('[I5] handleSubmit не берёт id через .idFor(requestSignature(...))');
  else if (obj && !/clientRequestId\s*:\s*attemptId|\bclientRequestId\b\s*[,}\n]/.test(obj))
    bad.push('[I5] в requests.create уходит не id выдатчика');

  return bad;
}

async function runCases(overrides) {
  installHarness({ overrides });
  const bad = [];
  const expect = (code, cond, msg) => {
    if (!cond) bad.push(`[${code}] ${msg}`);
  };

  // Структурные правила по тексту формы (с мутациями формы, если заданы).
  let form = fs.readFileSync(FORM, 'utf8');
  for (const [from, to] of overrides[path.join(SRC, FORM_REL)] ?? []) {
    if (form.split(from).length - 1 !== 1) throw new Error(`якорь формы не уникален: ${from}`);
    form = form.replace(from, to);
  }
  bad.push(...check(form));

  const mod = await import(srcUrl(POLICY_REL));
  const guard = (code, f) => {
    try {
      f();
    } catch (e) {
      bad.push(`[${code}] кейс бросил ${e}`);
    }
  };
  const gen = () => {
    let n = 0;
    return { calls: () => n, next: () => `id${++n}` };
  };

  guard('Q1', () => {
    const g = gen();
    const it = mod.createRequestIdIssuer(g.next);
    const a = it.idFor('s');
    const b = it.idFor('s');
    expect('Q1', a === b && g.calls() === 1, `Q1: ${a} / ${b}, вызовов генератора ${g.calls()}`);
  });
  guard('Q2', () => {
    const g = gen();
    const it = mod.createRequestIdIssuer(g.next);
    const a = it.idFor('s');
    const b = it.idFor('s2');
    const c = it.idFor('s2');
    expect('Q2', a !== b, `Q2: после правки id не сменился (${a} / ${b})`);
    expect('Q2', b === c, `Q2: id нестабилен при неизменной подписи (${b} / ${c})`);
  });
  guard('Q3', () => {
    const g = gen();
    const it = mod.createRequestIdIssuer(g.next);
    const a = it.idFor('s');
    it.reset();
    const b = it.idFor('s');
    expect('Q3', a !== b, `Q3: после reset id прежний (${a})`);
  });
  guard('Q4', () => {
    const sig = mod.requestSignature;
    const base = sig('cartridge_replace', 5, null, 'текст');
    expect('Q4', base === sig('cartridge_replace', 5, null, 'текст'), 'Q4: равные значения -> разные подписи');
    expect('Q4', base !== sig('free_form', 5, null, 'текст'), 'Q4: тип не влияет');
    expect('Q4', base !== sig('cartridge_replace', 6, null, 'текст'), 'Q4: принтер не влияет');
    expect('Q4', base !== sig('cartridge_replace', 5, null, 'другой'), 'Q4: описание не влияет');
    expect('Q4', base === sig('cartridge_replace', 5, null, '  текст \n'), 'Q4: trim описания не учтён');
    expect('Q4', base === sig('cartridge_replace', 5, 9, 'текст'), 'Q4: категория попала в подпись cartridge_replace');
    const ff = sig('free_form', null, 3, 'т');
    expect('Q4', ff !== sig('free_form', null, 4, 'т'), 'Q4: категория не влияет на подпись free_form');
    expect('Q4', ff === sig('free_form', 7, 3, 'т'), 'Q4: принтер попал в подпись free_form');
  });
  return bad;
}

const F = POLICY_REL;
const MUTANTS = [
  {
    name: 'выдатчик выпускает новый id на каждый вызов',
    file: F,
    from: 'if (current === null || signature !== lastSignature) {',
    to: 'if (true) {',
    expect: 'Q1',
  },
  {
    name: 'выдатчик не реагирует на смену подписи',
    file: F,
    from: 'if (current === null || signature !== lastSignature) {',
    to: 'if (current === null) {',
    expect: 'Q2',
  },
  {
    name: 'reset не сбрасывает id',
    file: F,
    from: '      current = null;\n      lastSignature = null;',
    to: '      lastSignature = lastSignature;',
    expect: 'Q3',
  },
  {
    name: 'подпись игнорирует описание',
    file: F,
    from: 'description.trim()]',
    to: "'']",
    expect: 'Q4',
  },
  {
    name: 'handleSubmit без idFor(',
    file: FORM_REL,
    from: 'const attemptId = idIssuer.idFor(',
    to: 'const attemptId = String(',
    expect: 'I5',
  },
  {
    name: 'id генерируется внутри handleSubmit',
    file: FORM_REL,
    from: '    submitting = true;\n    try {\n      if (requestType',
    to: '    submitting = true;\n    const regen = newClientId();\n    try {\n      if (requestType',
    expect: 'I4',
  },
  {
    name: 'requests.create без clientRequestId',
    file: FORM_REL,
    from: '          clientRequestId: attemptId,\n        }),',
    to: '        }),',
    expect: 'I3',
  },
  {
    name: 'без sendWithRetry',
    file: FORM_REL,
    from: 'await sendWithRetry(() =>',
    to: 'await Promise.resolve().then(() =>',
    expect: 'I3',
  },
  {
    name: 'выдатчик объявлен после handleSubmit',
    file: FORM_REL,
    edits: [
      ['  const idIssuer = createRequestIdIssuer(newClientId);\n', ''],
      ['</script>', '  const idIssuer2 = createRequestIdIssuer(newClientId);\n</script>'],
    ],
    expect: 'I2',
  },
  {
    name: 'импорт newClientId убран',
    file: FORM_REL,
    from: 'import { newClientId, sendWithRetry } from',
    to: 'import { sendWithRetry } from',
    expect: 'I1',
  },
];

await gateMain({ tag: TAG, scriptUrl: import.meta.url, mutants: MUTANTS, runCases });
