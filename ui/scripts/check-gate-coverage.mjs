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
//        корнями (ui/public в скан не входит и в счёт не берётся);
//   [C4] множество check-*.mjs, импортирующих _roots.mjs (кроме этого гейта), равно
//        ключам baseline.perGate — потерянный/неподключённый обходчик краснит;
//   [C5] каждый обходчик запущен с GATE_COVERAGE_REPORT: count > 0 и >= perGate[имя];
//   [C6] ни один check-*.mjs не обходит ui/src локально (readdirSync + путь src) в
//        обход _roots.mjs; исключены только гейты из baseline.pinned (см. [C7]) и
//        check-lint-selftest-wiring (листает scripts/);
//   [C7] гейты вне walk() (baseline.pinned): каждый именованный файл существует по
//        ТОМУ ЖЕ пути, который читает гейт, а число файлов в его каталоге не меньше
//        замороженного. Гейт, у которого переехал вход, краснеет здесь: план, что
//        переносит вход, обязан в том же коммите перенацелить гейт и осознанно
//        обновить pinned (виден в git diff).
// SKIP недопустим.
//
// Чего гейт НЕ проверяет: смысловую потерю правил внутри файла.
//
// Usage:
//   node scripts/check-gate-coverage.mjs
//   node scripts/check-gate-coverage.mjs --selftest

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { REPO, UI_ROOT, walk } from './_roots.mjs';
import { stripComments } from './_strip.mjs';

const TAG = '[check-gate-coverage]';
const FIX = path.join(UI_ROOT, 'scripts', 'fixtures');

/**
 * Чистая проверка на данных. exists(relPath) -> boolean; scanCount — число файлов скана.
 * Возвращает массив нарушений с кодами [C0]..[C3].
 */
export function evaluate({
  baseline,
  moves,
  exists,
  scanCount,
  walkers = null,
  counts = null,
  localWalkers = [],
  dirCount = null,
}) {
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
  const perGate = baseline?.perGate ?? {};
  if (walkers) {
    const keys = Object.keys(perGate);
    if (walkers.length === 0 || keys.length === 0) failures.push('[C4] инвентарь обходчиков пуст');
    for (const w of walkers) {
      if (!(w in perGate)) failures.push(`[C4] обходчик без ключа perGate: ${w}`);
    }
    for (const k of keys) {
      if (!walkers.includes(k)) failures.push(`[C4] ключ perGate без обходчика: ${k}`);
    }
  }
  if (counts) {
    for (const [g, base] of Object.entries(perGate)) {
      const c = counts[g];
      if (!(c > 0)) failures.push(`[C5] ${g}: count ${c ?? 'нет отчёта'} (нужно > 0)`);
      else if (c < base) failures.push(`[C5] ${g}: count ${c} < baseline ${base}`);
    }
  }
  for (const g of localWalkers) {
    failures.push(`[C6] ${g}: локальный обход src в обход _roots.mjs`);
  }
  if (dirCount) {
    for (const [g, spec] of Object.entries(baseline?.pinned ?? {})) {
      for (const f of spec.files ?? []) {
        if (!exists(f)) failures.push(`[C7] ${g}: входной файл потерян: ${f}`);
      }
      for (const d of spec.dirs ?? []) {
        const c = dirCount(d);
        if (!(c > 0) || c < d.count) {
          failures.push(`[C7] ${g}: ${d.dir} — файлов ${c}, было ${d.count}`);
        }
      }
    }
  }
  return failures;
}

const LOCAL_WALK_EXEMPT = new Set(['check-gate-coverage.mjs', 'check-lint-selftest-wiring.mjs']);

