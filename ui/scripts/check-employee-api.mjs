#!/usr/bin/env node
// [check-employee-api] Гейт фазы 41.4, план 44 (D-38): сотрудник не вызывает
// staff-only команды из компонентов, достижимых ему, без проверки роли.
//
// Как работает (структурно, без зависимостей; рантайм он не доказывает):
//  1. Замыкание импортов от трёх входов: RequestsPage, EmployeeLayout, LoginPage
//     (App.svelte и routes.ts не входят — они тянут весь UI администратора).
//     Типовые импорты (`import type`), bindings*.ts и ассеты пропускаются.
//  2. Карта «метод обёртки -> команда» строится по `export const X = { m: ... apiCall('cmd') }`
//     из файлов замыкания (api.ts, lib/api/*.ts). Прямые apiCall('cmd') берутся по месту;
//     apiCall(не_литерал) и метод обёртки без команды — «неизвестная команда» (нарушение).
//  3. Для каждого вызова: команда в фикстуре ui/scripts/fixtures/employee-allowed-commands.json
//     (её сверяет с сервером тест employee_allowed_fixture_sound) — разрешено.
//  4. Иначе нужна охрана (STAFF_GUARD) одним из способов:
//     a) функция-обработчик/эффект до вызова содержит выражение из STAFF_GUARD
//        (префикс охватывающих блоков, закрытые блоки свернуты; верхний уровень файла
//        не считается — там объявления `const isSpecialist = ...` дали бы ложную охрану);
//     b) вызов в шаблоне стоит под {#if} с выражением из STAFF_GUARD;
//     c) компонент монтируется только персоналу: каждое `<Имя` в замыкании стоит под
//        {#if} с STAFF_GUARD либо внутри компонента, который сам так монтируется.
//     Вход (нет импортирующих) пункта c) не имеет.
//  5. Модули lib/stores/*: вызов внутри функции приписывается вызывающему (сторы
//     общие, ensureLoaded зовёт команду справочника); остальные .ts-модули замыкания
//     проверяются по месту определения.
//
// Ограничения: свойства-методы сторов (obj.method()) не прослеживаются; охрана на
// верхнем уровне скрипта не видна; эвристика скобок не знает о скобках в строках.
//
// Usage:
//   node scripts/check-employee-api.mjs
//   node scripts/check-employee-api.mjs --selftest
//   node scripts/check-employee-api.mjs --override src/путь=/абсолютный/файл ...   (проверка на подменённых исходниках)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-employee-api]';

const ENTRIES = [
  'src/features/requests/RequestsPage.svelte',
  'src/features/layout/EmployeeLayout.svelte',
  'src/features/auth/LoginPage.svelte',
];

// Счётные минимумы против «зелёного по пустоте»: прогон на исправленном коде дал 81 файл и 79
// вызова команд; запас на естественное сокращение кода.
const MIN_FILES = 60;
const MIN_CALLS = 50;

/**
 * Выражения-охраны. Каждая запись с причиной: расширять список ради зелёного нельзя.
 * `re` применяется к тексту префикса/условия; объявления и описания пропсов исключены.
 */
