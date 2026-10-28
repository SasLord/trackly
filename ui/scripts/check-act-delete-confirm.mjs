#!/usr/bin/env node
// [check-act-delete-confirm] Структурный гейт подтверждения удаления акта
// (ActsPage.svelte, ActDeleteModal.svelte, ActDetail.svelte; задача 261028-fw6).
//
// Почему он существует: нативный диалог браузера (confirm) в десктопном webview
// до пользователя не доходит, акт удалялся без подтверждения, а предупреждение о
// последствиях (в том числе «вернутся в свои группы») оставалось невидимым.
// Подтверждение переехало в собственное модальное окно ActDeleteModal.
// svelte-check / eslint / build не видят, что кто-то вернул нативный вызов или
// «временно» подключил кнопку «Удалить» к handleDelete напрямую, — приложение
// собирается и молча удаляет без вопроса.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он не доказывает, что окно открывается, фокус стоит
// на «Отмена», а Esc отменяет, в живом десктопном webview, — это живая проверка.
//
// Правила (над текстом без комментариев, все .svelte/.ts под src/features/acts/):
//   (A) нигде нет вызова нативного confirm(…) (window./globalThis./self. или голый);
//       onConfirm( и confirmDelete( не считаются.
//   (B) ActsPage: импорт ActDeleteModal; в РАЗМЕТКЕ (вне <script>) есть
//       <ActDeleteModal, и ближайший незакрытый {#if} перед ним (стек, не счёт)
//       содержит deleteTargetAct; в разметке нет handleDelete (иначе подтверждение
//       обойдено); тело confirmDelete вызывает handleDelete(; ActDetail получает
//       onDelete={requestDelete}.
//   (C) ActDeleteModal: после схлопывания пробелов содержит КАЖДОЕ из двух целых
//       предложений про возврат устройств в группы РОВНО по одному разу (общий
//       счёт подстроки перенос фразы из ветки в ветку не заметил бы).
//   (D) ActDeleteModal презентационный: нет acts.delete( и нет импорта из api.
//   (E) ActDeleteModal содержит initialFocus=".modal-footer .btn-secondary".
//   (M) ActsPage.svelte / ActDeleteModal.svelte отсутствуют либо каталог пуст —
//       нарушение, а не SKIP (гейт, пропускающий файл молча, нем навсегда).
//
// Self-test (`--selftest`): чистые РЕАЛЬНЫЕ файлы проходят; мутанты по реальному
// исходнику через mutate() с проверкой ровно одного вхождения якоря; порядок и
// принадлежность фразы защищены мутантами-ПОДМЕНАМИ, а не только удалением.
//
// Zero-dependency: node:fs/node:os/node:path/node:url.
//
// Usage:
//   node scripts/check-act-delete-confirm.mjs              # проверить репозиторий
//   node scripts/check-act-delete-confirm.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-act-delete-confirm.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-delete-confirm]';

const ACTS_DIR = 'src/features/acts';
const PAGE = `${ACTS_DIR}/ActsPage.svelte`;
const MODAL = `${ACTS_DIR}/ActDeleteModal.svelte`;
const DETAIL = `${ACTS_DIR}/ActDetail.svelte`;

const SENTENCE_ACT = 'Устройства, выведенные этим актом из состава групп, вернутся в свои группы.';
const SENTENCE_RETURN =
  'Устройства, выведенные этим возвратом из состава групп, вернутся в свои группы.';

/** Содержимое парных скобок, начиная с индекса открывающей. */
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

/** Тело `[async] function name(...) { ... }` или null. */
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

/** Разметка без блоков <script> и <style> (длина не сохраняется, индексы не нужны). */
function markupOf(code) {
  return code.replace(/<script\b[\s\S]*?<\/script>/g, '').replace(/<style\b[\s\S]*?<\/style>/g, '');
}

/** Ближайший незакрытый {#if} перед позицией pos в разметке: его условие или null. */
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

/**
 * @param {Record<string,string>} files rel -> текст всех .svelte/.ts под src/features/acts/
 * @param {string[]} violations
 */
