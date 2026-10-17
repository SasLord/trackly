#!/usr/bin/env node
// [check-modal-isolation] Гейт изоляции типографики Modal (фаза 41.1, план 23;
// GRD-01, WR-03, GAP-2, GAP-3).
//
// Modal монтируется на месте вызова, то есть внутри `td.cell` (white-space: nowrap),
// `td.cell-actions` (text-align: center) и строки принтера (cursor: pointer). Эти
// свойства наследуются, поэтому корневое правило `.modal-backdrop` обязано их
// сбрасывать: `white-space: normal`, `text-align: start` (допустимо `left`),
// `cursor: auto`. Без сброса callout последствий выходит за край попапа, а заголовки
// полей в «Просмотре» выравниваются иначе, чем в «Редактировании».
//
// Правила (комментарии вырезаются: комментарий к сбросу, упоминающий
// «white-space: nowrap», иначе подделал бы объявление):
//   [M1] src/lib/components/Modal.svelte отсутствует, либо в его блоке
//        <style lang="scss"> нет верхнеуровневого правила с селектором ровно
//        `.modal-backdrop`. SKIP недопустим: отсутствие файла — нарушение;
//   [M2] в ЭТОМ правиле нет одного из трёх объявлений либо значение иное
//        (сброс, перенесённый в другое правило, тоже [M2]);
//   [M3] в разметке `class="modal-backdrop"` не содержит `class="modal-container`
//        (корень перестал быть обёрткой над контейнером).
//
// Self-test (`--selftest`): мутанты РЕАЛЬНОГО Modal.svelte (якорь каждого встречается
// ровно один раз, иначе replace молча отключил бы соседний фрагмент); у каждого
// сверяется ожидаемый код нарушения. Только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-modal-isolation.mjs               # проверить репозиторий
//   node scripts/check-modal-isolation.mjs --src=<root>  # КОРЕНЬ копии с каталогом src/ внутри
//   node scripts/check-modal-isolation.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-modal-isolation]';

const MODAL_FILE = 'src/lib/components/Modal.svelte';
const ROOT_SELECTOR = '.modal-backdrop';
// Обязательные объявления корневого правила: свойство -> допустимые значения.
const REQUIRED = [
  ['white-space', ['normal']],
  ['text-align', ['start', 'left']],
  ['cursor', ['auto']],
];

const blank = (m) => m.replace(/[^\n]/g, ' ');

export function stripComments(src) {
  return src
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[\s;{(,])\/\/[^\n]*/g, (m, pre) => pre + ' '.repeat(m.length - pre.length));
}

