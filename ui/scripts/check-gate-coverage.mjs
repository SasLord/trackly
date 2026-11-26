#!/usr/bin/env node
// [check-gate-coverage] Гейт потери покрытия при переезде фронтенда (фаза 42, план 01;
// LNK-01, D-20).
//
// ЗАЧЕМ: при переезде файлов в `packages/*` гейты обходят меньше файлов, но не 0, и
// остаются зелёными. Baseline набора файлов (fixtures/gate-coverage.baseline.json) снят
// ДО переездов; каждый переезд обязан быть записан в fixtures/move-map.json.
//
// Правила:
//   [C0] baseline непуст и скан (scanRoots()+walk()) непуст — вакуумный PASS запрещён;
//   [C1] каждый путь baseline, пропущенный через move-map (from -> to), существует;
//   [C2] для каждой записи move-map `from` отсутствует, а `to` существует;
//   [C3] файлов в скане не меньше, чем файлов baseline, лежащих под сканируемыми
//        корнями (ui/public в скан не входит и в счёт не берётся).
// SKIP недопустим.
//
// Чего гейт НЕ проверяет: per-gate счётчики (их добавит план 07) и смысловую потерю
// правил внутри файла.
//
// Usage:
//   node scripts/check-gate-coverage.mjs
//   node scripts/check-gate-coverage.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { REPO, UI_ROOT, walk } from './_roots.mjs';

const TAG = '[check-gate-coverage]';
const FIX = path.join(UI_ROOT, 'scripts', 'fixtures');

/**
 * Чистая проверка на данных. exists(relPath) -> boolean; scanCount — число файлов скана.
 * Возвращает массив нарушений с кодами [C0]..[C3].
 */
export function evaluate({ baseline, moves, exists, scanCount }) {
  const failures = [];
  const files = baseline?.files ?? [];
  if (files.length === 0) failures.push('[C0] baseline пуст');
  if (!(scanCount > 0)) failures.push('[C0] скан дал 0 файлов — гейт сломан');

  const moveFrom = new Map(moves.map((m) => [m.from, m.to]));
  for (const f of files) {
    const target = moveFrom.has(f) ? moveFrom.get(f) : f;
    if (!exists(target)) failures.push(`[C1] файл baseline потерян: ${f} -> ${target}`);
  }
  for (const m of moves) {
    if (exists(m.from)) failures.push(`[C2] from ещё существует: ${m.from}`);
    if (!exists(m.to)) failures.push(`[C2] to не существует: ${m.to}`);
  }
  const scanned = files.filter((f) => !f.startsWith('ui/public/')).length;
  if (scanCount < scanned) {
    failures.push(`[C3] в скане ${scanCount} файлов, в baseline под корнями ${scanned}`);
  }
  return failures;
}

function loadJson(name) {
  return JSON.parse(fs.readFileSync(path.join(FIX, name), 'utf8'));
}

function selftest() {
  const baseline = { files: ['ui/src/a.ts', 'ui/src/b.svelte'] };
  const all = new Set(baseline.files);
  const ex = (set) => (p) => set.has(p);
  const cases = [
    ['ok', { baseline, moves: [], exists: ex(all), scanCount: 2 }, null],
    [
      'потерян файл без move-map',
      { baseline, moves: [], exists: ex(new Set(['ui/src/a.ts'])), scanCount: 2 },
      '[C1]',
    ],
    [
      'from ещё существует',
      {
        baseline,
        moves: [{ from: 'ui/src/a.ts', to: 'packages/x/src/a.ts' }],
        exists: ex(new Set([...all, 'packages/x/src/a.ts'])),
        scanCount: 2,
      },
      '[C2]',
    ],
    [
      'to не существует',
      {
        baseline,
        moves: [{ from: 'ui/src/a.ts', to: 'packages/x/src/a.ts' }],
        exists: ex(new Set(['ui/src/b.svelte'])),
        scanCount: 2,
      },
      '[C2]',
    ],
    [
      'пустой baseline',
      { baseline: { files: [] }, moves: [], exists: ex(all), scanCount: 2 },
      '[C0]',
    ],
    ['пустой скан', { baseline, moves: [], exists: ex(all), scanCount: 0 }, '[C0]'],
  ];
  let bad = 0;
  for (const [name, input, code] of cases) {
    const failures = evaluate(input);
    const ok = code === null ? failures.length === 0 : failures.some((f) => f.startsWith(code));
    console.log(`${TAG} selftest ${ok ? 'ok' : 'FAIL'}: ${name}${code ? ` (ждали ${code})` : ''}`);
    if (!ok) bad++;
  }
  if (bad) {
    console.error(`${TAG} FAIL: selftest, расхождений: ${bad}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest PASS (${cases.length} случаев)`);
}

function main() {
  if (process.argv.includes('--selftest')) return selftest();
  const baseline = loadJson('gate-coverage.baseline.json');
  const moves = loadJson('move-map.json').moves ?? [];
  const scanCount = walk().length;
  const failures = evaluate({
    baseline,
    moves,
    exists: (p) => fs.existsSync(path.join(REPO, p)),
    scanCount,
  });
  if (failures.length) {
    for (const f of failures) console.error(`${TAG} ${f}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS: в скане ${scanCount} файлов, записей move-map ${moves.length}`);
}

main();
