#!/usr/bin/env node
// [check-act-delete-choice] Структурный гейт попапа выбора при удалении акта
// (UD-1, D-42; ActsPage.svelte, GroupRestoreChoicePopup.svelte, api/acts.ts).
//
// Почему он существует: сервер при конфликте («группа перенесена после акта») ничего
// не удаляет и возвращает needs_group_restore. Если UI эту ветку не обрабатывает,
// пользователь видит «Акт удалён» и success-тост про акт, которого на деле не
// удалили. svelte-check / build этого не видят.
//
// ВАЖНО: гейт СТРУКТУРНЫЙ. Фокус на «Отмена», Esc, переход ActDeleteModal -> попап и
// вид в WebView2/WKWebView проверяет пользователь по чек-листу плана 41.2-28.
//
// Правила:
//   (A) handleDelete: needs_group_restore проверяется ВЫШЕ первого pushToast('success'
//       и notifyPlaceContentChanged( (порядок по индексам); ветка не содержит
//       pushToast / notifyPlaceContentChanged / refresh(.
//   (B) каждый acts.delete( в ActsPage передаёт третий аргумент (выбор) и такой вызов
//       есть; <GroupRestoreChoicePopup в разметке стоит внутри {#if restorePrompt} и
//       не вложен в <ActDeleteModal.
//   (C) GroupRestoreChoicePopup презентационный (нет acts.delete(, нет импорта api),
//       initialFocus=".modal-footer .btn-secondary", обе фразы вариантов ровно по
//       одному разу, кнопки «Отмена» и «Удалить акт».
//   (D) api/acts.ts: delete принимает третий параметр и передаёт groupChoices.
//   (M) отсутствующий файл — нарушение, не SKIP.
//
// Usage:
//   node scripts/check-act-delete-choice.mjs              # проверить репозиторий
//   node scripts/check-act-delete-choice.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-act-delete-choice.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-delete-choice]';

const PAGE = 'src/features/acts/ActsPage.svelte';
const POPUP = 'src/features/acts/GroupRestoreChoicePopup.svelte';
const API = 'src/lib/api/acts.ts';
const FILES = [PAGE, POPUP, API];

const PHRASE_RESTORE = 'Вернуть в группу (на её новое место)';
const PHRASE_KEEP = 'Оставить вне группы на прежнем месте';

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

