#!/usr/bin/env node
// [check-group-move-status] Структурный гейт попапа переноса группы со сменой
// статуса (фаза 41.8, план 34, D-34).
//
// Правило перехода «склад <-> не склад», число устройств и подпись целевого
// статуса приходят с сервера (groups.moveStatusPreview); UI их не зеркалит.
// Гейт закрепляет только структуру GroupMoveModal.svelte (вне комментариев):
//   [M1] groups.moveStatusPreview( вызван внутри $effect, вместе с untrack(;
//   [M2] устаревший ответ не затирает свежий: сравнение `mySeq === previewSeq`
//        стоит ДО присваивания `preview = p`;
//   [M3] в confirmMove поле change_status вычисляется из preview и changeStatus
//        (нет голого true/false);
//   [M4] единственный <Checkbox обёрнут {#if по showStatusBox, а showStatusBox
//        выводится из preview и transition;
//   [M5] в файле нет литералов кодов и подписей статусов (подписи с сервера);
//   [M6] notifyPlaceContentChanged( и groups.move( в одной функции.
// Структурный гейт НЕ доказывает поведение в живом приложении: это живая UAT.
//
// Usage:
//   node scripts/check-group-move-status.mjs
//   node scripts/check-group-move-status.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-move-status]';
const FILE = 'src/features/groups/GroupMoveModal.svelte';

function extractBalanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return src.slice(openIdx + 1, i);
    }
  }
  return null;
}

function functionBody(code, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

function effectBodies(code) {
  const out = [];
  for (const m of code.matchAll(/\$effect\(/g)) {
    const body = extractBalanced(code, m.index + '$effect'.length, '(', ')');
    if (body !== null) out.push(body);
  }
  return out;
}

export function scan(raw) {
  const code = stripComments(raw);
  const v = [];

  // [M1] + [M2]
  const eff = effectBodies(code).find((b) => b.includes('moveStatusPreview('));
  if (!eff) {
    v.push(`[M1] ${FILE}: groups.moveStatusPreview( не вызван внутри $effect.`);
  } else {
    if (!eff.includes('untrack(')) {
      v.push(`[M1] ${FILE}: эффект предпросмотра без untrack( (запись состояния зациклит эффект).`);
    }
    const cmp = eff.search(/\bmySeq\s*===\s*previewSeq\b/);
    const assign = eff.search(/\bpreview\s*=\s*p\b/);
    if (cmp === -1 || assign === -1 || cmp > assign) {
      v.push(`[M2] ${FILE}: нет сравнения mySeq === previewSeq ДО присваивания preview = p.`);
    }
  }

  // [M3] + [M6]
  const cm = functionBody(code, 'confirmMove');
  if (cm === null) {
    v.push(`[M3] ${FILE}: не найдена функция confirmMove.`);
  } else {
    const m = /\bchange_status\s*:\s*([^\n]+)/.exec(cm);
    if (!m || !/\bpreview\b/.test(m[1]) || !/\bchangeStatus\b/.test(m[1])) {
      v.push(`[M3] ${FILE}: change_status в confirmMove не вычисляется из preview и changeStatus.`);
    }
    if (!cm.includes('groups.move(') || !cm.includes('notifyPlaceContentChanged(')) {
      v.push(
        `[M6] ${FILE}: groups.move( и notifyPlaceContentChanged( не в одной функции confirmMove.`,
      );
    }
  }

  // [M4]
  const boxes = [...code.matchAll(/<Checkbox\b/g)];
  if (boxes.length !== 1) {
    v.push(`[M4] ${FILE}: ожидался один <Checkbox, найдено ${boxes.length}.`);
  } else {
    const before = code.slice(0, boxes[0].index);
    const ifIdx = before.lastIndexOf('{#if');
    const cond = ifIdx === -1 ? '' : before.slice(ifIdx, before.indexOf('}', ifIdx) + 1);
    if (!cond.includes('showStatusBox')) {
      v.push(`[M4] ${FILE}: <Checkbox не обёрнут {#if showStatusBox …}.`);
    }
    const sb = /const\s+showStatusBox\s*=\s*\$derived\(([\s\S]*?)\);/.exec(code);
    if (!sb || !/\bpreview\b/.test(sb[1]) || !/\btransition\b/.test(sb[1])) {
      v.push(`[M4] ${FILE}: showStatusBox не выводится из preview и transition.`);
    }
  }

  // [M5]
  for (const lit of ['в_работе', 'на_складе', 'В работе', 'На складе']) {
    if (code.includes(lit))
      v.push(`[M5] ${FILE}: литерал статуса «${lit}» (подписи приходят с сервера).`);
  }
  return v;
}

function run() {
  const raw = fs.readFileSync(path.join(UI_ROOT, FILE), 'utf8');
  const v = scan(raw);
  if (v.length) {
    for (const x of v) console.error(`${TAG} ${x}`);
    process.exit(1);
  }
  console.log(`${TAG} OK — структура попапа переноса со сменой статуса на месте`);
}

function selftest() {
  const raw = fs.readFileSync(path.join(UI_ROOT, FILE), 'utf8');
  const base = scan(raw);
  if (base.length) {
    console.error(`${TAG} selftest: контроль красный: ${base.join(' | ')}`);
    process.exit(1);
  }
  const mutants = [
    [
      'убран untrack',
      'M1',
      'untrack(() => {\n      changeStatus = true;',
      'void (() => {\n      changeStatus = true;',
    ],
    [
      'убрано сравнение с previewSeq',
      'M2',
      'if (!stale && mySeq === previewSeq) preview = p;',
      'if (!stale) preview = p;',
    ],
    [
      'change_status: true',
      'M3',
      'preview?.transition != null && preview.devices_affected > 0 && changeStatus,',
      'true,',
    ],
    ['чекбокс вне условия', 'M4', '{#if showStatusBox && preview}', '{#if true}'],
    [
      'литерал «На складе»',
      'M5',
      'let previewSeq = $state(0);',
      "let previewSeq = $state(0);\n  const lit = 'На складе';",
    ],
    [
      'notify вынесен в другую функцию',
      'M6',
      'function handleClose(): void {',
      'function elsewhere(): void {\n    notifyPlaceContentChanged([]);\n  }\n\n  function handleClose(): void {',
      ['notifyPlaceContentChanged(result.changed_place_ids);', ''],
    ],
  ];
  let bad = 0;
  for (const [name, rule, anchor, repl, extra] of mutants) {
    const count = raw.split(anchor).length - 1;
    if (count !== 1) {
      console.error(
        `${TAG} selftest: якорь мутанта «${name}» встречается ${count} раз(а), нужен 1`,
      );
      bad++;
      continue;
    }
    let mutated = raw.replace(anchor, () => repl);
    if (extra) {
      if (mutated.split(extra[0]).length - 1 !== 1) {
        console.error(`${TAG} selftest: второй якорь мутанта «${name}» не уникален`);
        bad++;
        continue;
      }
      mutated = mutated.replace(extra[0], () => extra[1]);
    }
    const found = scan(mutated);
    if (!found.some((x) => x.startsWith(`[${rule}]`))) {
      console.error(`${TAG} selftest: мутант «${name}» не пойман правилом [${rule}]`);
      bad++;
    } else {
      console.log(`${TAG} мутант «${name}»: пойман [${rule}]`);
    }
  }
  if (bad) process.exit(1);
  console.log(`${TAG} selftest OK — контроль зелёный, ${mutants.length} мутантов красные`);
}

if (process.argv.includes('--selftest')) selftest();
else run();
