#!/usr/bin/env node
// Кодмод переездов фронтенда в `packages/*` (фаза 42, план 09; D-20).
//
// Имя файла с `_`: это инструмент, не гейт (check-lint-selftest-wiring его не считает).
//
// Режимы:
//   --classify <from>            печатает импорты файла, указывающие на непакетный код
//                                (bindings*, auth, features/*, $lib/api/*, ui/-код вне
//                                переносимого набора). Такой файл переносить нельзя.
//   --apply <from>=<to> [...]    `git mv` файлов (пути repo-relative, например
//                                ui/src/lib/components/Button.svelte=
//                                packages/kit/src/components/Button.svelte), переписывает
//                                спецификаторы ($lib/... и относительные) во всех
//                                ui/src, packages/*/src, link-ui/src, дописывает записи
//                                в ui/scripts/fixtures/move-map.json.
//
// Формат нового спецификатора: `@trackly/<пакет>/<подпуть от packages/<пакет>/src>`;
// для .ts расширение опускается (`.svelte.ts` -> `.svelte`), для .svelte сохраняется.
// Внутри одного пакета (оба конца переехали в него) спецификатор остаётся относительным.
// Реэкспорт-прокладок в ui/ кодмод не создаёт; строки barrel-файлов (`index.ts`) на
// переехавший модуль удаляются (их потребители уже перенацелены на пакет).
// SCSS `@use` и пути в коде гейтов (ui/scripts) кодмод не трогает.

import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const REPO = path.resolve(UI_ROOT, '..');
const MOVE_MAP = path.join(__dirname, 'fixtures', 'move-map.json');
const EXTS = ['.ts', '.svelte', '.js', '.mjs'];
const SKIP = new Set(['node_modules', 'dist']);

const rel = (p) => path.relative(REPO, p).split(path.sep).join('/');

function listSources() {
  const roots = [path.join(UI_ROOT, 'src'), path.join(REPO, 'link-ui', 'src')];
  const pk = path.join(REPO, 'packages');
  if (fs.existsSync(pk)) {
    for (const n of fs.readdirSync(pk)) roots.push(path.join(pk, n, 'src'));
  }
  const out = [];
  const go = (d) => {
    if (!fs.existsSync(d)) return;
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      if (SKIP.has(e.name)) continue;
      const f = path.join(d, e.name);
      if (e.isDirectory()) go(f);
      else if (EXTS.includes(path.extname(e.name))) out.push(f);
    }
  };
  roots.forEach(go);
  return out;
}

