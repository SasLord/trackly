// Prettier ищет конфиг вверх от файла, поэтому пакеты получают тот же конфиг, что и ui/
// (ui/.prettierrc.json), а плагин резолвится из node_modules ui (в packages/ его нет).
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ui = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'ui');
const require = createRequire(path.join(ui, 'package.json'));
const base = JSON.parse(readFileSync(path.join(ui, '.prettierrc.json'), 'utf8'));

export default {
  ...base,
  plugins: base.plugins.map((p) => require.resolve(p)),
};