function countOf(text, needle) {
  return text.split(needle).length - 1;
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

/** Аргументы вызова, начиная с `acts.delete(` (текст между скобками). */
function deleteCalls(code) {
  const out = [];
  const re = /\bacts\.delete\s*\(/g;
  let m;
  while ((m = re.exec(code)) !== null) {
    out.push(extractBalanced(code, m.index + m[0].length - 1, '(', ')') ?? '');
  }
  return out;
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

/** @param {Record<string,string>} files rel -> текст */
export function scanSource(files, violations) {
  const s = {};
  for (const [rel, text] of Object.entries(files)) s[rel] = stripComments(text);
  for (const rel of FILES) {
    if (s[rel] === undefined) violations.push(`(M) ${rel}: файл отсутствует — нарушение, не SKIP.`);
  }

  const page = s[PAGE];
  if (page !== undefined) {
    const body = functionBody(page, 'handleDelete');
    if (body === null) {
      violations.push(`(A) ${PAGE}: не найдена функция handleDelete — антивакуум.`);
    } else {
      const iNeeds = body.indexOf('needs_group_restore');
      if (iNeeds === -1) {
        violations.push(
          `(A) ${PAGE}: handleDelete не обрабатывает needs_group_restore — сервер ничего не удалил, а UI сообщит «Акт удалён».`,
        );
      } else {
        const iSuccess = body.indexOf("pushToast(\n        'success'");
        const iSuccess2 = body.search(/pushToast\(\s*'success'/);
        const iNotify = body.indexOf('notifyPlaceContentChanged(');
        void iSuccess;
        if (iSuccess2 !== -1 && iNeeds > iSuccess2) {
          violations.push(
            `(A) ${PAGE}: проверка needs_group_restore стоит НИЖЕ success-тоста — тост «удалён» покажется до попапа.`,
          );
        }
        if (iNotify !== -1 && iNeeds > iNotify) {
          violations.push(
            `(A) ${PAGE}: проверка needs_group_restore стоит НИЖЕ notifyPlaceContentChanged( — места перерисуются без удаления.`,
          );
        }
        const ifIdx = body.lastIndexOf('if', iNeeds);
        const braceOpen = body.indexOf('{', iNeeds);
        const branch =
          ifIdx === -1 || braceOpen === -1 ? null : extractBalanced(body, braceOpen, '{', '}');
        if (branch === null) {
          violations.push(`(A) ${PAGE}: не удалось выделить ветку needs_group_restore.`);
        } else if (/pushToast\(|notifyPlaceContentChanged\(|\brefresh\(/.test(branch)) {
          violations.push(
            `(A) ${PAGE}: ветка needs_group_restore содержит pushToast/notifyPlaceContentChanged/refresh( — ничего не удалено.`,
          );
        }
      }
    }

    const calls = deleteCalls(page);
    if (calls.length === 0) {
      violations.push(`(B) ${PAGE}: нет вызова acts.delete( — антивакуум.`);
    }
    if (!calls.some((a) => topLevelArgCount(a) >= 3)) {
      violations.push(
        `(B) ${PAGE}: ни один acts.delete( не передаёт третий аргумент (выбор по группам).`,
      );
    }
    if (calls.some((a) => topLevelArgCount(a) < 3)) {
      violations.push(
        `(B) ${PAGE}: есть acts.delete( без третьего аргумента — повторный вызов потеряет выбор.`,
      );
    }
    const markup = markupOf(page);
    const tag = /<GroupRestoreChoicePopup\b/.exec(markup);
    if (tag === null) {
      violations.push(`(B) ${PAGE}: в разметке не смонтирован <GroupRestoreChoicePopup.`);
    } else {
      const cond = enclosingIfCondition(markup, tag.index);
      if (cond === null || !cond.includes('restorePrompt')) {
        violations.push(
          `(B) ${PAGE}: <GroupRestoreChoicePopup обязан стоять внутри {#if restorePrompt}.`,
        );
      }
      const dm = /<ActDeleteModal\b/.exec(markup);
      if (dm !== null) {
        const rest = markup.slice(dm.index);
        const end = rest.search(/\/>|<\/ActDeleteModal>/);
        const spanEnd = end === -1 ? markup.length : dm.index + end;
        if (tag.index > dm.index && tag.index < spanEnd) {
          violations.push(
            `(B) ${PAGE}: попап вложен в <ActDeleteModal — модалка над модалкой запрещена.`,
          );
        }
      }
    }
  }

  const popup = s[POPUP];
  if (popup !== undefined) {
    if (/\bacts\.delete\s*\(/.test(popup)) {
      violations.push(
        `(C) ${POPUP}: попап вызывает acts.delete( — он обязан быть презентационным.`,
      );
    }
    if (/from\s*['"][^'"]*\bapi\b[^'"]*['"]/.test(popup)) {
      violations.push(`(C) ${POPUP}: импорт из api — попап обязан быть презентационным.`);
    }
    if (!popup.includes('initialFocus=".modal-footer .btn-secondary"')) {
      violations.push(
        `(C) ${POPUP}: нет initialFocus=".modal-footer .btn-secondary" — фокус не на «Отмена».`,
      );
    }
    const flat = popup.replace(/\s+/g, ' ');
    for (const phrase of [PHRASE_RESTORE, PHRASE_KEEP]) {
      const n = countOf(flat, phrase);
      if (n !== 1)
        violations.push(`(C) ${POPUP}: фраза «${phrase}» встречается ${n} раз, нужно ровно 1.`);
    }
    for (const label of ['Отмена', 'Удалить акт']) {
      if (!new RegExp(`>\\s*${label}\\s*<`).test(popup)) {
        violations.push(`(C) ${POPUP}: нет кнопки «${label}».`);
      }
    }
  }

  const api = s[API];
  if (api !== undefined) {
    const m = /\bdelete:\s*\(/.exec(api);
    if (m === null) {
      violations.push(`(D) ${API}: не найден delete: — антивакуум.`);
    } else {
      const params = extractBalanced(api, m.index + m[0].length - 1, '(', ')') ?? '';
      if (topLevelArgCount(params) < 3) {
        violations.push(`(D) ${API}: delete не принимает третий параметр (groupChoices).`);
      }
      const tail = api.slice(m.index, m.index + 600);
      if (!tail.includes('groupChoices', tail.indexOf(')'))) {
        violations.push(`(D) ${API}: delete не передаёт groupChoices в apiCall.`);
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

  const pg = raw[PAGE];
  const branchRe = /\n\s*if \(result\.needs_group_restore\) \{[\s\S]*?\n\s*return;\n\s*\}\n/;
  const branchMatch = branchRe.exec(pg);
  let mutants;
  try {
    if (branchMatch === null) throw new Error('ветка needs_group_restore не найдена в ActsPage');
    const branchText = branchMatch[0];
    mutate(pg, branchText, '', 'probe branch');
    const successRe = /pushToast\(\s*'success'/;
    const iSuccess = pg.search(successRe);
    const withoutBranch = pg.replace(branchText, '');
    const swapped = withoutBranch.replace(successRe, (m) => `${branchText}\n      ${m}`);
    if (swapped === withoutBranch) throw new Error('перестановка не применилась');
    mutants = [
      {
        name: '(1) удалена ветка needs_group_restore',
        files: withFile(PAGE, withoutBranch),
        rule: 'A',
      },
      {
        name: '(2) ПЕРЕСТАНОВКА: проверка needs_group_restore ниже success-тоста',
        files: withFile(PAGE, swapped),
        rule: 'A',
      },
      {
        name: '(3) убран initialFocus в попапе',
        files: withFile(
          POPUP,
          mutate(
            raw[POPUP],
            '  initialFocus=".modal-footer .btn-secondary"\n',
            '',
            'drop initialFocus',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(4) acts.delete( внутри попапа',
        files: withFile(
          POPUP,
          mutate(
            raw[POPUP],
            '<script lang="ts">',
            '<script lang="ts">\n  const _x = () => acts.delete(1, 1, []);',
            'delete in popup',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(5) вызов удаления без groupChoices',
        files: withFile(
          PAGE,
          mutate(
            pg,
            'acts.delete(act.id, act.version, choices)',
            'acts.delete(act.id, act.version)',
            'drop third arg',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(6) ПОДМЕНА: фраза «оставить» заменена фразой «вернуть»',
        files: withFile(POPUP, mutate(raw[POPUP], PHRASE_KEEP, PHRASE_RESTORE, 'swap phrase')),
        rule: 'C',
      },
      {
        name: '(7) попап смонтирован под чужим {#if}',
        files: withFile(
          PAGE,
          mutate(pg, '{#if restorePrompt}', '{#if deleteTargetAct}', 'foreign if'),
        ),
        rule: 'B',
      },
      {
        name: '(8) api.delete без третьего параметра',
        files: withFile(
          API,
          mutate(
            raw[API],
            'delete: (id: number, version: number, groupChoices?: GroupRestoreChoiceDto[])',
            'delete: (id: number, version: number)',
            'api drop param',
          ),
        ),
        rule: 'D',
      },
    ];
    void iSuccess;
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