/** Литералы-спецификаторы: import/export ... from '...', import('...'), import '...'. */
const SPEC_RE =
  /(\bfrom\s*|\bimport\s*\(\s*|\bimport\s+)(['"])([^'"\n]+)\2/g;

function resolveSpec(spec, importerAbs) {
  if (spec.startsWith('$lib/')) return path.join(UI_ROOT, 'src', 'lib', spec.slice(5));
  if (spec.startsWith('./') || spec.startsWith('../')) {
    return path.resolve(path.dirname(importerAbs), spec);
  }
  return null;
}

/** Совпадение разрешённого пути с файлом из набора (с добором расширения/index). */
function matchFile(abs, fileSet) {
  const cands = [abs, ...EXTS.map((e) => abs + e), ...EXTS.map((e) => path.join(abs, 'index' + e))];
  return cands.find((c) => fileSet.has(c)) ?? null;
}

export function pkgSpecifier(toRel) {
  const m = /^packages\/([^/]+)\/src\/(.+)$/.exec(toRel);
  if (!m) throw new Error(`[codemod] цель не внутри packages/<пакет>/src: ${toRel}`);
  let sub = m[2];
  if (sub.endsWith('.ts')) sub = sub.slice(0, -3);
  return `@trackly/${m[1]}/${sub}`;
}

const pkgOf = (r) => /^packages\/([^/]+)\//.exec(r)?.[1] ?? null;

function relativeSpec(fromAbs, toAbs) {
  let r = path.relative(path.dirname(fromAbs), toAbs).split(path.sep).join('/');
  if (!r.startsWith('.')) r = './' + r;
  return r;
}

function classify(fromRel) {
  const abs = path.join(REPO, fromRel);
  const text = fs.readFileSync(abs, 'utf8');
  const bad = [];
  for (const m of text.matchAll(SPEC_RE)) {
    const spec = m[3];
    const r = resolveSpec(spec, abs);
    const target = r ? rel(r) : spec;
    if (
      /(^|\/)bindings(\.|$)/.test(spec) ||
      /(^|\/)auth(\.svelte)?(\.ts)?$/.test(spec) ||
      /(^|\/)features\//.test(spec) ||
      target.startsWith('ui/src/features/') ||
      /\$lib\/api\//.test(spec)
    ) {
      bad.push(spec);
    }
  }
  return bad;
}

function main(argv) {
  const mode = argv[0];
  if (mode === '--classify') {
    let failed = false;
    for (const f of argv.slice(1)) {
      const bad = classify(f);
      console.log(`${f}: ${bad.length ? 'НЕЛЬЗЯ переносить: ' + bad.join(', ') : 'чисто'}`);
      if (bad.length) failed = true;
    }
    process.exit(failed ? 1 : 0);
  }
  if (mode !== '--apply') {
    console.error('Usage: _move-codemod.mjs --classify <from>... | --apply <from>=<to>...');
    process.exit(2);
  }
  const moves = argv.slice(1).map((a) => {
    const [from, to] = a.split('=');
    if (!from || !to) throw new Error(`[codemod] ожидалось from=to: ${a}`);
    return { from, to };
  });
  for (const mv of moves) {
    const bad = classify(mv.from);
    if (bad.length) {
      console.error(`[codemod] ${mv.from} тянет непакетный код: ${bad.join(', ')}`);
      process.exit(1);
    }
  }
  // Файлы до переезда (по старым путям).
  const files = listSources();
  const oldSet = new Set(files);
  const fromAbs = new Map(moves.map((m) => [path.join(REPO, m.from), m]));
  const newAbsOfOld = (abs) => (fromAbs.has(abs) ? path.join(REPO, fromAbs.get(abs).to) : abs);
  const oldAbsOfNew = new Map(moves.map((m) => [path.join(REPO, m.to), path.join(REPO, m.from)]));

  // Переписывание содержимого (читаем по старым путям, пишем в новые).
  const edits = [];
  for (const file of files) {
    const text = fs.readFileSync(file, 'utf8');
    const outAbs = newAbsOfOld(file);
    const importerMoved = fromAbs.has(file);
    const next = text.replace(SPEC_RE, (full, pre, q, spec) => {
      const resolved = resolveSpec(spec, file);
      if (!resolved) return full;
      const hit = matchFile(resolved, oldSet);
      if (!hit) {
        if (importerMoved && (spec.startsWith('./') || spec.startsWith('../'))) {
          throw new Error(`[codemod] ${rel(file)}: относительный импорт ${spec} не резолвится`);
        }
        return full;
      }
      const targetMoved = fromAbs.has(hit);
      if (!targetMoved) {
        if (importerMoved) {
          throw new Error(`[codemod] ${rel(file)} (переезжает) импортирует непереносимый ${rel(hit)}`);
        }
        return full;
      }
      const toRel = fromAbs.get(hit).to;
      let ns;
      if (importerMoved && pkgOf(rel(outAbs)) === pkgOf(toRel)) {
        ns = relativeSpec(outAbs, path.join(REPO, toRel));
      } else {
        ns = pkgSpecifier(toRel);
      }
      return `${pre}${q}${ns}${q}`;
    });
    if (next !== text) edits.push([file, outAbs, next]);
    else edits.push([file, outAbs, null]);
  }
  // barrel: убрать строки реэкспорта, нацеленные на переехавший модуль.
  for (const e of edits) {
    const [file, , next] = e;
    if (path.basename(file) !== 'index.ts') continue;
    const text = next ?? fs.readFileSync(file, 'utf8');
    const kept = text
      .split('\n')
      .filter((l) => !/^\s*export\b.*from\s*['"]@trackly\//.test(l) || !next);
    const cleaned = kept.join('\n');
    if (cleaned !== text) e[2] = cleaned;
  }
  // git mv, затем запись.
  for (const mv of moves) {
    fs.mkdirSync(path.dirname(path.join(REPO, mv.to)), { recursive: true });
    execFileSync('git', ['mv', mv.from, mv.to], { cwd: REPO, stdio: 'inherit' });
  }
  for (const [, outAbs, next] of edits) {
    if (next !== null) fs.writeFileSync(outAbs, next);
  }
  void oldAbsOfNew;
  // move-map
  const map = JSON.parse(fs.readFileSync(MOVE_MAP, 'utf8'));
  for (const mv of moves) {
    if (!map.moves.some((x) => x.from === mv.from)) map.moves.push({ from: mv.from, to: mv.to });
  }
  fs.writeFileSync(MOVE_MAP, JSON.stringify(map, null, 2) + '\n');
  console.log(`[codemod] перенесено файлов: ${moves.length}; правок содержимого: ${edits.filter((e) => e[2] !== null).length}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}
