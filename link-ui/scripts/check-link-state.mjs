#!/usr/bin/env node
// [check-link-state] Гейт зеркала контракта Link (фаза 44, план 10).
//
// Зачем: svelte-check и vite build не видят расхождения TS-типов с Rust-контрактом
// (crates/link-app/src/state.rs). Источник истины — golden-фикстуры Rust-тестов.
//
// Правила (комментарии вырезаются):
//   [S1] каждый kind из link_state.json есть в union LinkState (linkState.ts) и в
//        switch screenFor (case 'kind');
//   [S2] каждое поле каждого состояния и поля события есть в linkState.ts;
//   [S3] каждая команда из link_commands.json (кроме link_info) вызывается в linkApi.ts
//        строкой 'имя_команды';
//   [S4] в link-ui/src/lib нет fetch( / XMLHttpRequest / WebSocket.
// Отсутствие файла — нарушение, не SKIP.
//
// Usage:
//   node scripts/check-link-state.mjs [--root=<копия>]
//   node scripts/check-link-state.mjs --selftest

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const REPO = path.resolve(path.dirname(__filename), '..', '..');

export function stripComments(text) {
  return text
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/[^\n]*/g, '$1');
}

const FILES = {
  state: 'link-ui/src/lib/linkState.ts',
  api: 'link-ui/src/lib/linkApi.ts',
  states: 'crates/link-app/tests/fixtures/link_state.json',
  commands: 'crates/link-app/tests/fixtures/link_commands.json',
};

export function check(root) {
  const violations = [];
  let checks = 0;
  const need = (ok, msg) => {
    checks++;
    if (!ok) violations.push(msg);
  };
  const read = (rel, code) => {
    const p = path.join(root, rel);
    if (!fs.existsSync(p)) {
      violations.push(`[${code}] нет файла ${rel}`);
      return null;
    }
    return fs.readFileSync(p, 'utf8');
  };

  const stateRaw = read(FILES.state, 'S1');
  const apiRaw = read(FILES.api, 'S3');
  const statesRaw = read(FILES.states, 'S1');
  const cmdsRaw = read(FILES.commands, 'S3');

  if (stateRaw !== null && statesRaw !== null) {
    const ts = stripComments(stateRaw);
    let fx = null;
    try {
      fx = JSON.parse(statesRaw);
    } catch {
      violations.push('[S1] link_state.json не разобрался как JSON');
    }
    if (fx) {
      const unionStart = ts.indexOf('export type LinkState =');
      const unionEnd = ts.indexOf('export type LinkStateKind');
      const union = unionStart >= 0 && unionEnd > unionStart ? ts.slice(unionStart, unionEnd) : '';
      need(union !== '', '[S1] в linkState.ts не найден union LinkState');
      const sw = ts.slice(ts.indexOf('function screenFor'));
      for (const [name, st] of Object.entries(fx.states ?? {})) {
        const kind = st.kind;
        need(
          new RegExp(`kind:\\s*'${kind}'`).test(union),
          `[S1] kind '${kind}' (состояние ${name}) отсутствует в union LinkState`,
        );
        need(
          new RegExp(`case\\s*'${kind}'`).test(sw),
          `[S1] kind '${kind}' отсутствует в switch screenFor`,
        );
        for (const field of Object.keys(st)) {
          if (field === 'kind') continue;
          need(
            new RegExp(`\\b${field}\\s*:`).test(union),
            `[S2] поле '${field}' состояния '${kind}' отсутствует в union LinkState`,
          );
        }
      }
      const iface = ts.slice(ts.indexOf('export interface LinkEvent'));
      for (const field of Object.keys(fx.event ?? {})) {
        need(
          new RegExp(`\\b${field}\\s*:`).test(iface),
          `[S2] поле события '${field}' отсутствует в LinkEvent`,
        );
      }
    }
  }

  if (apiRaw !== null && cmdsRaw !== null) {
    const api = stripComments(apiRaw);
    let cmds = [];
    try {
      cmds = JSON.parse(cmdsRaw);
    } catch {
      violations.push('[S3] link_commands.json не разобрался как JSON');
    }
    need(cmds.length > 0, '[S3] список команд пуст');
    for (const c of cmds) {
      if (c === 'link_info') continue;
      need(api.includes(`'${c}'`), `[S3] команда '${c}' не обёрнута в linkApi.ts`);
    }
  }

  const libDir = path.join(root, 'link-ui', 'src', 'lib');
  for (const f of fs.existsSync(libDir) ? fs.readdirSync(libDir) : []) {
    if (!/\.ts$/.test(f)) continue;
    const t = stripComments(fs.readFileSync(path.join(libDir, f), 'utf8'));
    for (const re of [/\bfetch\s*\(/, /XMLHttpRequest/, /\bWebSocket\b/]) {
      checks++;
      if (re.test(t)) violations.push(`[S4] lib/${f}: запрещённая конструкция ${re.source}`);
    }
  }
  return { violations, checks };
}

function copyTree(root) {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'link-state-'));
  for (const rel of [
    'link-ui/src/lib',
    'crates/link-app/tests/fixtures/link_state.json',
    'crates/link-app/tests/fixtures/link_commands.json',
  ]) {
    const to = path.join(tmp, rel);
    fs.mkdirSync(path.dirname(to), { recursive: true });
    fs.cpSync(path.join(root, rel), to, { recursive: true });
  }
  return tmp;
}