/** Индекс закрывающей `}` для `{` в text[openIdx]; -1 — не нашлось. */
function matchBrace(text, openIdx) {
  let depth = 0;
  for (let i = openIdx; i < text.length; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/** Верхнеуровневые правила SCSS: [{selector, body}]. At-правила пропускаются целиком. */
function topLevelRules(css) {
  const rules = [];
  let i = 0;
  while (i < css.length) {
    const open = css.indexOf('{', i);
    if (open === -1) break;
    const close = matchBrace(css, open);
    if (close === -1) break;
    const selector = css.slice(i, open).replace(/\s+/g, ' ').trim();
    if (!selector.startsWith('@')) rules.push({ selector, body: css.slice(open + 1, close) });
    i = close + 1;
  }
  return rules;
}

/** Объявления верхнего уровня тела правила (вложенные блоки вырезаются). */
function declarations(body) {
  let flat = '';
  let depth = 0;
  for (const c of body) {
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (depth === 0) flat += c;
  }
  const out = new Map();
  for (const part of flat.split(';')) {
    const idx = part.indexOf(':');
    if (idx === -1) continue;
    const prop = part.slice(0, idx).trim().toLowerCase();
    const value = part
      .slice(idx + 1)
      .trim()
      .toLowerCase();
    if (prop) out.set(prop, value);
  }
  return out;
}

/** Возвращает [{code, msg}]; пусто — всё в порядке. */
export function checkModalSource(source) {
  const v = [];
  const text = stripComments(source);
  const styleMatch = /<style\b[^>]*lang=["']scss["'][^>]*>([\s\S]*?)<\/style>/.exec(text);
  const markup = styleMatch ? text.replace(styleMatch[0], '') : text;

  const rule = styleMatch
    ? topLevelRules(styleMatch[1]).find((r) => r.selector === ROOT_SELECTOR)
    : undefined;
  if (!rule) {
    v.push({
      code: 'M1',
      msg: `в блоке <style lang="scss"> нет верхнеуровневого правила «${ROOT_SELECTOR}»`,
    });
  } else {
    const decl = declarations(rule.body);
    for (const [prop, allowed] of REQUIRED) {
      const value = decl.get(prop);
      if (value === undefined || !allowed.includes(value)) {
        v.push({
          code: 'M2',
          msg: `в «${ROOT_SELECTOR}» ${
            value === undefined ? `нет объявления ${prop}` : `${prop}: ${value}`
          }; ожидается ${prop}: ${allowed.join(' | ')}`,
        });
      }
    }
  }

  if (!/class="modal-backdrop"/.test(markup) || !/class="modal-container/.test(markup)) {
    v.push({
      code: 'M3',
      msg: 'в разметке нет class="modal-backdrop" либо class="modal-container" (корень перестал быть обёрткой)',
    });
  }
  return v;
}

/** Замена с проверкой, что якорь встречается ровно один раз. */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

const RESET_BLOCK = '    white-space: normal;\n    text-align: start;\n    cursor: auto;\n';
const FOOTER_ANCHOR = '    gap: var(--tr-space-xs);\n    flex-shrink: 0;\n';

const MUTANTS = [
  {
    name: 'удалена white-space: normal',
    apply: (s) => mutate(s, '    white-space: normal;\n', '', 'del white-space'),
    expect: 'M2',
  },
  {
    name: 'white-space: nowrap',
    apply: (s) => mutate(s, 'white-space: normal;', 'white-space: nowrap;', 'ws nowrap'),
    expect: 'M2',
  },
  {
    name: 'удалена text-align: start',
    apply: (s) => mutate(s, '    text-align: start;\n', '', 'del text-align'),
    expect: 'M2',
  },
  {
    name: 'text-align: center',
    apply: (s) => mutate(s, 'text-align: start;', 'text-align: center;', 'ta center'),
    expect: 'M2',
  },
  {
    name: 'удалена cursor: auto',
    apply: (s) => mutate(s, '    cursor: auto;\n', '', 'del cursor'),
    expect: 'M2',
  },
  {
    name: 'сброс перенесён из .modal-backdrop в .modal-footer',
    apply: (s) =>
      mutate(
        mutate(s, RESET_BLOCK, '', 'move: remove'),
        FOOTER_ANCHOR,
        FOOTER_ANCHOR + RESET_BLOCK,
        'move: insert',
      ),
    expect: 'M2',
  },
  {
    name: 'сброс закомментирован',
    apply: (s) => mutate(s, RESET_BLOCK, `    /*\n${RESET_BLOCK}    */\n`, 'comment out'),
    expect: 'M2',
  },
  {
    name: 'селектор .modal-backdrop переименован',
    apply: (s) => mutate(s, '  .modal-backdrop {\n', '  .modal-backdrop-x {\n', 'rename selector'),
    expect: 'M1',
  },
  {
    name: 'контейнер переименован в разметке',
    apply: (s) =>
      mutate(s, 'class="modal-container modal-{size}"', 'class="modal-box modal-{size}"', 'M3'),
    expect: 'M3',
  },
];

function selftest(root) {
  let source;
  try {
    source = fs.readFileSync(path.join(root, MODAL_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${MODAL_FILE}`);
    process.exit(1);
  }
  const base = checkModalSource(source);
  if (base.length > 0) {
    console.error(
      `${TAG} SELFTEST FAIL — реальный Modal.svelte красный: ${base.map((x) => `[${x.code}] ${x.msg}`).join('; ')}`,
    );
    process.exit(1);
  }
  let bad = 0;
  for (const m of MUTANTS) {
    const codes = checkModalSource(m.apply(source)).map((x) => x.code);
    if (!codes.includes(m.expect)) {
      bad++;
      console.error(
        `${TAG} SELFTEST FAIL — мутант «${m.name}»: ожидался [${m.expect}], получено [${codes.join(', ') || 'зелено'}]`,
      );
    }
  }
  if (bad > 0) process.exit(1);
  console.log(
    `${TAG} SELFTEST PASS — реальный файл зелёный, ${MUTANTS.length} мутантов исходника красные`,
  );
}

function main() {
  let root = UI_ROOT;
  let isSelftest = false;
  for (const a of process.argv.slice(2)) {
    if (a === '--selftest') isSelftest = true;
    else if (a.startsWith('--src=')) root = path.resolve(process.cwd(), a.slice(6));
  }
  if (isSelftest) return selftest(root);

  let source;
  try {
    source = fs.readFileSync(path.join(root, MODAL_FILE), 'utf8');
  } catch {
    console.error(`${TAG} FAIL — [M1] файла ${MODAL_FILE} нет в ${root}`);
    process.exit(1);
  }
  const violations = checkModalSource(source);
  if (violations.length > 0) {
    for (const x of violations) console.error(`${TAG} FAIL — [${x.code}] ${x.msg}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — 0 нарушений`);
}

main();
