#!/usr/bin/env node
// [check-chat-linkify] Гейт D-08 / T-41.5-43: linkify выделяет только http(s)-ссылки;
// javascript:, file:, mailto:, голые www. остаются текстом; склейка фрагментов = исходник.
// Структурный/поведенческий; рантайм UI доказывает живая проверка.
//
// Usage:
//   node scripts/check-chat-linkify.mjs
//   node scripts/check-chat-linkify.mjs --selftest   (мутант, пропускающий javascript:)

import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-chat-linkify]';
const TARGET = path.resolve(__dirname, '../src/features/requests/chat/linkify.ts');

const T = (value) => ({ type: 'text', value });
const L = (value) => ({ type: 'link', value });

const CASES = [
  ['Принтер снова мнёт бумагу', [T('Принтер снова мнёт бумагу')]],
  [
    'см. https://example.com/a?b=1, и всё',
    [T('см. '), L('https://example.com/a?b=1'), T(', и всё')],
  ],
  ['(http://example.com)', [T('('), L('http://example.com'), T(')')]],
  ['HTTPS://Example.com', [L('HTTPS://Example.com')]],
  ['javascript:alert(1)', [T('javascript:alert(1)')]],
  ['file:///etc/passwd', [T('file:///etc/passwd')]],
  ['mailto:a@b.c', [T('mailto:a@b.c')]],
  ['www.example.com', [T('www.example.com')]],
  ['https://', [T('https://')]],
  ['http://.', [T('http://.')]],
];

const INVARIANT = ['', 'строка\nвторая https://a.b/c\nтретья', 'x http://a.b. y https://', '   '];

export function runChecks(linkify) {
  const failed = [];
  for (const [input, expected] of CASES) {
    const got = linkify(input);
    if (JSON.stringify(got) !== JSON.stringify(expected)) {
      failed.push(`${JSON.stringify(input)} -> ${JSON.stringify(got)}`);
    }
  }
  for (const input of [...INVARIANT, ...CASES.map((c) => c[0])]) {
    const joined = linkify(input)
      .map((s) => s.value)
      .join('');
    if (joined !== input) failed.push(`инвариант склейки: ${JSON.stringify(input)}`);
  }
  // Ни одна ссылка не начинается не с http(s).
  for (const input of [...INVARIANT, ...CASES.map((c) => c[0])]) {
    for (const s of linkify(input)) {
      if (s.type === 'link' && !/^https?:\/\//i.test(s.value)) {
        failed.push(`ссылка с чужой схемой: ${s.value}`);
      }
    }
  }
  return failed;
}

async function main() {
  const { linkify } = await import(pathToFileURL(TARGET).href);
  if (process.argv.includes('--selftest')) {
    // Мутант: javascript:… считается ссылкой.
    const mutant = (text) =>
      /^javascript:/i.test(text) ? [{ type: 'link', value: text }] : linkify(text);
    if (runChecks(mutant).length === 0) {
      console.error(`${TAG} FAIL: selftest не ловит мутанта с javascript:`);
      process.exit(1);
    }
    console.log(`${TAG} selftest OK`);
    return;
  }
  const failed = runChecks(linkify);
  if (failed.length > 0) {
    console.error(`${TAG} FAIL:\n  ${failed.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}

main();
