#!/usr/bin/env node
// [check-return-group-split] Структурный гейт вопроса при разбиении контейнерной
// группы возвратом (UD-2, D-45, D-46; ReturnModal.svelte, GroupSplitChoicePopup.svelte,
// api/acts.ts).
//
// Почему он существует: без вопроса возврат части группы молча выводит устройства из
// группы (Release), а перенос группы вообще недоступен. Сервер выбор принимает, но
// «спросить» обязан клиент. svelte-check / build этого не видят.
//
// ВАЖНО: гейт СТРУКТУРНЫЙ. Вид попапа, фокус на «Отмена», Esc и перенос переносимого
// проверяет пользователь по чек-листу плана 41.2-28.
//
// Правила:
//   (A) handleSubmit: вызовы returnGroupSplitPreview( и updateReturnGroupSplitPreview(
//       есть; проверка place_conflicts стоит ниже обоих вызовов preview, ведёт к
//       раннему return и стоит ВЫШЕ открытия попапа (splitPrompt =) и ВЫШЕ первого
//       вызова send( (отправки); открытие попапа тоже выше send(; тело send содержит
//       acts.doReturn( с третьим и acts.updateReturn( со вторым аргументом (выбор).
//   (B) <GroupSplitChoicePopup стоит внутри {#if splitPrompt} в разметке и не вложен
//       в <Modal ...> формы возврата (модалка над модалкой запрещена); разметка
//       читает formError (встроенная ошибка).
//   (C) GroupSplitChoicePopup презентационный (нет acts., нет импорта api),
//       initialFocus=".modal-footer .btn-secondary", три подписи по одному разу,
//       disabled кнопки «Изменить расположение группы» читает places_match, слова
//       mixed_places нет.
//   (D) api/acts.ts: объявлены returnGroupSplitPreview и updateReturnGroupSplitPreview
//       (с именами команд), doReturn и updateReturn принимают необязательный выбор.
//   (M) отсутствующий файл — нарушение, не SKIP.
//
// Usage:
//   node scripts/check-return-group-split.mjs              # проверить репозиторий
//   node scripts/check-return-group-split.mjs --src=<dir>   # копия каталога ui/
//   node scripts/check-return-group-split.mjs --selftest    # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-return-group-split]';

const MODAL = 'src/features/acts/ReturnModal.svelte';
const POPUP = 'src/features/acts/GroupSplitChoicePopup.svelte';
const API = 'src/lib/api/acts.ts';
const FILES = [MODAL, POPUP, API];

const LABELS = ['Отмена', 'Вывести из группы', 'Изменить расположение группы'];

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

function topLevelArgCount(args) {
  let depth = 0;
  let n = args.trim() === '' ? 0 : 1;
  for (const c of args) {
    if ('([{'.includes(c)) depth++;
    else if (')]}'.includes(c)) depth--;
    else if (c === ',' && depth === 0) n++;
  }
  return n;
}

function callArgs(code, name) {
  const out = [];
  const re = new RegExp(`\\b${name.replace('.', '\\.')}\\s*\\(`, 'g');
  let m;
  while ((m = re.exec(code)) !== null) {
    out.push(extractBalanced(code, m.index + m[0].length - 1, '(', ')') ?? '');
  }
  return out;
}

function markupOf(code) {
  return code.replace(/<script\b[\s\S]*?<\/script>/g, '').replace(/<style\b[\s\S]*?<\/style>/g, '');
}

