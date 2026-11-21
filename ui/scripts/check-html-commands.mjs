#!/usr/bin/env node
// [check-html-commands] Паритет белого списка HTML-команд клиента с сервером
// (quick 261121-heg, INT-35).
//
// ЗАЧЕМ. `apiCall` (ui/src/lib/api/client.ts) по HTTP возвращает тело
// `text/html` как строку ТОЛЬКО для команд из `HTML_COMMANDS`; на любую другую
// команду HTML означает отсутствующий маршрут (SPA-fallback отдаёт index.html
// с 200) и превращается в ошибку NOT_FOUND. Если сервер заведёт новый
// text/html-обработчик, а список забудут, печатная форма молча сломается по
// LAN; если в списке останется имя без HTML-обработчика, клиент снова начнёт
// выдавать страницу за данные. Гейт держит оба направления.
//
// Читает НАСТОЯЩИЕ исходники (от репозитория, никакого вкомпилированного
// списка ожидаемых имён):
//   A = для каждого вхождения `CONTENT_TYPE, "text/html` в
//       crates/trackly-app/src/http/*.rs (строки-комментарии игнорируются;
//       тело `spa_fallback` в mod.rs — не маршрут, исключено) — объемлющий
//       `async fn <handler>` и в том же файле
//       `.route("/api/v1/<name>", post(<handler>))` → name;
//   B = строковые литералы `HTML_COMMANDS` в ui/src/lib/api/client.ts.
// Проверки:
//   [H0] A не пусто, B разобран (защита от вакуумного прохода);
//   [H1] у каждого text/html-обработчика есть маршрут в том же файле;
//   [H2] A == B (разница печатается в обе стороны).
//
// Чего гейт НЕ видит (честно): HTML, собранный не через литерал
// `CONTENT_TYPE, "text/html…"` (например, `Html(...)` из axum или заголовок
// из переменной); маршрут, объявленный не литералом `.route("/api/v1/…",
// post(handler))`. Таких мест сегодня нет — появятся, гейт надо расширить.
//
// Usage:
//   node scripts/check-html-commands.mjs                 # реальный репозиторий
//   node scripts/check-html-commands.mjs --src=<dir>     # копия корня репозитория
//   node scripts/check-html-commands.mjs --selftest      # мутанты копий настоящих исходников
//
// Zero-dependency: только node:fs/node:os/node:path/node:url.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-html-commands]';
const REPO_ROOT = path.resolve(__dirname, '../..');
const HTTP_DIR = 'crates/trackly-app/src/http';
const CLIENT = 'ui/src/lib/api/client.ts';
const HTML_MARK = /CONTENT_TYPE,\s*"text\/html/g;
const ROUTE_RE = /\.route\(\s*"\/api\/v1\/(\w+)"\s*,\s*post\(\s*(\w+)\s*\)/g;
const FN_RE = /async\s+fn\s+(\w+)/g;

/** Обработчики, чьё text/html — не маршрут: файл → имена функций. */
const NOT_ROUTES = { 'mod.rs': ['spa_fallback'] };

function lineStart(src, idx) {
  return src.lastIndexOf('\n', idx - 1) + 1;
}

function isCommentLine(src, idx) {
  return /^\s*\/\//.test(src.slice(lineStart(src, idx), idx));
}

/** Множество A: { names, errors }. */
export function collectServer(root) {
  const dir = path.join(root, HTTP_DIR);
  const errors = [];
  const names = new Set();
  const files = fs
    .readdirSync(dir)
    .filter((f) => f.endsWith('.rs'))
    .sort();
  if (files.length === 0) errors.push(`[H0] в ${HTTP_DIR} нет ни одного .rs`);
  for (const f of files) {
    const src = fs.readFileSync(path.join(dir, f), 'utf8');
    const routes = new Map();
    for (const m of src.matchAll(ROUTE_RE)) {
      if (isCommentLine(src, m.index)) continue;
      routes.set(m[2], [...(routes.get(m[2]) ?? []), m[1]]);
    }
    const fns = [...src.matchAll(FN_RE)].filter((m) => !isCommentLine(src, m.index));
    for (const m of src.matchAll(HTML_MARK)) {
      if (isCommentLine(src, m.index)) continue;
      const enclosing = fns.filter((fm) => fm.index < m.index).pop();
      const line = src.slice(0, m.index).split('\n').length;
      if (!enclosing) {
        errors.push(`[H1] ${HTTP_DIR}/${f}:${line} text/html вне async fn`);
        continue;
      }
      const handler = enclosing[1];
      if ((NOT_ROUTES[f] ?? []).includes(handler)) continue;
      const routed = routes.get(handler);
      if (!routed || routed.length === 0) {
        errors.push(
          `[H1] ${HTTP_DIR}/${f}:${line} обработчик ${handler} отвечает text/html, но маршрута .route("/api/v1/…", post(${handler})) в файле нет`,
        );
        continue;
      }
      for (const r of routed) names.add(r);
    }
  }
  return { names, errors };
}

/** Множество B: литералы HTML_COMMANDS или null, если объявление не найдено. */
export function collectClient(root) {
  const src = fs.readFileSync(path.join(root, CLIENT), 'utf8');
  const m = src.match(/export const HTML_COMMANDS\b[^=]*=\s*new Set\(\[([\s\S]*?)\]\)/);
  if (!m) return null;
  const body = m[1]
    .split('\n')
    .map((l) => l.replace(/\/\/.*$/, ''))
    .join('\n');
  return new Set([...body.matchAll(/'([^']*)'|"([^"]*)"/g)].map((x) => x[1] ?? x[2]));
}

export function check(root) {
  const failures = [];
  const server = collectServer(root);
  failures.push(...server.errors);
  if (server.names.size === 0) {
    failures.push(`[H0] ни одного text/html-маршрута в ${HTTP_DIR} — разбор сломан`);
  }
  const client = collectClient(root);
  if (client === null) {
    failures.push(
      `[H0] в ${CLIENT} не найдено объявление export const HTML_COMMANDS = new Set([...])`,
    );
    return { failures, server: server.names, client: new Set() };
  }
  const onlyServer = [...server.names].filter((n) => !client.has(n)).sort();
  const onlyClient = [...client].filter((n) => !server.names.has(n)).sort();
  if (onlyServer.length > 0) {
    failures.push(
      `[H2] сервер отвечает text/html, а в HTML_COMMANDS нет: ${onlyServer.join(', ')} — по LAN клиент бросит NOT_FOUND`,
    );
  }
  if (onlyClient.length > 0) {
    failures.push(
      `[H2] в HTML_COMMANDS есть, а text/html-обработчика нет: ${onlyClient.join(', ')} — клиент выдаст HTML за данные`,
    );
  }
  return { failures, server: server.names, client };
}

// ---------------------------------------------------------------------------
// Self-test: мутанты копий НАСТОЯЩИХ исходников
// ---------------------------------------------------------------------------

function copyInputs(from) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'check-html-commands-'));
  fs.cpSync(path.join(from, HTTP_DIR), path.join(tmp, HTTP_DIR), { recursive: true });
  fs.mkdirSync(path.dirname(path.join(tmp, CLIENT)), { recursive: true });
  fs.copyFileSync(path.join(from, CLIENT), path.join(tmp, CLIENT));
  return tmp;
}