export function scanSource(files, violations) {
  const stripped = {};
  for (const [rel, text] of Object.entries(files)) stripped[rel] = stripComments(text);

  if (Object.keys(stripped).length === 0) {
    violations.push(`(M) ${ACTS_DIR}: каталог пуст или не читается — нарушение, не SKIP.`);
  }

  // (A) нативный confirm нигде в каталоге.
  const nativeConfirm = /(?<![\w$])(?:window\.|globalThis\.|self\.)?confirm\s*\(/;
  for (const [rel, code] of Object.entries(stripped)) {
    if (nativeConfirm.test(code)) {
      violations.push(
        `(A) ${rel}: найден вызов нативного confirm( — в десктопном webview он не доходит до пользователя; подтверждение только через ActDeleteModal.`,
      );
    }
  }

  // (B) ActsPage.
  const page = stripped[PAGE];
  if (page === undefined) {
    violations.push(`(M) ${PAGE}: файл отсутствует — нарушение, не SKIP.`);
  } else {
    if (!/import\s+ActDeleteModal\s+from\s+['"]\.\/ActDeleteModal\.svelte['"]/.test(page)) {
      violations.push(`(B) ${PAGE}: нет импорта ActDeleteModal из './ActDeleteModal.svelte'.`);
    }
    const markup = markupOf(page);
    const tag = /<ActDeleteModal\b/.exec(markup);
    if (tag === null) {
      violations.push(`(B) ${PAGE}: в разметке не смонтирован <ActDeleteModal — окна нет.`);
    } else {
      const cond = enclosingIfCondition(markup, tag.index);
      if (cond === null || !cond.includes('deleteTargetAct')) {
        violations.push(
          `(B) ${PAGE}: <ActDeleteModal обязан стоять внутри {#if deleteTargetAct} (ближайший незакрытый {#if}: ${cond === null ? 'нет' : `{#if${cond}}`}) — иначе окно открыто без цели.`,
        );
      }
    }
    if (/\bhandleDelete\b/.test(markup)) {
      violations.push(
        `(B) ${PAGE}: handleDelete упомянут в разметке — подтверждение обойдено; в разметке допустим только requestDelete.`,
      );
    }
    if (!/onDelete=\{requestDelete\}/.test(markup)) {
      violations.push(`(B) ${PAGE}: ActDetail обязан получать onDelete={requestDelete}.`);
    }
    const confirmBody = functionBody(page, 'confirmDelete');
    if (confirmBody === null) {
      violations.push(`(B) ${PAGE}: не найдена функция confirmDelete — антивакуум.`);
    } else if (!confirmBody.includes('handleDelete(')) {
      violations.push(
        `(B) ${PAGE}: confirmDelete не вызывает handleDelete( — удаление не происходит.`,
      );
    }
  }

  // (C), (D), (E) ActDeleteModal.
  const modalRaw = stripped[MODAL];
  if (modalRaw === undefined) {
    violations.push(`(M) ${MODAL}: файл отсутствует — нарушение, не SKIP.`);
  } else {
    const flat = modalRaw.replace(/\s+/g, ' ');
    for (const [label, sentence] of [
      ['акт', SENTENCE_ACT],
      ['возврат', SENTENCE_RETURN],
    ]) {
      const n = countOf(flat, sentence);
      if (n !== 1) {
        violations.push(
          `(C) ${MODAL}: предложение ветки «${label}» («${sentence}») встречается ${n} раз, нужно ровно 1.`,
        );
      }
    }
    if (/\bacts\.delete\s*\(/.test(modalRaw)) {
      violations.push(
        `(D) ${MODAL}: окно вызывает acts.delete( — оно обязано быть презентационным.`,
      );
    }
    if (/from\s*['"][^'"]*\bapi\b[^'"]*['"]/.test(modalRaw)) {
      violations.push(`(D) ${MODAL}: импорт из api — окно обязано быть презентационным.`);
    }
    if (!modalRaw.includes('initialFocus=".modal-footer .btn-secondary"')) {
      violations.push(
        `(E) ${MODAL}: нет initialFocus=".modal-footer .btn-secondary" — фокус не на «Отмена».`,
      );
    }
  }
}

/** Рекурсивно читает .svelte/.ts под root/src/features/acts. Ключи — относительные пути. */
function readSources(root) {
  const out = {};
  const base = path.join(root, ACTS_DIR);
  const walk = (dir) => {
    let entries;
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const abs = path.join(dir, e.name);
      if (e.isDirectory()) walk(abs);
      else if (/\.(svelte|ts)$/.test(e.name)) {
        const rel = path.relative(root, abs).split(path.sep).join('/');
        out[rel] = fs.readFileSync(abs, 'utf8');
      }
    }
  };
  walk(base);
  return out;
}

/** Замена с проверкой, что якорь встречается ровно один раз. */
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
  const without = (rel) => {
    const copy = { ...raw };
    delete copy[rel];
    return copy;
  };

  let mutants;
  try {
    mutants = [
      {
        name: '(1) нативный confirm вернулся в handleDelete',
        files: withFile(
          PAGE,
          mutate(
            raw[PAGE],
            'async function handleDelete(act: ActDto) {',
            "async function handleDelete(act: ActDto) {\n    if (!window.confirm('x')) return;",
            'confirm in handleDelete',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(2) голый confirm( в ActDetail',
        files: withFile(
          DETAIL,
          mutate(
            raw[DETAIL],
            'onclick={() => onDelete(act)}',
            "onclick={() => confirm('x') && onDelete(act)}",
            'bare confirm in ActDetail',
          ),
        ),
        rule: 'A',
      },
      {
        name: '(3) <ActDeleteModal в разметке заменён другим тегом',
        files: withFile(
          PAGE,
          mutate(
            raw[PAGE],
            '<ActDeleteModal\n    act={deleteTargetAct}',
            '<SomethingElse\n    act={deleteTargetAct}',
            'replace tag',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(4) убран открывающий {#if deleteTargetAct} (висячий {/if})',
        files: withFile(PAGE, mutate(raw[PAGE], '{#if deleteTargetAct}\n', '', 'drop opening if')),
        rule: 'B',
      },
      {
        name: '(5) ПОДМЕНА: onDelete={requestDelete} -> onDelete={handleDelete}',
        files: withFile(
          PAGE,
          mutate(
            raw[PAGE],
            'onDelete={requestDelete}',
            'onDelete={handleDelete}',
            'bypass confirmation',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(6) стёрто предложение ветки «актом»',
        files: withFile(MODAL, mutate(raw[MODAL], `${SENTENCE_ACT} `, '', 'drop act sentence')),
        rule: 'C',
      },
      {
        name: '(7) стёрто предложение ветки «возвратом»',
        files: withFile(
          MODAL,
          mutate(raw[MODAL], ` ${SENTENCE_RETURN}`, '', 'drop return sentence'),
        ),
        rule: 'C',
      },
      {
        name: '(8) ПОДМЕНА: предложение «актом» заменено предложением «возвратом»',
        files: withFile(
          MODAL,
          mutate(raw[MODAL], SENTENCE_ACT, SENTENCE_RETURN, 'swap act->return sentence'),
        ),
        rule: 'C',
      },
      {
        name: '(9) окно импортирует api',
        files: withFile(
          MODAL,
          mutate(
            raw[MODAL],
            "import Button from '$lib/components/Button.svelte';",
            "import Button from '$lib/components/Button.svelte';\n  import { acts } from './api';",
            'import api',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(10) убран initialFocus',
        files: withFile(
          MODAL,
          mutate(
            raw[MODAL],
            '  initialFocus=".modal-footer .btn-secondary"\n',
            '',
            'drop initialFocus',
          ),
        ),
        rule: 'E',
      },
      {
        name: '(11) ActDeleteModal.svelte отсутствует в карте файлов',
        files: without(MODAL),
        rule: 'M',
      },
      {
        name: '(12) ПОДМЕНА: confirmDelete зовёт acts.delete( мимо handleDelete',
        files: withFile(
          PAGE,
          mutate(
            raw[PAGE],
            'await handleDelete(act);',
            'await acts.delete(act.id, act.version);',
            'confirmDelete bypass',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(13) ПОДМЕНА: окно под чужим {#if pdfModalOpen}',
        files: withFile(
          PAGE,
          mutate(raw[PAGE], '{#if deleteTargetAct}\n', '{#if pdfModalOpen}\n', 'foreign if'),
        ),
        rule: 'B',
      },
      {
        name: '(14) ActsPage.svelte отсутствует в карте файлов',
        files: without(PAGE),
        rule: 'M',
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  let caught = 0;
  for (const m of mutants) {
    const v = [];
    scanSource(m.files, v);
    const hit = v.some((line) => line.startsWith(`(${m.rule})`));
    if (hit) {
      caught++;
      console.error(`${TAG} [selftest] PASS — ${m.name} (поймано правилом ${m.rule})`);
    } else {
      failed++;
      console.error(
        `${TAG} [selftest] FAIL — ${m.name}: гейт не покраснел правилом ${m.rule} (нарушения: ${v.length ? v.join(' | ') : 'нет'})`,
      );
    }
  }

  // Пустой каталог (временный корень без src/) — нарушение, не SKIP.
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'check-act-delete-confirm-'));
  try {
    const v = [];
    scanSource(readSources(tmp), v);
    if (v.some((line) => line.startsWith('(M)'))) {
      caught++;
      console.error(`${TAG} [selftest] PASS — пустой каталог — нарушение, не SKIP`);
    } else {
      failed++;
      console.error(`${TAG} [selftest] FAIL — пустой каталог не дал нарушения.`);
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — реальные исходники чисты, мутантов поймано: ${caught}.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-act-delete-confirm.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
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