function selftest() {
  const base = check(REPO);
  if (base.violations.length) {
    console.error('[check-link-state] selftest: чистое дерево красное:\n' + base.violations.join('\n'));
    process.exit(1);
  }
  const mutants = [
    // [код, файл, что заменить, на что]; якорь обязан быть уникальным
    ['S1', FILES.states, '"kind": "revoked"', '"kind": "brand_new_kind"'],
    ['S1', FILES.state, "case 'revoked':\n", "case 'revoked_x':\n"],
    ['S2', FILES.states, '"after_logout": false', '"after_logout_new": false'],
    ['S2', FILES.states, '"secrets_persistent": false', '"secrets_persistent_new": false'],
    ['S3', FILES.commands, '"link_quit"', '"link_quit_new"'],
    ['S3', FILES.api, "'link_retry_now'", "'link_retry_later'"],
    [
      'S4',
      FILES.api,
      "export const quit",
      "const leak = () => fetch('/x');\nexport const quit",
    ],
  ];
  let bad = 0;
  for (const [code, rel, from, to] of mutants) {
    const tmp = copyTree(REPO);
    const p = path.join(tmp, rel);
    const src = fs.readFileSync(p, 'utf8');
    const n = src.split(from).length - 1;
    if (n !== 1) {
      console.error(`  ANCHOR ${rel}: «${from}» встречается ${n} раз (нужно ровно 1)`);
      bad++;
      continue;
    }
    fs.writeFileSync(p, src.replace(from, () => to));
    const hit = check(tmp).violations.some((v) => v.startsWith(`[${code}]`));
    console.log(`  ${hit ? 'ok  ' : 'FAIL'} мутант [${code}] ${path.basename(rel)}: «${from.trim()}»`);
    if (!hit) bad++;
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  if (bad) {
    console.error(`[check-link-state] selftest: провалов ${bad}`);
    process.exit(1);
  }
  console.log('[check-link-state] selftest: все мутанты пойманы');
}

if (process.argv[1] === __filename) {
  if (process.argv.includes('--selftest')) selftest();
  else {
    const arg = process.argv.find((a) => a.startsWith('--root='));
    const { violations, checks } = check(arg ? path.resolve(arg.slice(7)) : REPO);
    if (violations.length) {
      console.error('[check-link-state] FAIL\n' + violations.join('\n'));
      process.exit(1);
    }
    console.log(`[check-link-state] PASS (проверено признаков: ${checks})`);
  }
}
