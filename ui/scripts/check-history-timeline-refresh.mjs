#!/usr/bin/env node
// [check-history-timeline-refresh] Каждый .svelte, вызывающий history_get_timeline,
// обязан (а) перечитывать ленту по событию другого клиента
// (placeContentEventsStore.reloadSeq) и (б) защищаться от устаревшего ответа
// (флаг cancelled или сравнение токена). Урок R8-01/R8-04: новая лента без
// подписки не перечитывается у второго клиента.
//
// Инвентарь строится ОТ репозитория: обход ui/src, не от списка имён.
//
// Usage:
//   node scripts/check-history-timeline-refresh.mjs              # репозиторий
//   node scripts/check-history-timeline-refresh.mjs --src=<dir>  # копия ui/
//   node scripts/check-history-timeline-refresh.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';
import { walk } from './_roots.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-history-timeline-refresh]';

const NEEDLE = 'history_get_timeline';

// Файл -> причина: подписка приходит пропом, а не напрямую из стора.
const PROP_TOKEN = {
  'src/features/groups/GroupPanel.svelte':
    'принимает проп refreshToken, в GroupsPage связан с reloadSeq (D-17 41.7)',
};
const PARENT_FILE = 'src/features/groups/GroupsPage.svelte';
// Файл -> причина освобождения (изначально пусто).
const EXEMPT = {};

/** Тела всех $effect(...) в тексте. */
function effectBodies(c) {
  const out = [];
  for (const m of c.matchAll(/\$effect(?:\.pre)?\s*\(/g)) {
    const open = m.index + m[0].length - 1;
    let depth = 0;
    for (let i = open; i < c.length; i++) {
      if (c[i] === '(') depth++;
      else if (c[i] === ')' && --depth === 0) {
        out.push(c.slice(open + 1, i));
        break;
      }
    }
  }
  return out;
}

/** files: { relPath: text } -> нарушения. */
export function scan(files) {
  const v = [];
  const code = {};
  for (const [f, t] of Object.entries(files)) code[f] = stripComments(t);
  const inventory = Object.keys(code).filter((f) => code[f].includes(NEEDLE));
  if (inventory.length === 0) {
    v.push('инвентарь пуст: ни один .svelte не вызывает history_get_timeline (вакуум)');
    return { v, checked: 0 };
  }
  for (const [f, reason] of Object.entries(PROP_TOKEN)) {
    if (!reason || reason.trim() === '') v.push(`${f}: allowlist без причины`);
    if (!(f in code)) v.push(`${f}: запись allowlist указывает на несуществующий файл`);
  }
  for (const [f, reason] of Object.entries(EXEMPT)) {
    if (!reason || reason.trim() === '') v.push(`${f}: освобождение без причины`);
    if (!(f in code)) v.push(`${f}: освобождение указывает на несуществующий файл`);
  }
  let checked = 0;
  for (const f of inventory) {
    if (f in EXEMPT) continue;
    checked++;
    const c = code[f];
    if (f in PROP_TOKEN) {
      if (!/void\s+refreshToken\b/.test(c))
        v.push(`${f}: нет «void refreshToken» (подписка через проп)`);
      const p = code[PARENT_FILE];
      if (p === undefined) v.push(`${PARENT_FILE}: родитель не найден`);
      else if (!/\breloadSeq\b/.test(p) || !/panelRefreshToken\s*\+=/.test(p))
        v.push(`${PARENT_FILE}: не связывает reloadSeq с panelRefreshToken`);
    } else {
      if (!effectBodies(c).some((b) => /placeContentEventsStore\s*\.\s*reloadSeq/.test(b)))
        v.push(`${f}: лента без чтения placeContentEventsStore.reloadSeq внутри $effect`);
      else if (!/\buntrack\s*\(/.test(c) && !/\bcancelled\b/.test(c))
        v.push(`${f}: подписка без untrack и без cancelled`);
    }
    const guarded =
      (/\blet\s+cancelled\b/.test(c) && /!\s*cancelled\b|if\s*\(\s*cancelled\b/.test(c)) ||
      /token\s*[!=]==?\s*\w*[tT]oken/.test(c);
    if (!guarded) v.push(`${f}: нет защиты от устаревшего ответа (cancelled / токен)`);
  }
  return { v, checked };
}

function readAll(root) {
  const base = path.join(root, 'src');
  const out = {};
  const files = walk({
    exts: ['.svelte'],
    roots: path.resolve(root) === UI_ROOT ? null : [base],
  });
  for (const full of files) {
    out[path.relative(root, full).split(path.sep).join('/')] = fs.readFileSync(full, 'utf8');
  }
  return out;
}

function runSelfTest() {
  const good = `<script>
  let cancelled = false;
  $effect(() => { void placeContentEventsStore.reloadSeq; apiCall('history_get_timeline'); if (!cancelled) {} return () => { cancelled = true; }; });
</script>`;
  const cases = [
    ['файл с подпиской и cancelled', { 'src/a.svelte': good }, true],
    [
      'reloadSeq вне $effect',
      {
        'src/a.svelte': good
          .replace('void placeContentEventsStore.reloadSeq;', '')
          .replace('$effect(', 'let l0 = placeContentEventsStore.reloadSeq; $effect('),
      },
      false,
    ],
    [
      'без reloadSeq',
      { 'src/a.svelte': good.replace('placeContentEventsStore.reloadSeq', 'x') },
      false,
    ],
    [
      'закомментированный reloadSeq',
      {
        'src/a.svelte': good.replace(
          'void placeContentEventsStore.reloadSeq;',
          '// placeContentEventsStore.reloadSeq',
        ),
      },
      false,
    ],
    ['без защиты', { 'src/a.svelte': good.replace('if (!cancelled) {}', '') }, false],
    ['пустой инвентарь', { 'src/a.svelte': '<script></script>' }, false],
  ];
  let failed = 0;
  for (const [name, files, ok] of cases) {
    // PROP_TOKEN-записи в фикстурах отсутствуют; игнорируем их нарушения
    const { v } = scan(files);
    const own = v.filter((x) => !x.includes('GroupPanel') && !x.includes('allowlist'));
    const pass = ok ? own.length === 0 : own.length > 0;
    console.error(`${TAG} [selftest] ${pass ? 'PASS' : 'FAIL'} — ${name}`);
    if (!pass) failed++;
  }
  // allowlist без причины
  const saved = PROP_TOKEN['src/features/groups/GroupPanel.svelte'];
  PROP_TOKEN['src/features/groups/GroupPanel.svelte'] = '';
  const { v } = scan({ 'src/a.svelte': good, 'src/features/groups/GroupPanel.svelte': good });
  PROP_TOKEN['src/features/groups/GroupPanel.svelte'] = saved;
  const pass = v.some((x) => x.includes('без причины'));
  console.error(`${TAG} [selftest] ${pass ? 'PASS' : 'FAIL'} — allowlist без причины`);
  if (!pass) failed++;
  if (failed) {
    console.error(`${TAG} [selftest] FAIL — ${failed}`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — ${cases.length + 1} случаев.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--selftest')) process.exit(runSelfTest() ? 0 : 1);
  const srcArg = args.find((a) => a.startsWith('--src='));
  const root = srcArg ? path.resolve(srcArg.slice('--src='.length)) : UI_ROOT;
  const { v, checked } = scan(readAll(root));
  if (v.length > 0) {
    console.error(`${TAG} FAIL — ${v.length} нарушений:`);
    for (const x of v) console.error(`${TAG}   ${x}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — проверено файлов с лентой: ${checked}.`);
}

main();
