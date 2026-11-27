// [_rune-harness] Поведенческий стенд для модулей с рунами (*.svelte.ts) в Node — без браузера.
//
// Зачем: svelte-check / eslint / build не исполняют логику сторов, а позиционные гейты
// (check-chat-ui) не видят поведения. Стенд ИСПОЛНЯЕТ настоящие модули:
//   *.ts        → typescript.transpileModule (Node 20 в CI не импортирует .ts напрямую);
//   *.svelte.ts → затем svelte/compiler compileModule (generate: 'client', настоящий рантайм рун).
// Импорты разрешаются хуками загрузчика (module.register, Node ≥ 20.6):
//   `$lib/...` → ui/src/lib/..., `@trackly/<пакет>/<подпуть>` → packages/<пакет>/src/<подпуть>
//   (D-20; пакетные файлы транслируются так же, как ui/src), относительные без расширения → `.ts`;
//   спецификаторы из `stubs` → data:-модули (заглушки ws/auth/api/плагинов).
// Мутации: `overrides` = { абсолютный путь: [[from, to], ...] } применяется к исходнику ДО
// трансляции (якорь обязан встречаться ровно один раз — иначе стенд падает).
//
// Ограничение: это логика сторов без DOM и без WKWebView/WebView2 — рендер, раскладка и
// нативные уведомления проверяются вручную (memory: synthetic harness ≠ verification).

import { Buffer } from 'node:buffer';
import { register } from 'node:module';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const UI = path.resolve(__dirname, '..');
export const SRC = path.join(UI, 'src');
export const PKGS = path.resolve(UI, '..', 'packages');

const HOOKS = `
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
let cfg;
let ts;
let compileModule;
export async function initialize(data) {
  cfg = data;
  const req = createRequire(pathToFileURL(path.join(cfg.ui, 'package.json')));
  ts = req('typescript');
  ({ compileModule } = req('svelte/compiler'));
}
function inRoots(p) {
  if (p.startsWith(cfg.src)) return true;
  const rel = path.relative(cfg.pkgs, p);
  return !rel.startsWith('..') && !path.isAbsolute(rel) && /^[^/\\\\]+[/\\\\]src[/\\\\]/.test(rel);
}
function withTs(p) {
  if (fs.existsSync(p) && fs.statSync(p).isFile()) return p;
  if (fs.existsSync(p + '.ts')) return p + '.ts';
  return null;
}
export async function resolve(spec, ctx, next) {
  if (Object.prototype.hasOwnProperty.call(cfg.stubs, spec)) {
    return { url: 'data:text/javascript;base64,' + Buffer.from(cfg.stubs[spec]).toString('base64'), shortCircuit: true };
  }
  let base = null;
  if (spec.startsWith('$lib/')) base = path.join(cfg.src, 'lib', spec.slice(5));
  else if (spec.startsWith('@trackly/')) {
    const rest = spec.slice('@trackly/'.length);
    const i = rest.indexOf('/');
    if (i > 0) base = path.join(cfg.pkgs, rest.slice(0, i), 'src', rest.slice(i + 1));
  } else if ((spec.startsWith('./') || spec.startsWith('../')) && ctx.parentURL && ctx.parentURL.startsWith('file:'))
    base = path.resolve(path.dirname(fileURLToPath(ctx.parentURL)), spec);
  if (base !== null) {
    const q = base.indexOf('?');
    const query = q < 0 ? '' : base.slice(q);
    const p = withTs(q < 0 ? base : base.slice(0, q));
    if (p !== null && inRoots(p)) return { url: pathToFileURL(p).href + query, shortCircuit: true };
  }
  return next(spec, ctx);
}
export async function load(url, ctx, next) {
  if (!url.startsWith('file:')) return next(url, ctx);
  const p = fileURLToPath(url.split('?')[0]);
  if (!inRoots(p) || !p.endsWith('.ts')) return next(url, ctx);
  let src = fs.readFileSync(p, 'utf8');
  for (const [from, to] of cfg.overrides[p] ?? []) {
    const n = src.split(from).length - 1;
    if (n !== 1) throw new Error('[_rune-harness] якорь мутации встречается ' + n + ' раз в ' + p + ': ' + from);
    src = src.replace(from, to);
  }
  let js = ts.transpileModule(src, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext, verbatimModuleSyntax: false },
    fileName: p,
    reportDiagnostics: false,
  }).outputText;
  if (p.endsWith('.svelte.ts')) {
    js = compileModule(js, { filename: p.replace(/\\.ts$/, '.js'), generate: 'client', dev: false }).js.code;
  }
  return { format: 'module', source: js, shortCircuit: true };
}
`;