export const STAFF_GUARD = [
  {
    name: 'isSpecialist',
    re: /(?<![\w.$])isSpecialist\b(?!\s*(?:=(?!=)|:))/,
    reason: 'производная RequestDetail: роль admin или manager (TransitionRequests на сервере)',
  },
  {
    name: 'canUseRepairs()',
    re: /(?<!function\s)\bcanUseRepairs\s*\(\s*\)/,
    reason: 'repairLocks: роль admin или manager; ремонтные команды закрыты сотруднику',
  },
  {
    name: "role === 'admin'|'manager'",
    re: /\brole\s*(?:===?\s*'(?:admin|manager)'|!==?\s*'employee')/,
    reason: 'прямая проверка роли текущего пользователя',
  },
  {
    name: 'canManageDeleted',
    re: /(?<![\w.$])canManageDeleted\b(?!\s*=(?!=))/,
    reason: 'серверный флаг прав из attachments_list: восстановление и окончательное удаление',
  },
  {
    name: 'canDeleteAny/canDeleteItem()',
    re: /(?<![\w.$])canDeleteAny\b|(?<!function\s)(?<![\w.$])canDeleteItem\s*\(/,
    reason: 'серверный флаг attachments_list (canDeleteAny) или своё вложение автора',
  },
  {
    name: 'canManage',
    re: /(?<![\w.$])canManage\b(?!\s*[=:?])/,
    reason:
      'проп попапов и секций ремонта: родитель передаёт isSpecialist (canManage={isSpecialist})',
  },
];
for (const g of STAFF_GUARD)
  if (g.reason.length < 20) throw new Error('reason too short: ' + g.name);

/**
 * Команды без сессии (публичные маршруты роутера, http/mod.rs, топология п.2): вызывать их
 * можно и сотруднику, и анониму, роль не проверяется.
 */
export const PUBLIC_COMMANDS = new Set([
  'auth_login',
  'request_ad_restore',
  'auth_bootstrap',
  'auth_status',
]);

const matchesGuard = (t) => STAFF_GUARD.some((g) => g.re.test(t));

/** Условие {#if}: охрана, если содержит STAFF_GUARD и не отрицание, и нет верхнеуровневого `||`. */
function posGuard(cond) {
  if (!matchesGuard(cond)) return false;
  if (/^\s*!/.test(cond)) return false;
  let t = cond;
  let prev;
  do {
    prev = t;
    t = t.replace(/\([^()]*\)/g, '');
  } while (t !== prev);
  return !t.includes('||');
}
/** Ветка {:else} после `{#if !охрана}` — охраняется. */
function negGuard(cond) {
  return /^\s*!/.test(cond) && matchesGuard(cond);
}

// ---------- файловый слой ----------

function mkFs(root, overrides) {
  const cache = new Map();
  const exists = (rel) => overrides.has(rel) || fs.existsSync(path.join(root, rel));
  const read = (rel) => {
    if (!cache.has(rel)) {
      const raw = overrides.has(rel)
        ? overrides.get(rel)
        : fs.readFileSync(path.join(root, rel), 'utf8');
      cache.set(rel, raw);
    }
    return cache.get(rel);
  };
  return { exists, read, root };
}

function resolveImport(F, fromRel, spec) {
  let base;
  if (spec.startsWith('$lib/')) base = 'src/lib/' + spec.slice(5);
  else if (spec.startsWith('.')) base = path.posix.join(path.posix.dirname(fromRel), spec);
  else return { skip: true };
  if (/\?/.test(base) || /\.(scss|css|json|svg|png|woff2?|ttf|html|txt|js)$/.test(base))
    return { skip: true };
  for (const cand of [
    base,
    base + '.ts',
    base + '.svelte',
    base + '.svelte.ts',
    base + '/index.ts',
  ]) {
    if (
      /\.(ts|svelte)$/.test(cand) &&
      F.exists(cand) &&
      fs.statSync(path.join(F.root, cand), { throwIfNoEntry: false })?.isFile() !== false
    )
      return { rel: cand };
  }
  return { missing: base };
}

const IMPORT_RE = /import\s+(type\s+)?([\s\S]*?)\s*from\s*['"]([^'"]+)['"]/g;
const SIDE_IMPORT_RE = /import\s*['"]([^'"]+)['"]/g;
const DYN_IMPORT_RE = /import\(\s*['"]([^'"]+)['"]\s*\)/g;

/** Привязки import: local -> { spec, name } (name: 'default' | имя | '*'). */
function parseImports(text) {
  const out = [];
  for (const m of text.matchAll(IMPORT_RE)) {
    if (m[1]) continue;
    const clause = m[2].trim();
    const spec = m[3];
    const bindings = [];
    const named = /\{([\s\S]*?)\}/.exec(clause);
    const head = named ? clause.slice(0, named.index) : clause;
    const def = /^\s*([A-Za-z_$][\w$]*)\s*,?\s*$/.exec(head.replace(/,\s*$/, ''));
    if (def && def[1] !== 'type') bindings.push({ local: def[1], name: 'default' });
    const ns = /\*\s+as\s+([\w$]+)/.exec(head);
    if (ns) bindings.push({ local: ns[1], name: '*' });
    if (named) {
      for (const part of named[1].split(',')) {
        const p = part.trim();
        if (!p || /^type\s/.test(p)) continue;
        const a = /^([\w$]+)(?:\s+as\s+([\w$]+))?$/.exec(p);
        if (a) bindings.push({ local: a[2] ?? a[1], name: a[1] });
      }
    }
    out.push({ spec, bindings });
  }
  for (const m of text.matchAll(SIDE_IMPORT_RE)) out.push({ spec: m[1], bindings: [] });
  for (const m of text.matchAll(DYN_IMPORT_RE)) out.push({ spec: m[1], bindings: [] });
  return out;
}

function lineOf(text, p) {
  let n = 1;
  for (let i = 0; i < p; i++) if (text.charCodeAt(i) === 10) n++;
  return n;
}

function blank(text, ranges) {
  const out = text.split('');
  for (const [a, b] of ranges) for (let i = a; i < b; i++) if (out[i] !== '\n') out[i] = ' ';
  return out.join('');
}

// ---------- разбор файла ----------

function parseFile(rel, raw) {
  const text = stripComments(raw);
  const isSvelte = rel.endsWith('.svelte');
  const scripts = []; // [start, end] содержимого <script>
  const hidden = [];
  if (isSvelte) {
    for (const m of text.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)) {
      const s = m.index + m[0].indexOf('>') + 1;
      scripts.push([s, s + m[1].length]);
      hidden.push([m.index, m.index + m[0].length]);
    }
    for (const m of text.matchAll(/<style[^>]*>[\s\S]*?<\/style>/g))
      hidden.push([m.index, m.index + m[0].length]);
  } else scripts.push([0, text.length]);
  const template = isSvelte ? blank(text, hidden) : '';
  return { rel, text, isSvelte, scripts, template, imports: parseImports(text) };
}