/** Гейты, листающие src локально: readdirSync + путь src, без импорта _roots.mjs. */
export function findLocalWalkers(scriptsDir, pinnedGates) {
  const out = [];
  for (const f of fs.readdirSync(scriptsDir).sort()) {
    if (!/^check-.*\.mjs$/.test(f) || LOCAL_WALK_EXEMPT.has(f)) continue;
    const name = f.replace(/^check-/, '').replace(/\.mjs$/, '');
    if (pinnedGates.includes(name)) continue;
    const raw = fs.readFileSync(path.join(scriptsDir, f), 'utf8');
    if (raw.includes('_roots.mjs')) continue;
    const code = stripComments(raw);
    if (/readdirSync/.test(code) && /['"`](?:\.\.\/)?src['"`/]/.test(code)) out.push(f);
  }
  return out;
}

/** Инвентарь обходчиков от каталога. */
export function findWalkers(scriptsDir) {
  return fs
    .readdirSync(scriptsDir)
    .filter((f) => /^check-.*\.mjs$/.test(f) && f !== 'check-gate-coverage.mjs')
    .filter((f) => fs.readFileSync(path.join(scriptsDir, f), 'utf8').includes('_roots.mjs'))
    .map((f) => f.replace(/^check-/, '').replace(/\.mjs$/, ''))
    .sort();
}

/** Запуск обходчиков с GATE_COVERAGE_REPORT; возвращает { counts, errors }. */
export function runWalkers(names, cwd) {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'gate-cov-'));
  const report = path.join(tmpDir, 'r.jsonl');
  const counts = {};
  const errors = [];
  try {
    for (const n of names) {
      fs.rmSync(report, { force: true });
      const r = spawnSync('node', [`scripts/check-${n}.mjs`], {
        cwd,
        env: { ...process.env, GATE_COVERAGE_REPORT: report },
        encoding: 'utf8',
        timeout: 120000,
      });
      if (r.status !== 0) errors.push(`[C5] ${n}: процесс завершился кодом ${r.status}`);
      const lines = fs.existsSync(report)
        ? fs.readFileSync(report, 'utf8').split('\n').filter(Boolean)
        : [];
      counts[n] = Math.max(0, ...lines.map((l) => JSON.parse(l).count));
    }
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
  return { counts, errors };
}

function countDir(d) {
  const re = new RegExp(d.pattern);
  let n = 0;
  const rec = (rel) => {
    const abs = path.join(REPO, rel);
    if (!fs.existsSync(abs)) return;
    for (const ent of fs.readdirSync(abs, { withFileTypes: true })) {
      if (ent.isDirectory()) {
        if (d.recursive && ent.name !== 'node_modules' && ent.name !== 'dist') {
          rec(rel + '/' + ent.name);
        }
      } else if (re.test(ent.name)) n++;
    }
  };
  rec(d.dir);
  return n;
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
  // Мутанты [C4]-[C7] на данных.
  const pb = {
    files: baseline.files,
    perGate: { a: 10, b: 5 },
    pinned: {
      p: {
        files: ['ui/src/a.ts'],
        dirs: [{ dir: 'ui/src', pattern: '.', recursive: false, count: 2 }],
      },
    },
  };
  const base7 = {
    baseline: pb,
    moves: [],
    exists: ex(all),
    scanCount: 2,
    walkers: ['a', 'b'],
    counts: { a: 10, b: 6 },
    localWalkers: [],
    dirCount: () => 2,
  };
  cases.push(
    ['C4-C7 ok', base7, null],
    ['обходчик без ключа', { ...base7, walkers: ['a', 'b', 'c'] }, '[C4]'],
    ['ключ без обходчика', { ...base7, walkers: ['a'] }, '[C4]'],
    ['пустой инвентарь обходчиков', { ...base7, walkers: [] }, '[C4]'],
    ['count ниже baseline', { ...base7, counts: { a: 9, b: 6 } }, '[C5]'],
    ['count 0', { ...base7, counts: { a: 0, b: 6 } }, '[C5]'],
    ['нет отчёта', { ...base7, counts: { a: 10 } }, '[C5]'],
    ['локальный обход', { ...base7, localWalkers: ['check-zz.mjs'] }, '[C6]'],
    ['pinned: файл потерян', { ...base7, exists: ex(new Set(['ui/src/b.svelte'])) }, '[C7]'],
    ['pinned: каталог усох', { ...base7, dirCount: () => 1 }, '[C7]'],
  );
  let bad = 0;
  for (const [name, input, code] of cases) {
    const failures = evaluate(input);
    const ok = code === null ? failures.length === 0 : failures.some((f) => f.startsWith(code));
    console.log(`${TAG} selftest ${ok ? 'ok' : 'FAIL'}: ${name}${code ? ` (ждали ${code})` : ''}`);
    if (!ok) bad++;
  }
  // Мутант [C6] на временном каталоге (якорь — ровно одно вхождение readdirSync).
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'gate-cov-c6-'));
  try {
    const gate = "import fs from 'node:fs';\nfs.readdirSync(path.join(UI_ROOT, 'src'));\n";
    fs.writeFileSync(path.join(tmp, 'check-zz-new.mjs'), gate);
    fs.writeFileSync(path.join(tmp, 'check-zz-ok.mjs'), "import './_roots.mjs';\n" + gate);
    fs.writeFileSync(
      path.join(tmp, 'check-zz-comment.mjs'),
      "// readdirSync(path.join(UI_ROOT, 'src'))\nconsole.log(1);\n",
    );
    const found = findLocalWalkers(tmp, []);
    const ok = found.length === 1 && found[0] === 'check-zz-new.mjs';
    console.log(`${TAG} selftest ${ok ? 'ok' : 'FAIL'}: [C6] находит только локальный обход`);
    if (!ok) bad++;
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  if (bad) {
    console.error(`${TAG} FAIL: selftest, расхождений: ${bad}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest PASS (${cases.length + 1} случаев)`);
}

function main() {
  if (process.argv.includes('--selftest')) return selftest();
  const baseline = loadJson('gate-coverage.baseline.json');
  const moves = loadJson('move-map.json').moves ?? [];
  const scanCount = walk().length;
  const scriptsDir = path.join(UI_ROOT, 'scripts');
  const walkers = findWalkers(scriptsDir);
  const { counts, errors } = runWalkers(Object.keys(baseline.perGate ?? {}), UI_ROOT);
  const failures = evaluate({
    baseline,
    moves,
    exists: (p) => fs.existsSync(path.join(REPO, p)),
    scanCount,
    walkers,
    counts,
    localWalkers: findLocalWalkers(scriptsDir, Object.keys(baseline.pinned ?? {})),
    dirCount: countDir,
  });
  failures.push(...errors);
  if (failures.length) {
    for (const f of failures) console.error(`${TAG} ${f}`);
    process.exit(1);
  }
  console.log(
    `${TAG} PASS: в скане ${scanCount} файлов, записей move-map ${moves.length}; обходчиков: ${walkers.length}, файлов суммарно: ${Object.values(counts).reduce((a, b) => a + b, 0)}`,
  );
}

main();
