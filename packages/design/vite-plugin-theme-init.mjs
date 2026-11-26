// Общий Vite-плагин: отдаёт /theme-init.js из packages/design/public в dev и кладёт файл
// в корень dist при сборке. Скрипт остаётся внешним (CSP script-src 'self', без inline).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), 'public', 'theme-init.js');

export default function themeInit() {
  return {
    name: 'trackly-theme-init',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url?.split('?')[0] !== '/theme-init.js') return next();
        res.setHeader('Content-Type', 'application/javascript; charset=utf-8');
        res.end(fs.readFileSync(SRC));
      });
    },
    generateBundle() {
      this.emitFile({
        type: 'asset',
        fileName: 'theme-init.js',
        source: fs.readFileSync(SRC),
      });
    },
  };
}