/** Объекты-обёртки `export const X = { m: ... apiCall('cmd') }` -> { X: { m: cmd|null } }. */
function parseWrappers(text) {
  const res = {};
  for (const m of text.matchAll(/export const (\w+)\s*=\s*\{/g)) {
    let depth = 1;
    let i = m.index + m[0].length;
    const start = i;
    while (i < text.length && depth > 0) {
      if (text[i] === '{') depth++;
      else if (text[i] === '}') depth--;
      i++;
    }
    const block = text.slice(start, i - 1);
    const keys = [...block.matchAll(/^ {2}(\w+)\s*:/gm)];
    const methods = {};
    keys.forEach((k, idx) => {
      const seg = block.slice(k.index, idx + 1 < keys.length ? keys[idx + 1].index : block.length);
      const c = /apiCall\s*(?:<[^()]*?>)?\(\s*['"]([\w]+)['"]/.exec(seg);
      methods[k[1]] = c ? c[1] : null;
    });
    if (Object.values(methods).some((v) => v)) res[m[1]] = methods;
  }
  return res;
}

/** Функции файла: [{name, bodyStart, bodyEnd}] (объявления и `const f = (...) => {`). */
function parseFunctions(text) {
  const out = [];
  const add = (name, from) => {
    // найти «)» закрывающую параметры, затем тело
    let i = text.indexOf('(', from);
    if (i < 0) return;
    let depth = 0;
    for (; i < text.length; i++) {
      if (text[i] === '(') depth++;
      else if (text[i] === ')') {
        depth--;
        if (depth === 0) break;
      }
    }
    const tail = text.slice(i + 1, i + 400);
    const m = /^[^{;]*?(?:=>)?\s*\{/.exec(tail);
    if (!m) return;
    const bs = i + 1 + m[0].length;
    let d = 1;
    let j = bs;
    while (j < text.length && d > 0) {
      if (text[j] === '{') d++;
      else if (text[j] === '}') d--;
      j++;
    }
    out.push({ name, bodyStart: bs, bodyEnd: j - 1 });
  };
  for (const m of text.matchAll(
    /(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*(?:<[^>(]*>)?\s*\(/g,
  ))
    add(m[1], m.index + m[0].length - 1);
  for (const m of text.matchAll(/(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s*)?\(/g))
    add(m[1], m.index + m[0].length - 1);
  return out;
}

// ---------- охрана ----------

function collapse(t) {
  let prev;
  do {
    prev = t;
    t = t.replace(/\{[^{}]*\}/g, '{}');
  } while (t !== prev);
  return t;
}

/** Охрана вызова в скрипте: префиксы охватывающих блоков (кроме корня файла/скрипта). */
function scriptGuarded(text, start, p) {
  const stack = [];
  for (let i = start; i < p; i++) {
    if (text[i] === '{') stack.push(i);
    else if (text[i] === '}') stack.pop();
  }
  for (const b of stack) {
    if (matchesGuard(collapse(text.slice(b + 1, p)))) return true;
  }
  return false;
}

/** Стек {#if} шаблона на позиции p: список охранённости записей. */
function templateIfStack(template, p) {
  const stack = [];
  const re = /\{(#if|:else if)\s|\{:else\}|\{\/if\}/g;
  let m;
  while ((m = re.exec(template)) && m.index < p) {
    if (m[0] === '{/if}') stack.pop();
    else if (m[0] === '{:else}') {
      const top = stack[stack.length - 1];
      if (top) stack[stack.length - 1] = { kind: 'else', prev: top.cond ?? top.prev ?? '' };
    } else {
      let depth = 1;
      let i = m.index + 1;
      while (i < template.length && depth > 0) {
        i++;
        if (template[i] === '{') depth++;
        else if (template[i] === '}') depth--;
      }
      const cond = template.slice(m.index + m[0].length, i);
      if (i >= p) break; // позиция внутри самого условия
      if (m[1] === '#if') stack.push({ kind: 'if', cond });
      else {
        const top = stack[stack.length - 1];
        if (top) stack[stack.length - 1] = { kind: 'if', cond };
      }
    }
  }
  return stack.some((e) => (e.kind === 'if' ? posGuard(e.cond) : negGuard(e.prev)));
}

// ---------- основной проход ----------

export function check({ root = UI_ROOT, overrides = new Map(), fixture } = {}) {
  const bad = [];
  const F = mkFs(root, overrides);
  const allowed = new Set(
    fixture ??
      JSON.parse(
        fs.readFileSync(
          path.join(UI_ROOT, 'scripts/fixtures/employee-allowed-commands.json'),
          'utf8',
        ),
      ),
  );

  // 1. замыкание
  const files = new Map(); // rel -> parsed
  const importers = new Map(); // rel -> Set(rel)
  const queue = [...ENTRIES];
  for (const e of ENTRIES) if (!F.exists(e)) bad.push(`нет входа ${e}`);
  while (queue.length) {
    const rel = queue.shift();
    if (files.has(rel) || !F.exists(rel)) continue;
    const pf = parseFile(rel, F.read(rel));
    files.set(rel, pf);
    for (const imp of pf.imports) {
      const r = resolveImport(F, rel, imp.spec);
      if (r.skip) continue;
      if (r.missing) {
        if (!/bindings[\w-]*$/.test(r.missing))
          bad.push(`${rel}: не разрешён импорт '${imp.spec}'`);
        continue;
      }
      if (/\/bindings[\w-]*\.ts$/.test(r.rel)) continue;
      imp.rel = r.rel;
      if (!importers.has(r.rel)) importers.set(r.rel, new Set());
      importers.get(r.rel).add(rel);
      queue.push(r.rel);
    }
  }

  // 2. обёртки
  const wrappers = new Map(); // rel -> { X: {m: cmd} }
  for (const [rel, pf] of files) {
    if (rel.endsWith('.svelte')) continue;
    const w = parseWrappers(pf.text);
    if (Object.keys(w).length) wrappers.set(rel, w);
  }
  const resolveExport = (rel, name, depth = 0) => {
    if (depth > 4) return null;
    if (wrappers.get(rel)?.[name]) return { file: rel, name };
    const pf = files.get(rel);
    if (!pf) return null;
    for (const m of pf.text.matchAll(/export\s*\{([^}]*)\}\s*from\s*['"]([^'"]+)['"]/g)) {
      for (const part of m[1].split(',')) {
        const a = /^\s*([\w$]+)(?:\s+as\s+([\w$]+))?\s*$/.exec(part);
        if (a && (a[2] ?? a[1]) === name) {
          const r = resolveImport(F, rel, m[2]);
          if (r.rel) return resolveExport(r.rel, a[1], depth + 1);
        }
      }
    }
    return null;
  };

  // функции сторов и их команды (приписываются вызывающему)
  const isStore = (rel) => rel.startsWith('src/lib/stores/');
  const fnInfo = new Map(); // rel -> parseFunctions
  const getFns = (rel) => {
    if (!fnInfo.has(rel)) fnInfo.set(rel, parseFunctions(files.get(rel).text));
    return fnInfo.get(rel);
  };

  // 3. сбор вызовов файла
  function callsInRange(pf, a, b) {
    const calls = []; // {p, cmd|null, what}
    const text = pf.text;
    const hasApiCall = pf.imports.some((i) => i.bindings.some((x) => x.local === 'apiCall'));
    const seg = text.slice(a, b);
    if (hasApiCall) {
      for (const m of seg.matchAll(/(?<![\w.$])apiCall\s*(?:<[^()]*?>)?\(\s*(['"])([\w]+)\1/g))
        calls.push({ p: a + m.index, cmd: m[2], what: `apiCall('${m[2]}')` });
      for (const m of seg.matchAll(/(?<![\w.$])apiCall\s*(?:<[^()]*?>)?\(\s*(?!['"\s])/g)) {
        calls.push({ p: a + m.index, cmd: null, what: 'apiCall(не литерал)' });
      }
    }
    for (const imp of pf.imports) {
      if (!imp.rel) continue;
      for (const bnd of imp.bindings) {
        const w = resolveExport(imp.rel, bnd.name);
        if (w) {
          const map = wrappers.get(w.file)[w.name];
          const re = new RegExp(
            `(?<![\\w.$])${bnd.local.replace(/\$/g, '\\$')}\\s*\\.\\s*(\\w+)\\s*\\(`,
            'g',
          );
          for (const m of seg.matchAll(re)) {
            const cmd = Object.hasOwn(map, m[1]) ? map[m[1]] : null;
            calls.push({
              p: a + m.index,
              cmd,
              what: cmd
                ? `${bnd.local}.${m[1]}() -> ${cmd}`
                : `${bnd.local}.${m[1]}() (метода нет в обёртке)`,
            });
          }
        } else if (isStore(imp.rel) && bnd.name !== '*' && files.get(imp.rel)) {
          const cmds = storeCommands(imp.rel, bnd.name);
          if (cmds.size === 0) continue;
          const re = new RegExp(`(?<![\\w.$])${bnd.local.replace(/\$/g, '\\$')}\\s*\\(`, 'g');
          for (const m of seg.matchAll(re))
            for (const c of cmds)
              calls.push({ p: a + m.index, cmd: c, what: `${bnd.local}() (стор) -> ${c}` });
        }
      }
    }
    return calls;
  }

  const storeMemo = new Map();
  function storeCommands(rel, name, seen = new Set()) {
    const key = rel + '#' + name;
    if (storeMemo.has(key)) return storeMemo.get(key);
    if (seen.has(key)) return new Set();
    seen.add(key);
    const pf = files.get(rel);
    const fns = getFns(rel);
    const fn = fns.find((f) => f.name === name);
    const out = new Set();
    if (fn) {
      for (const c of callsInRange(pf, fn.bodyStart, fn.bodyEnd)) {
        if (c.cmd && !scriptGuarded(pf.text, 0, c.p)) out.add(c.cmd);
        if (!c.cmd) out.add('<неизвестная>');
      }
      for (const other of fns) {
        if (other.name === name) continue;
        const body = pf.text.slice(fn.bodyStart, fn.bodyEnd);
        if (new RegExp(`(?<![\\w.$])${other.name}\\s*\\(`).test(body))
          for (const c of storeCommands(rel, other.name, seen)) out.add(c);
      }
    }
    storeMemo.set(key, out);
    return out;
  }

  // 4. оценка
  const wrapperFiles = new Set(wrappers.keys());
  const findingsByFile = new Map(); // rel -> [{line, text}] (нужна охрана монтирования)
  let totalCalls = 0;
  const addFinding = (rel, f) => {
    if (!findingsByFile.has(rel)) findingsByFile.set(rel, []);
    findingsByFile.get(rel).push(f);
  };

  for (const [rel, pf] of files) {
    if (wrapperFiles.has(rel)) continue;
    const text = pf.text;
    const fns = !pf.isSvelte && isStore(rel) ? getFns(rel) : [];
    const inFn = (p) => fns.some((f) => p >= f.bodyStart && p < f.bodyEnd);
    const seen = new Set();
    for (const c of callsInRange(pf, 0, text.length)) {
      const inScript = pf.scripts.some(([x, y]) => c.p >= x && c.p < y);
      if (pf.isSvelte && !inScript && pf.template[c.p] === ' ') continue; // стили
      const key = c.p + ':' + c.what;
      if (seen.has(key)) continue;
      seen.add(key);
      if (!pf.isSvelte && inFn(c.p)) continue; // атрибуция вызывающему
      {
        totalCalls++;
        const line = lineOf(text, c.p);
        if (!c.cmd) {
          bad.push(`${rel}:${line}: неизвестная команда: ${c.what}`);
          continue;
        }
        if (c.cmd === '<неизвестная>') {
          bad.push(`${rel}:${line}: стор вызывает неразрешимую команду (${c.what})`);
          continue;
        }
        if (allowed.has(c.cmd) || PUBLIC_COMMANDS.has(c.cmd)) continue;
        const guarded = inScript
          ? scriptGuarded(
              text,
              pf.isSvelte ? pf.scripts.find(([x, y]) => c.p >= x && c.p < y)[0] : 0,
              c.p,
            )
          : templateIfStack(pf.template, c.p);
        if (guarded) continue;
        if (pf.isSvelte) addFinding(rel, { line, what: c.what });
        else
          bad.push(
            `${rel}:${line}: ${c.what} без проверки роли (модуль общий для сотрудника и персонала)`,
          );
      }
    }
  }

  // 5. монтирование компонентов с неохраняемыми вызовами
  const compUsages = (rel) => {
    const res = []; // {file, p}
    for (const [f2, pf2] of files) {
      if (!pf2.isSvelte || f2 === rel) continue;
      for (const imp of pf2.imports) {
        if (imp.rel !== rel) continue;
        for (const bnd of imp.bindings) {
          if (bnd.name !== 'default') continue;
          for (const m of pf2.template.matchAll(new RegExp(`<${bnd.local}(?=[\\s/>])`, 'g')))
            res.push({ file: f2, p: m.index });
        }
      }
    }
    return res;
  };
  const mountMemo = new Map();
  const staffOnlyMounted = (rel, visiting = new Set()) => {
    if (mountMemo.has(rel)) return mountMemo.get(rel);
    if (visiting.has(rel)) return false;
    visiting.add(rel);
    const imps = importers.get(rel);
    let ok;
    if (!imps || imps.size === 0)
      ok = false; // вход
    else {
      const us = compUsages(rel);
      ok = us.every(
        (u) =>
          templateIfStack(files.get(u.file).template, u.p) || staffOnlyMounted(u.file, visiting),
      );
    }
    visiting.delete(rel);
    mountMemo.set(rel, ok);
    return ok;
  };
  for (const [rel, fl] of findingsByFile) {
    if (staffOnlyMounted(rel)) continue;
    const uses = compUsages(rel).filter(
      (u) => !templateIfStack(files.get(u.file).template, u.p) && !staffOnlyMounted(u.file),
    );
    const where = uses.length
      ? 'монтируется без охраны в ' + [...new Set(uses.map((u) => u.file))].join(', ')
      : 'вход или нет охраны монтирования';
    for (const f of fl) bad.push(`${rel}:${f.line}: ${f.what} без проверки роли; ${where}`);
  }

  // 6. счётные минимумы и здравость фикстуры
  const stats = { files: files.size, calls: totalCalls, wrappers: wrappers.size };
  if (files.size < MIN_FILES) bad.push(`замыкание пусто: ${files.size} файлов < ${MIN_FILES}`);
  if (totalCalls < MIN_CALLS)
    bad.push(`найдено вызовов ${totalCalls} < ${MIN_CALLS}: разбор молчит`);
  if (!(allowed.size > 0) || [...allowed].some((x) => typeof x !== 'string' || !x))
    bad.push('фикстура разрешённых команд пуста или содержит не строки');
  return { bad, stats };
}

// ---------- selftest ----------

function selftest() {
  let fail = 0;
  const base = check();
  if (base.bad.length) {
    console.error(`${TAG} selftest: чистое состояние красное:\n  ` + base.bad.join('\n  '));
    return 1;
  }
  const cases = [
    {
      name: 'ensureLoaded справочника статусов в бейдже (вариант до плана 43)',
      file: 'src/features/requests/RepairDeviceStatus.svelte',
      from: '<script lang="ts">',
      to: `<script lang="ts">\n  import { ensureLoaded as ensureDeviceStatuses } from '$lib/stores/deviceStatuses.svelte';\n  $effect(() => {\n    void ensureDeviceStatuses();\n  });`,
      expect: /RepairDeviceStatus\.svelte:\d+: .*device_statuses_list/,
    },
    {
      name: 'снято isSpecialist у попапов ремонта',
      file: 'src/features/requests/RequestDetail.svelte',
      from: '{#if isSpecialist && request !== null && isRepair && curState !== null}',
      to: '{#if request !== null && isRepair && curState !== null}',
      expect: /Repair\w*Modal\.svelte:\d+: .*монтируется без охраны/,
    },
    {
      name: 'снята проверка роли в loadOpenRepairs',
      file: 'src/features/requests/repairLocks.ts',
      from: 'if (ids.length === 0 || !canUseRepairs()) return map;',
      to: 'if (ids.length === 0) return map;',
      expect: /repairLocks\.ts:\d+: .*repairs_open_for_devices/,
    },
    {
      name: 'apiCall(device_statuses_list) в EmployeeLayout',
      file: 'src/features/layout/EmployeeLayout.svelte',
      from: '<script lang="ts">',
      to: `<script lang="ts">\n  void apiCall('device_statuses_list');`,
      expect: /EmployeeLayout\.svelte:\d+: .*device_statuses_list/,
    },
    {
      name: 'неизвестная команда (apiCall не литерал)',
      file: 'src/features/layout/EmployeeLayout.svelte',
      from: '<script lang="ts">',
      to: `<script lang="ts">\n  const cmdName = 'x';\n  void apiCall(cmdName);`,
      expect: /EmployeeLayout\.svelte:\d+: неизвестная команда/,
    },
    {
      name: 'метод обёртки, которого нет в карте',
      file: 'src/features/requests/RequestDetail.svelte',
      from: 'const st = await repairs.getState(id);',
      to: 'const st = await repairs.noSuchMethod(id);',
      expect: /RequestDetail\.svelte:\d+: неизвестная команда/,
    },
    {
      name: 'снята ранняя проверка роли в приёме заявки (обработчик)',
      file: 'src/features/requests/RequestDetail.svelte',
      from: '  async function startAccept() {\n    if (!isSpecialist) return;\n',
      to: '  async function startAccept() {\n',
      expect: /RequestDetail\.svelte:\d+: repairs\.accept/,
    },
    {
      name: 'снята проверка прав у восстановления вложения',
      file: 'src/features/attachments/AttachmentsBlock.svelte',
      from: '    if (!canManageDeleted) return;\n    try {\n      await attachments.restore(item.id);',
      to: '    try {\n      await attachments.restore(item.id);',
      expect: /AttachmentsBlock\.svelte:\d+: attachments\.restore/,
    },
  ];
  for (const c of cases) {
    const src = fs.readFileSync(path.join(UI_ROOT, c.file), 'utf8');
    const n = src.split(c.from).length - 1;
    if (n !== 1) {
      console.error(`${TAG} selftest «${c.name}»: якорь встречается ${n} раз (нужен ровно 1)`);
      fail++;
      continue;
    }
    const r = check({ overrides: new Map([[c.file, src.replace(c.from, () => c.to)]]) });
    if (!r.bad.some((l) => c.expect.test(l))) {
      console.error(
        `${TAG} selftest «${c.name}»: нет красного по своему правилу; получено:\n  ` +
          (r.bad.join('\n  ') || '(зелёный)'),
      );
      fail++;
    }
  }
  // пустая фикстура и пустое замыкание
  if (check({ fixture: [] }).bad.every((l) => !/фикстура/.test(l))) {
    console.error(`${TAG} selftest: пустая фикстура не краснит`);
    fail++;
  }
  if (fail === 0)
    console.log(
      `${TAG} selftest OK (${cases.length + 3} проверок, ${base.stats.files} файлов, ${base.stats.calls} вызовов)`,
    );
  return fail ? 1 : 0;
}

// ---------- CLI ----------

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--selftest')) process.exit(selftest());
  const overrides = new Map();
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--override') {
      const [rel, file] = args[++i].split('=');
      overrides.set(rel, fs.readFileSync(file, 'utf8'));
    }
  }
  const { bad, stats } = check({ overrides });
  if (bad.length) {
    console.error(`${TAG} FAIL (${bad.length}):\n  ` + bad.join('\n  '));
    process.exit(1);
  }
  console.log(`${TAG} OK (${stats.files} файлов, ${stats.calls} вызовов)`);
}
