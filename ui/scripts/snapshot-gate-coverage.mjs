#!/usr/bin/env node
// Снимает baseline набора файлов фронтенда в fixtures/gate-coverage.baseline.json.
//
// ЗАПУСКАТЬ ОДИН РАЗ, ДО любого переезда файлов (фаза 42, D-20). Повторный запуск
// после начала переездов запрещён: он затёр бы эталон потерянным набором.
// В файл попадают только пути кода из `git ls-files`.

import { execFileSync, spawnSync } from 'node:child_process';
import os from 'node:os';
import fs from 'node:fs';
import path from 'node:path';
import { REPO, UI_ROOT } from './_roots.mjs';

const DEST = path.join(UI_ROOT, 'scripts', 'fixtures', 'gate-coverage.baseline.json');

/** Рекурсивный список файлов каталога (относительно REPO), фильтр по regex имени. */
export function listDir(dirRel, pattern, recursive) {
  const re = new RegExp(pattern);
  const out = [];
  const rec = (d) => {
    for (const ent of fs.readdirSync(path.join(REPO, d), { withFileTypes: true })) {
      const rel = d + '/' + ent.name;
      if (ent.isDirectory()) {
        if (recursive && ent.name !== 'node_modules' && ent.name !== 'dist') rec(rel);
      } else if (re.test(ent.name)) out.push(rel);
    }
  };
  rec(dirRel);
  return out;
}

/**
 * Гейты, НЕ обходящие дерево через walk() (читают именованные файлы / нерекурсивный
 * каталог / все расширения). Их вход заморожен явно: files — обязаны существовать по
 * тем же путям, dirs — число подошедших файлов не должно падать ([C7]).
 */
const F = 'ui/src/features/';
const CHAT = F + 'requests/chat/';
const PINNED_SPEC = {
  'attachments-block': {
    files: [
      F + 'printers/PrinterDetail.svelte',
      F + 'cartridges/CartridgeDetail.svelte',
      F + 'places/PlaceEntityViewModal.svelte',
      'crates/trackly-app/src/services/attachment_service.rs',
    ],
    dirs: [{ dir: F + 'attachments', pattern: '\\.svelte$|^api\\.ts$', recursive: false }],
  },
  'chat-effects': {
    files: [
      CHAT + 'RequestChat.svelte',
      CHAT + 'ChatComposer.svelte',
      F + 'requests/RequestDetail.svelte',
      CHAT + 'draftStore.svelte.ts',
    ],
    dirs: [
      { dir: F + 'requests', pattern: '\\.svelte$', recursive: false },
      { dir: CHAT.slice(0, -1), pattern: '\\.svelte$', recursive: false },
    ],
  },
  'chat-ui': {
    files: [
      ...[
        'RequestChat.svelte',
        'ChatComposer.svelte',
        'ChatMessage.svelte',
        'ChatMuteToggle.svelte',
        'ChatEvent.svelte',
        'ChatAttachmentEvent.svelte',
        'chatNotifications.svelte.ts',
        'unreadStore.svelte.ts',
        'draftStore.svelte.ts',
        'tabStore.svelte.ts',
        'linkify.ts',
      ].map((n) => CHAT + n),
      F + 'requests/RequestDetail.svelte',
      F + 'layout/sidebar-config.ts',
      'ui/src/lib/components/DetailPanel.svelte',
      'ui/src/lib/platform/systemNotify.ts',
      F + 'requests/requestTarget.ts',
      F + 'layout/EmployeeLayout.svelte',
      F + 'requests/RequestsPage.svelte',
      'crates/trackly-app/src/tauri_cmds/system_notify.rs',
    ],
    dirs: [{ dir: CHAT.slice(0, -1), pattern: '\\.svelte$', recursive: false }],
  },
  'lint-selftest-wiring': {
    files: ['ui/package.json'],
    dirs: [{ dir: 'ui/scripts', pattern: '^check-.*\\.mjs$', recursive: false }],
  },
  'html-commands': {
    files: ['packages/transport/src/client.ts'],
    dirs: [{ dir: 'crates/trackly-app/src/http', pattern: '\\.rs$', recursive: false }],
  },
  'repair-modal-focus': {
    files: [],
    dirs: [{ dir: F + 'requests', pattern: '^Repair.*Modal\\.svelte$', recursive: false }],
  },
  'place-tree-invalidation': {
    files: [],
    dirs: [{ dir: 'ui/src', pattern: '.', recursive: true }],
  },
};

