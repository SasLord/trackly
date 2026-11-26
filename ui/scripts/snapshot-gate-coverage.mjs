#!/usr/bin/env node
// Снимает baseline набора файлов фронтенда в fixtures/gate-coverage.baseline.json.
//
// ЗАПУСКАТЬ ОДИН РАЗ, ДО любого переезда файлов (фаза 42, D-20). Повторный запуск
// после начала переездов запрещён: он затёр бы эталон потерянным набором.
// В файл попадают только пути кода из `git ls-files`.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { REPO, UI_ROOT } from './_roots.mjs';

const EXTS = ['svelte', 'ts', 'js', 'scss', 'css'];
const out = execFileSync('git', ['ls-files', 'ui/src', 'ui/public'], {
  cwd: REPO,
  encoding: 'utf8',
});
const files = out
  .split('\n')
  .filter(Boolean)
  .filter((f) => EXTS.includes(f.split('.').pop()))
  .filter((f) => f !== 'ui/src/bindings.ts')
  .sort();

const totals = { all: files.length };
for (const e of EXTS) totals[e] = files.filter((f) => f.endsWith('.' + e)).length;

const dest = path.join(UI_ROOT, 'scripts', 'fixtures', 'gate-coverage.baseline.json');
fs.mkdirSync(path.dirname(dest), { recursive: true });
fs.writeFileSync(dest, JSON.stringify({ files, totals }, null, 2) + '\n');
console.log(`baseline: ${files.length} файлов -> ${path.relative(REPO, dest)}`);