function enclosingIfCondition(markup, pos) {
  const stack = [];
  const re = /\{#if\b([^}]*)\}|\{\/if\}/g;
  let m;
  while ((m = re.exec(markup)) !== null && m.index < pos) {
    if (m[0] === '{/if}') stack.pop();
    else stack.push(m[1]);
  }
  return stack.length > 0 ? stack[stack.length - 1] : null;
}

/** Есть ли `return` в ветке `if (...) { ... }`, чьё условие содержит needle. */
function branchReturns(body, needle) {
  const i = body.indexOf(needle);
  if (i === -1) return false;
  const braceOpen = body.indexOf('{', i);
  if (braceOpen === -1) return false;
  const branch = extractBalanced(body, braceOpen, '{', '}');
  return branch !== null && /\breturn\b/.test(branch);
}

export function scanSource(files, violations) {
  const s = {};
  for (const [rel, text] of Object.entries(files)) s[rel] = stripComments(text);
  for (const rel of FILES) {
    if (s[rel] === undefined) violations.push(`(M) ${rel}: файл отсутствует — нарушение, не SKIP.`);
  }

  const modal = s[MODAL];
  if (modal !== undefined) {
    const body = functionBody(modal, 'handleSubmit');
    if (body === null) {
      violations.push(`(A) ${MODAL}: не найдена функция handleSubmit — антивакуум.`);
    } else {
      const iPrev = body.indexOf('returnGroupSplitPreview(');
      const iPrevUpd = body.indexOf('updateReturnGroupSplitPreview(');
      if (iPrev === -1) {
        violations.push(
          `(A) ${MODAL}: handleSubmit не вызывает returnGroupSplitPreview( (создание).`,
        );
      }
      if (iPrevUpd === -1) {
        violations.push(
          `(A) ${MODAL}: handleSubmit не вызывает updateReturnGroupSplitPreview( (правка).`,
        );
      }
      const iConf = body.indexOf('warning.place_conflicts');
      const iPrompt = body.indexOf('splitPrompt =');
      const iSend = body.search(/\bsend\s*\(/);
      if (iSend === -1)
        violations.push(`(A) ${MODAL}: handleSubmit не вызывает send( — антивакуум.`);
      if (iConf === -1) {
        violations.push(`(A) ${MODAL}: handleSubmit не проверяет place_conflicts (D-46).`);
      } else {
        if (Math.max(iPrev, iPrevUpd) > iConf) {
          violations.push(`(A) ${MODAL}: place_conflicts проверяется ВЫШЕ вызова preview.`);
        }
        if (iPrompt !== -1 && iConf > iPrompt) {
          violations.push(`(A) ${MODAL}: place_conflicts проверяется НИЖЕ открытия попапа.`);
        }
        if (iSend !== -1 && iConf > iSend) {
          violations.push(
            `(A) ${MODAL}: place_conflicts проверяется ПОСЛЕ send( — возврат уже ушёл.`,
          );
        }
        if (!branchReturns(body, 'warning.place_conflicts')) {
          violations.push(`(A) ${MODAL}: ветка place_conflicts не завершается return.`);
        }
      }
      if (iPrompt === -1) {
        violations.push(`(A) ${MODAL}: handleSubmit не открывает попап (splitPrompt =).`);
      } else {
        if (Math.max(iPrev, iPrevUpd) > iPrompt) {
          violations.push(`(A) ${MODAL}: попап открывается ВЫШЕ вызова preview.`);
        }
        if (iSend !== -1 && iPrompt > iSend) {
          violations.push(`(A) ${MODAL}: попап открывается ПОСЛЕ send( — возврат уже ушёл.`);
        }
        if (!branchReturns(body, 'warning.items')) {
          violations.push(
            `(A) ${MODAL}: ветка warning.items не завершается return (отправка без вопроса).`,
          );
        }
      }
      if (iSend !== -1 && Math.max(iPrev, iPrevUpd) > iSend) {
        violations.push(`(A) ${MODAL}: send( вызывается ВЫШЕ preview.`);
      }
    }

    const sendBody = functionBody(modal, 'send');
    if (sendBody === null) {
      violations.push(`(A) ${MODAL}: не найдена функция send — антивакуум.`);
    } else {
      const dr = callArgs(sendBody, 'acts.doReturn');
      const ur = callArgs(sendBody, 'acts.updateReturn');
      if (dr.length === 0 || ur.length === 0) {
        violations.push(`(A) ${MODAL}: send не вызывает acts.doReturn( и acts.updateReturn(.`);
      }
      if (dr.some((a) => topLevelArgCount(a) < 3)) {
        violations.push(`(A) ${MODAL}: acts.doReturn( без третьего аргумента (выбор).`);
      }
      if (ur.some((a) => topLevelArgCount(a) < 2)) {
        violations.push(`(A) ${MODAL}: acts.updateReturn( без второго аргумента (выбор).`);
      }
    }
    if (/\bacts\.(doReturn|updateReturn)\s*\(/.test(modal.replace(sendBody ?? '\u0000', ''))) {
      violations.push(`(A) ${MODAL}: отправка возврата вне send — мимо вопроса.`);
    }

    const markup = markupOf(modal);
    const tag = /<GroupSplitChoicePopup\b/.exec(markup);
    if (tag === null) {
      violations.push(`(B) ${MODAL}: в разметке не смонтирован <GroupSplitChoicePopup.`);
    } else {
      const cond = enclosingIfCondition(markup, tag.index);
      if (cond === null || !cond.includes('splitPrompt')) {
        violations.push(
          `(B) ${MODAL}: <GroupSplitChoicePopup обязан стоять внутри {#if splitPrompt}.`,
        );
      }
      const mm = /<Modal\b/.exec(markup);
      const mEnd = markup.indexOf('</Modal>');
      if (mm !== null && mEnd !== -1 && tag.index > mm.index && tag.index < mEnd) {
        violations.push(
          `(B) ${MODAL}: попап вложен в <Modal формы — модалка над модалкой запрещена.`,
        );
      }
    }
    if (!/\{#if\s+formError\b/.test(markup)) {
      violations.push(`(B) ${MODAL}: разметка не показывает formError (встроенная ошибка, D-46).`);
    }
  }

  const popup = s[POPUP];
  if (popup !== undefined) {
    if (/\bacts\./.test(popup)) {
      violations.push(`(C) ${POPUP}: попап вызывает acts. — он обязан быть презентационным.`);
    }
    if (/from\s*['"][^'"]*\bapi\b[^'"]*['"]/.test(popup)) {
      violations.push(`(C) ${POPUP}: импорт из api — попап обязан быть презентационным.`);
    }
    if (!popup.includes('initialFocus=".modal-footer .btn-secondary"')) {
      violations.push(
        `(C) ${POPUP}: нет initialFocus=".modal-footer .btn-secondary" — фокус не на «Отмена».`,
      );
    }
    if (popup.includes('mixed_places')) {
      violations.push(`(C) ${POPUP}: упомянут mixed_places — такого исхода у попапа нет.`);
    }
    for (const label of LABELS) {
      const n = (popup.match(new RegExp(`>\\s*${label}\\s*<`, 'g')) ?? []).length;
      if (n !== 1)
        violations.push(`(C) ${POPUP}: подпись «${label}» встречается ${n} раз, нужно ровно 1.`);
    }
    const mv =
      /<Button\b(?:(?!<Button\b)[\s\S])*?>\s*Изменить расположение группы\s*<\/Button\s*>/.exec(
        popup,
      );
    const def = /\bmoveDisabled\s*=\s*\$derived\(([^;]*)\);/.exec(popup);
    if (mv === null || !/disabled=\{moveDisabled\}/.test(mv[0])) {
      violations.push(
        `(C) ${POPUP}: «Изменить расположение группы» не привязана disabled={moveDisabled}.`,
      );
    } else if (def === null || !def[1].includes('places_match')) {
      violations.push(`(C) ${POPUP}: moveDisabled не зависит от places_match (D-46, UD-3).`);
    }
  }

  const api = s[API];
  if (api !== undefined) {
    for (const [name, cmd] of [
      ['returnGroupSplitPreview', 'acts_return_group_split_preview'],
      ['updateReturnGroupSplitPreview', 'acts_update_return_group_split_preview'],
    ]) {
      if (!new RegExp(`\\b${name}:`).test(api) || !api.includes(`'${cmd}'`)) {
        violations.push(`(D) ${API}: не объявлен ${name} (команда ${cmd}).`);
      }
    }
    for (const [name, min] of [
      ['doReturn', 3],
      ['updateReturn', 2],
    ]) {
      const m = new RegExp(`\\b${name}:\\s*\\(`).exec(api);
      if (m === null) {
        violations.push(`(D) ${API}: не найден ${name}: — антивакуум.`);
      } else {
        const params = extractBalanced(api, m.index + m[0].length - 1, '(', ')') ?? '';
        if (topLevelArgCount(params) < min || !params.includes('groupSplit?')) {
          violations.push(`(D) ${API}: ${name} не принимает необязательный groupSplit.`);
        }
      }
    }
  }
}

function readSources(root) {
  const out = {};
  for (const rel of FILES) {
    try {
      out[rel] = fs.readFileSync(path.join(root, rel), 'utf8');
    } catch {
      /* отсутствует — (M) */
    }
  }
  return out;
}

function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
  }
  return source.replace(from, () => to);
}

function runSelfTest() {
  const raw = readSources(UI_ROOT);
  const clean = [];
  scanSource(raw, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники обязаны проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }
  const withFile = (rel, text) => ({ ...raw, [rel]: text });
  const md = raw[MODAL];
  const pp = raw[POPUP];
  let mutants;
  try {
    mutants = [
      {
        name: '(1) убран preview в создании',
        files: withFile(
          MODAL,
          mutate(md, 'acts.returnGroupSplitPreview(', 'acts.noPreview(', 'drop create preview'),
        ),
        rule: 'A',
      },
      {
        name: '(2) убран preview в правке',
        files: withFile(
          MODAL,
          mutate(md, 'acts.updateReturnGroupSplitPreview(', 'acts.noPreview(', 'drop edit preview'),
        ),
        rule: 'A',
      },
      {
        name: '(3) ПЕРЕСТАНОВКА: send( выше preview',
        files: withFile(
          MODAL,
          mutate(
            md,
            '    let warning: ReturnGroupSplitWarningDto;',
            '    await send(built);\n    let warning: ReturnGroupSplitWarningDto;',
            'send before preview',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(4) ПЕРЕСТАНОВКА: place_conflicts после send(',
        files: withFile(
          MODAL,
          mutate(
            md,
            '    if (warning.place_conflicts.length > 0) {',
            '    await send(built);\n    if (warning.place_conflicts.length > 0) {',
            'send before conflicts',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(5) убрана проверка place_conflicts',
        files: withFile(
          MODAL,
          mutate(md, 'warning.place_conflicts.length > 0', 'false', 'drop conflicts'),
        ),
        rule: 'A',
      },
      {
        name: '(6) убран третий аргумент doReturn',
        files: withFile(
          MODAL,
          mutate(
            md,
            'acts.doReturn(built.actId, built.payload, groupSplit)',
            'acts.doReturn(built.actId, built.payload)',
            'drop doReturn arg',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(7) убран второй аргумент updateReturn',
        files: withFile(
          MODAL,
          mutate(
            md,
            'acts.updateReturn(built.payload, groupSplit)',
            'acts.updateReturn(built.payload)',
            'drop updateReturn arg',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(8) убран initialFocus',
        files: withFile(
          POPUP,
          mutate(pp, '  initialFocus=".modal-footer .btn-secondary"\n', '', 'drop initialFocus'),
        ),
        rule: 'C',
      },
      {
        name: '(9) acts.doReturn( внутри попапа',
        files: withFile(
          POPUP,
          mutate(
            pp,
            '<script lang="ts">',
            '<script lang="ts">\n  const _x = () => acts.doReturn(1, {}, undefined);',
            'acts in popup',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(10) потерян исход «Вывести из группы»',
        files: withFile(
          POPUP,
          mutate(pp, '>Вывести из группы<', '>Вывести<', 'drop release label'),
        ),
        rule: 'C',
      },
      {
        name: '(11) disabled не привязан к places_match',
        files: withFile(
          POPUP,
          mutate(
            pp,
            'items.some((item) => !item.places_match)',
            'false',
            'drop places_match dependency',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(12) попап под чужим {#if}',
        files: withFile(MODAL, mutate(md, '{#if splitPrompt}', '{#if open}', 'foreign if')),
        rule: 'B',
      },
      {
        name: '(13) api.doReturn без groupSplit',
        files: withFile(
          API,
          mutate(
            raw[API],
            'doReturn: (actId: number, payload: ActReturnDto, groupSplit?: GroupSplitChoiceDto)',
            'doReturn: (actId: number, payload: ActReturnDto)',
            'api drop param',
          ),
        ),
        rule: 'D',
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const m of mutants) {
    const v = [];
    scanSource(m.files, v);
    if (v.some((line) => line.startsWith(`(${m.rule})`))) {
      console.error(`${TAG} [selftest] PASS — ${m.name} (поймано правилом ${m.rule})`);
    } else {
      failed++;
      console.error(
        `${TAG} [selftest] FAIL — ${m.name}: гейт не покраснел правилом ${m.rule} (нарушения: ${v.length ? v.join(' | ') : 'нет'})`,
      );
    }
  }
  const v = [];
  scanSource({}, v);
  if (!v.some((l) => l.startsWith('(M)'))) {
    failed++;
    console.error(`${TAG} [selftest] FAIL — пустой набор файлов не дал нарушения.`);
  }
  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(
    `${TAG} [selftest] PASS — реальные исходники чисты, мутантов поймано: ${mutants.length}.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--selftest')) process.exit(runSelfTest() ? 0 : 1);
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;
  const violations = [];
  scanSource(readSources(root), violations);
  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