function perGate() {
  const base = JSON.parse(fs.readFileSync(DEST, 'utf8'));
  const moves = JSON.parse(
    fs.readFileSync(path.join(UI_ROOT, 'scripts', 'fixtures', 'move-map.json'), 'utf8'),
  ).moves;
  if (moves.length !== 0) {
    console.error('snapshot --per-gate: переезды уже начались (move-map не пуст) — запрещено');
    process.exit(1);
  }
  const dirty = execFileSync('git', ['status', '--porcelain', 'ui/src'], {
    cwd: REPO,
    encoding: 'utf8',
  }).trim();
  if (dirty) {
    console.error('snapshot --per-gate: ui/src не чист:\n' + dirty);
    process.exit(1);
  }
  const gates = fs
    .readdirSync(path.join(UI_ROOT, 'scripts'))
    .filter((f) => /^check-.*\.mjs$/.test(f) && f !== 'check-gate-coverage.mjs')
    .filter((f) => fs.readFileSync(path.join(UI_ROOT, 'scripts', f), 'utf8').includes('_roots.mjs'))
    .sort();
  const tmp = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'gate-cov-')), 'report.jsonl');
  const result = {};
  for (const g of gates) {
    fs.rmSync(tmp, { force: true });
    const r = spawnSync('node', ['scripts/' + g], {
      cwd: UI_ROOT,
      env: { ...process.env, GATE_COVERAGE_REPORT: tmp },
      encoding: 'utf8',
      timeout: 120000,
    });
    if (r.status !== 0) {
      console.error(`snapshot --per-gate: ${g} завершился кодом ${r.status}\n${r.stderr}`);
      process.exit(1);
    }
    const counts = fs.existsSync(tmp)
      ? fs
          .readFileSync(tmp, 'utf8')
          .split('\n')
          .filter(Boolean)
          .map((l) => JSON.parse(l).count)
      : [];
    const count = Math.max(0, ...counts);
    if (!(count > 0)) {
      console.error(`snapshot --per-gate: ${g} дал count 0`);
      process.exit(1);
    }
    result[g.replace(/^check-/, '').replace(/\.mjs$/, '')] = count;
  }
  const pinned = {};
  for (const [g, spec] of Object.entries(PINNED_SPEC)) {
    for (const f of spec.files) {
      if (!fs.existsSync(path.join(REPO, f))) {
        console.error(`snapshot --per-gate: pinned ${g}: нет файла ${f}`);
        process.exit(1);
      }
    }
    const dirs = spec.dirs.map((d) => {
      const count = listDir(d.dir, d.pattern, d.recursive).length;
      if (!(count > 0)) {
        console.error(`snapshot --per-gate: pinned ${g}: пустой ${d.dir}`);
        process.exit(1);
      }
      return { ...d, count };
    });
    pinned[g] = { files: spec.files, dirs };
  }
  base.perGate = result;
  base.pinned = pinned;
  fs.writeFileSync(DEST, JSON.stringify(base, null, 2) + '\n');
  console.log(
    `perGate: ${Object.keys(result).length} гейтов, pinned: ${Object.keys(pinned).length}`,
  );
}

function filesMode() {
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

  const dest = DEST;
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, JSON.stringify({ files, totals }, null, 2) + '\n');
  console.log(`baseline: ${files.length} файлов -> ${path.relative(REPO, dest)}`);
}

if (process.argv.includes('--per-gate')) perGate();
else filesMode();
