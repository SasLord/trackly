#!/usr/bin/env node
// eslint для packages/ и link-ui/ (фаза 42, план 09, допущение A4).
//
// ESLint 9 (flat config) не линтует файлы вне каталога конфига, поэтому `eslint .` в ui/
// пакеты не видит. Здесь eslint запускается из каталога каждого пакета с конфигом-реэкспортом
// ui/eslint.config.js. Имя с `_`: это обёртка, не гейт. Оба корня обязаны содержать src/:
// исчезновение link-ui/src не должно давать зелёный eslint без единого файла (WR-04).
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const UI = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const REPO = path.resolve(UI, '..');
const ESLINT = path.join(UI, 'node_modules', 'eslint', 'bin', 'eslint.js');

let failed = false;
for (const [dir, required] of [
  ['packages', true],
  ['link-ui', true],
]) {
  const cwd = path.join(REPO, dir);
  const hasSrc =
    dir === 'packages'
      ? fs.readdirSync(cwd).some((n) => fs.existsSync(path.join(cwd, n, 'src')))
      : fs.existsSync(path.join(cwd, 'src'));
  if (!hasSrc) {
    // required у обоих корней: пропуск = вакуумный проход (WR-04).
    console.error(`[eslint-workspaces] ${dir}: нет ни одного src/ — пустое покрытие недопустимо`);
    process.exit(1);
  }
  const r = spawnSync(process.execPath, [ESLINT, '.', '--ext', '.ts,.svelte'], {
    cwd,
    stdio: 'inherit',
  });
  if (r.status !== 0) failed = true;
}
process.exit(failed ? 1 : 0);