/** Регистрирует хуки. Вызывать до первого динамического import() тестируемых модулей. */
export function installHarness({ stubs = {}, overrides = {} } = {}) {
  register('data:text/javascript;base64,' + Buffer.from(HOOKS).toString('base64'), {
    data: { ui: UI, src: SRC, pkgs: PKGS, stubs, overrides },
  });
}

/** URL файла пакета: pkgUrl('kit', 'stores/toast.svelte.ts'). */
export const pkgUrl = (pkg, rel, query = '') =>
  pathToFileURL(path.join(PKGS, pkg, 'src', rel)).href + query;

export const srcUrl = (rel, query = '') => pathToFileURL(path.join(SRC, rel)).href + query;

/** Ждать условие с крайним сроком (без фиксированных пауз). */
export async function until(cond, ms = 2000) {
  const end = Date.now() + ms;
  while (!(await cond())) {
    if (Date.now() > end) return false;
    await new Promise((r) => setTimeout(r, 10));
  }
  return true;
}

/**
 * Каркас скрипта-гейта. Родитель запускает себя дочерним процессом (свежий граф модулей на
 * каждый прогон): без аргументов — реальные модули обязаны быть зелёными; `--selftest` —
 * дополнительно каждый мутант НАСТОЯЩЕГО исходника обязан покраснеть ожидаемым кейсом.
 *
 * mutants: [{ name, file (отн. src), from, to | edits: [[from, to], ...], expect (код кейса) }]
 *   либо { ..., equivalent: true, reason } — одиночное снятие, обязанное НЕ менять результат.
 *   edits применяются по порядку — так строится перестановка (вырезать + вставить ниже).
 * runCases(overrides) → Promise<string[]> нарушений «[КОД] текст».
 */
export async function gateMain({ tag, scriptUrl, mutants, runCases }) {
  const self = fileURLToPath(scriptUrl);
  if (process.env.RUNE_HARNESS_CHILD) {
    const idx = Number(process.env.RUNE_HARNESS_MUTANT ?? '-1');
    const overrides = {};
    if (idx >= 0) {
      const m = mutants[idx];
      overrides[path.join(SRC, m.file)] = m.edits ?? [[m.from, m.to]];
    }
    let bad;
    try {
      bad = await runCases(overrides);
    } catch (e) {
      bad = [`[CRASH] ${e && e.stack ? e.stack : e}`];
    }
    process.stdout.write(JSON.stringify(bad) + '\n');
    process.exit(0);
  }
  const run = (idx) => {
    const r = spawnSync(process.execPath, [self], {
      env: { ...process.env, RUNE_HARNESS_CHILD: '1', RUNE_HARNESS_MUTANT: String(idx) },
      encoding: 'utf8',
      timeout: 60000,
    });
    const line = (r.stdout || '').trim().split('\n').pop() || '';
    try {
      return JSON.parse(line);
    } catch {
      return [`[CRASH] дочерний процесс без результата (status ${r.status}): ${r.stderr}`];
    }
  };
  const real = run(-1);
  if (real.length > 0) {
    for (const b of real) console.error(`${tag} FAIL — ${b}`);
    process.exit(1);
  }
  if (!process.argv.includes('--selftest')) {
    console.log(`${tag} OK`);
    return;
  }
  let missed = 0;
  mutants.forEach((m, i) => {
    const bad = run(i);
    const crash = bad.find((b) => b.startsWith('[CRASH]'));
    // equivalent: задокументированный эквивалент (одиночное снятие слоя, закрытого соседним) —
    // обязан НЕ менять результат: так эквивалентность подтверждена, а не предположена.
    if (m.equivalent) {
      const eqOk = !crash && bad.length === 0;
      console.log(
        `${tag} эквивалент «${m.name}» (${m.reason}): ${eqOk ? 'результат не изменился' : 'РЕЗУЛЬТАТ ИЗМЕНИЛСЯ'}`,
      );
      if (!eqOk) {
        missed++;
        console.error(`${tag}   ${crash ?? `нарушения: ${JSON.stringify(bad)}`}`);
      }
      return;
    }
    const ok = !crash && bad.some((b) => b.startsWith(`[${m.expect}]`));
    console.log(`${tag} мутант «${m.name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${m.expect}]`);
    if (!ok) {
      missed++;
      if (crash) console.error(`${tag}   ${crash}`);
      else console.error(`${tag}   нарушения: ${JSON.stringify(bad)}`);
    }
  });
  if (missed > 0) process.exit(1);
  console.log(`${tag} SELFTEST PASS — реальные модули зелёные, ${mutants.length} мутантов красные`);
}