function mutate(root, rel, from, to) {
  const p = path.join(root, rel);
  const src = fs.readFileSync(p, 'utf8');
  const count = src.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`якорь мутанта в ${rel} встречается ${count} раз, нужен ровно 1: ${from}`);
  }
  fs.writeFileSync(p, src.replace(from, to));
}

const MUTANTS = [
  {
    name: 'из HTML_COMMANDS убрано имя (reports_export_pdf)',
    rel: CLIENT,
    from: "  'reports_export_pdf',\n",
    to: '',
    code: '[H2]',
  },
  {
    name: 'в HTML_COMMANDS лишнее имя (printers_list)',
    rel: CLIENT,
    from: "  'templates_validate_preview',\n",
    to: "  'templates_validate_preview',\n  'printers_list',\n",
    code: '[H2]',
  },
  {
    name: 'в http/printers.rs новый text/html-обработчик с маршрутом',
    rel: `${HTTP_DIR}/printers.rs`,
    from: 'pub fn router() -> Router<AppCtx> {\n    Router::new()\n',
    to:
      'pub async fn handler_zz_html() -> impl axum::response::IntoResponse {\n' +
      '    ([(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")], String::new())\n' +
      '}\n\n' +
      'pub fn router() -> Router<AppCtx> {\n    Router::new()\n' +
      '        .route("/api/v1/printers_zz_html", post(handler_zz_html))\n',
    code: '[H2]',
  },
  {
    name: 'в http/printers.rs text/html-обработчик без маршрута',
    rel: `${HTTP_DIR}/printers.rs`,
    from: 'pub fn router() -> Router<AppCtx> {\n',
    to:
      'pub async fn handler_zz_orphan() -> impl axum::response::IntoResponse {\n' +
      '    ([(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")], String::new())\n' +
      '}\n\n' +
      'pub fn router() -> Router<AppCtx> {\n',
    code: '[H1]',
  },
  {
    name: 'объявление HTML_COMMANDS переименовано (разбор пуст)',
    rel: CLIENT,
    from: 'export const HTML_COMMANDS:',
    to: 'export const HTML_COMMANDS_X:',
    code: '[H0]',
  },
];

function selftest(from) {
  const problems = [];
  const control = copyInputs(from);
  try {
    const res = check(control);
    if (res.failures.length !== 0) {
      problems.push(`контроль (немутированная копия) красный: ${res.failures.join(' | ')}`);
    }
  } finally {
    fs.rmSync(control, { recursive: true, force: true });
  }
  for (const mu of MUTANTS) {
    const tmp = copyInputs(from);
    try {
      mutate(tmp, mu.rel, mu.from, mu.to);
      const res = check(tmp);
      if (!res.failures.some((f) => f.startsWith(mu.code))) {
        problems.push(
          `мутант «${mu.name}» не пойман кодом ${mu.code}: ${res.failures.join(' | ') || 'зелёный'}`,
        );
      } else {
        console.log(`${TAG} мутант «${mu.name}»: пойман ${mu.code}`);
      }
    } catch (e) {
      problems.push(`мутант «${mu.name}»: ${e.message}`);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  }
  return problems;
}

function main() {
  const srcArg = process.argv.find((a) => a.startsWith('--src='));
  const root = srcArg ? path.resolve(srcArg.slice('--src='.length)) : REPO_ROOT;
  if (process.argv.includes('--selftest')) {
    const problems = selftest(root);
    if (problems.length > 0) {
      console.error(`${TAG} SELFTEST FAIL:`);
      for (const p of problems) console.error(`  ${p}`);
      process.exit(1);
    }
    console.log(`${TAG} selftest OK — контроль зелёный, ${MUTANTS.length} мутантов красные`);
    return;
  }
  const { failures, server } = check(root);
  if (failures.length > 0) {
    console.error(`${TAG} FAIL:`);
    for (const f of failures) console.error(`  ${f}`);
    process.exit(1);
  }
  console.log(
    `${TAG} OK — ${server.size} text/html-команд сервера == HTML_COMMANDS: ${[...server].sort().join(', ')}`,
  );
}

main();
