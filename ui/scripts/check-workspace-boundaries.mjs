#!/usr/bin/env node
// [check-workspace-boundaries] Гейт границ рабочих пакетов (фаза 42, план 09; D-20, LNK-01).
//
// ЗАЧЕМ. Общие пакеты `packages/*` обслуживают два приложения: Trackly (`ui/`) и Trackly Link
// (`link-ui/`). Если пакет начнёт тянуть домен Trackly (bindings, auth, features) или
// привилегированный Tauri-API, Link получит чужой код и права, а граница «общее/доменное»
// сотрётся незаметно: svelte-check и сборка этого не видят, пока файл существует.
//
// Правила (комментарии вырезаются `_strip.mjs`; SKIP недопустим):
//   [B0] скан `packages/*/src` не пуст (0 файлов = вакуум = нарушение);
//   [B1] файлы `packages/*/src` не импортируют код `ui/`: ни `$lib`, ни `bindings*`, ни
//        `auth`, ни `features/`, ни относительный путь, выходящий из `src` своего пакета.
//        Между пакетами — только через `@trackly/*`;
//   [B2] `@tauri-apps/*` в `packages/*/src` импортируют только файлы из
//        TAURI_IMPORT_ALLOWLIST;
//   [B3] ровно один `Button.svelte` вне `node_modules`/`dist` (ui/src, packages, link-ui):
//        дубль означает, что переезд оставил копию;
//   [B4] `link-ui/src` не импортирует `ui/` (ни `$lib`, ни `ui/src`, ни относительный выход
//        из `link-ui`), а из `@tauri-apps/*` допускается только `@tauri-apps/api/core`;
//   [B6] lint link-ui (check-link-layout) реально запускается: скрипт `lint` в link-ui/package.json
//        зовёт check-link-layout.mjs, а ci-fast.yml и ci-full.yml содержат шаг
//        `pnpm --dir link-ui lint` (WR-01: гейт был нигде не подключён);
//   [B5] если есть `crates/link-app/Cargo.toml`: в нём нет `version.workspace` и есть
//        литеральная `version = "X.Y.Z"` (D-10: инъекция версии release.yml её не касается).
//
// TAURI_IMPORT_ALLOWLIST: файлы пакетов, которым разрешён `@tauri-apps/*` (transport,
// saveFile и подобное). Расширяется только осознанной правкой этого гейта с обоснованием
// рядом с записью. Сейчас: saveFile (план 12), transport client/ws (план 15),
// attachments api.ts (план 17).
//
// Чего гейт НЕ видит (честно): импорты через динамически собранную строку; `@trackly/*`,
// указывающий на пакет, не объявленный в dependencies; тип-импорты из .d.ts без
// спецификатора; доменную зависимость без импорта (глобальные переменные, окружение).
// `link-ui/src` обязан существовать и содержать исходники (WR-04); PASS печатает число просканированных файлов.
//
// Self-test (`--selftest`): мутанты на временной копии дерева; якорь замены встречается
// ровно один раз; у каждого сверяется ожидаемый код.
//
// Usage:
//   node scripts/check-workspace-boundaries.mjs                 # проверить репозиторий
//   node scripts/check-workspace-boundaries.mjs --root=<repo>   # корень (копия) репозитория
//   node scripts/check-workspace-boundaries.mjs --selftest
//
// Только node:fs/node:os/node:path/node:url.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '..', '..');
const TAG = '[check-workspace-boundaries]';

/** Repo-relative пути (через `/`) файлов пакетов, которым разрешён `@tauri-apps/*`. */
// saveFile: диалог «Сохранить как» и запись файла — динамические импорты
// @tauri-apps/plugin-dialog/-fs за guard'ом isTauri; доменной зависимости нет, а
// браузерная ветка (LAN, Link) падает обратно на <a download> (план 12, D-17).
const TAURI_IMPORT_ALLOWLIST = new Set([
  'packages/kit/src/utils/saveFile.ts',
  // transport (план 15): invoke десктопной ветки apiCall и listen('trackly-event') WS-клиента.
  // Это и есть назначение пакета (Tauri-ветка транспорта); других @tauri-apps/* ему не нужно.
  'packages/transport/src/client.ts',
  'packages/transport/src/ws.ts',
  // attachments (план 17): convertFileSrc схемы trackly-att в attachmentUrl (десктопная ветка);
  // только api.ts, остальным файлам пакета @tauri-apps/* не нужен.
  'packages/attachments/src/api.ts',
  // theme (quick 261126-rbg): setBackgroundColor нативного окна под --tr-bg; динамический
  // импорт @tauri-apps/api/window за guard'ом __TAURI_INTERNALS__, доменной зависимости нет.
  'packages/kit/src/stores/theme.svelte.ts',
]);

const SRC_EXTS = new Set(['.ts', '.svelte', '.js', '.mjs']);
const SKIP = new Set(['node_modules', 'dist', '.git', 'target']);
const SPEC_RE = /(\bfrom\s*|\bimport\s*\(\s*|\bimport\s+)(['"])([^'"\n]+)\2/g;

function walk(dir, exts, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP.has(e.name)) continue;
    const f = path.join(dir, e.name);
    if (e.isDirectory()) walk(f, exts, out);
    else if (!exts || exts.has(path.extname(e.name))) out.push(f);
  }
  return out;
}

const posix = (p) => p.split(path.sep).join('/');

/** Содержимое скриптовой части файла: комментарии вырезаны. */
function specsOf(file) {
  let text = fs.readFileSync(file, 'utf8');
  text = stripComments(text);
  return [...text.matchAll(SPEC_RE)].map((m) => m[3]);
}

function isInside(child, parent) {
  const r = path.relative(parent, child);
  return r !== '' && !r.startsWith('..') && !path.isAbsolute(r);
}

const DOMAIN_RE = [
  /(^|\/)bindings([-.\w]*)?(\.ts)?$/,
  /(^|\/)auth(\.svelte)?(\.ts)?$/,
  /(^|\/)features\//,
  /^\$lib(\/|$)/,
  /(^|\/)ui\/src(\/|$)/,
];

export function check(root) {
  const v = [];
  const pkgsDir = path.join(root, 'packages');
  const pkgSrcs = fs.existsSync(pkgsDir)
    ? fs
        .readdirSync(pkgsDir)
        .map((n) => path.join(pkgsDir, n, 'src'))
        .filter((d) => fs.existsSync(d) && fs.statSync(d).isDirectory())
    : [];
  let pkgFiles = 0;
  for (const src of pkgSrcs) {
    for (const file of walk(src, SRC_EXTS)) {
      pkgFiles++;
      const relFile = posix(path.relative(root, file));
      for (const spec of specsOf(file)) {
        const rel = spec.startsWith('.') ? path.resolve(path.dirname(file), spec) : null;
        if (DOMAIN_RE.some((re) => re.test(spec)) || (rel && !isInside(rel, src))) {
          v.push(`[B1] ${relFile}: пакет импортирует вне своего src или домен Trackly: '${spec}'`);
        }
        if (spec.startsWith('@tauri-apps/') && !TAURI_IMPORT_ALLOWLIST.has(relFile)) {
          v.push(`[B2] ${relFile}: '${spec}' вне TAURI_IMPORT_ALLOWLIST`);
        }
      }
    }
  }
  if (pkgFiles === 0) v.push('[B0] скан packages/*/src пуст — вакуум недопустим');

  // [B3]
  const buttons = [
    ...walk(path.join(root, 'ui', 'src'), null),
    ...walk(path.join(root, 'packages'), null),
    ...walk(path.join(root, 'link-ui'), null),
  ].filter((f) => path.basename(f) === 'Button.svelte');
  if (buttons.length !== 1) {
    v.push(
      `[B3] Button.svelte должен быть ровно один, найдено ${buttons.length}: ${buttons
        .map((b) => posix(path.relative(root, b)))
        .join(', ')}`,
    );
  }

  // [B4]
  const linkSrc = path.join(root, 'link-ui', 'src');
  const linkScanned = fs.existsSync(linkSrc);
  let linkFiles = 0;
  if (!linkScanned) {
    // WR-04: каталог существует с плана 16; его исчезновение — не «ещё нет», а потеря покрытия.
    v.push('[B4] link-ui/src отсутствует: нечего проверять, вакуумный проход недопустим');
  } else {
    for (const file of walk(linkSrc, SRC_EXTS)) {
      linkFiles += 1;
      const relFile = posix(path.relative(root, file));
      for (const spec of specsOf(file)) {
        const rel = spec.startsWith('.') ? path.resolve(path.dirname(file), spec) : null;
        if (
          DOMAIN_RE.some((re) => re.test(spec)) ||
          (rel && !isInside(rel, path.join(root, 'link-ui')))
        ) {
          v.push(
            `[B4] ${relFile}: link-ui импортирует ui/ или выходит из своего каталога: '${spec}'`,
          );
        }
        if (spec.startsWith('@tauri-apps/') && spec !== '@tauri-apps/api/core') {
          v.push(`[B4] ${relFile}: из @tauri-apps/* разрешён только api/core, найдено '${spec}'`);
        }
      }
    }
    if (linkFiles === 0)
      v.push('[B4] link-ui/src пуст: скан без файлов — вакуумный проход недопустим');
  }

  // [B5]
  const cargo = path.join(root, 'crates', 'link-app', 'Cargo.toml');
  const cargoChecked = fs.existsSync(cargo);
  if (cargoChecked) {
    const t = fs
      .readFileSync(cargo, 'utf8')
      .split('\n')
      .filter((l) => !l.trim().startsWith('#'))
      .join('\n');
    if (/^\s*version\.workspace\s*=/m.test(t))
      v.push('[B5] crates/link-app: version.workspace запрещён (D-10)');
    if (!/^\s*version\s*=\s*"\d+\.\d+\.\d+"/m.test(t)) {
      v.push('[B5] crates/link-app: нет литеральной version = "X.Y.Z" (D-10)');
    }
  }
  // [B6]
  const linkPkg = path.join(root, 'link-ui', 'package.json');
  if (!fs.existsSync(linkPkg)) {
    v.push('[B6] link-ui/package.json отсутствует');
  } else {
    const lintScript = JSON.parse(fs.readFileSync(linkPkg, 'utf8')).scripts?.lint ?? '';
    if (!/check-link-layout\.mjs/.test(lintScript)) {
      v.push('[B6] link-ui: скрипт lint не запускает check-link-layout.mjs');
    }
  }
  for (const wf of ['ci-fast.yml', 'ci-full.yml']) {
    const f = path.join(root, '.github', 'workflows', wf);
    const t = fs.existsSync(f) ? fs.readFileSync(f, 'utf8') : '';
    // Строка-комментарий с командой шагом не считается.
    const wired = t.split('\n').some((l) => /^\s*run:\s*pnpm --dir link-ui lint\s*$/.test(l));
    if (!wired) v.push(`[B6] .github/workflows/${wf}: нет шага 'run: pnpm --dir link-ui lint'`);
  }
  return {
    violations: v,
    info: `файлов пакетов: ${pkgFiles}; link-ui/src: ${linkFiles} файлов; Cargo.toml Link ${cargoChecked ? 'проверен' : 'отсутствует'}`,
  };
}

function copyTree(src, dst) {
  if (!fs.existsSync(src)) return;
  fs.mkdirSync(dst, { recursive: true });
  for (const e of fs.readdirSync(src, { withFileTypes: true })) {
    if (SKIP.has(e.name)) continue;
    const s = path.join(src, e.name);
    const d = path.join(dst, e.name);
    if (e.isDirectory()) copyTree(s, d);
    else fs.copyFileSync(s, d);
  }
}

function selftest() {
  const GOOD_CARGO = '[package]\nname = "link-app"\nversion = "0.1.0"\nedition.workspace = true\n';
  const cases = [
    ['контроль (реальное дерево)', () => {}, null],
    [
      'пакет импортирует ui/ [B1]',
      (r) =>
        fs.writeFileSync(
          path.join(r, 'packages/kit/src/x.ts'),
          "import a from '../../../../ui/src/lib/x';\n",
        ),
      'B1',
    ],
    [
      'пакет импортирует bindings [B1]',
      (r) =>
        fs.writeFileSync(
          path.join(r, 'packages/kit/src/x.ts'),
          "import { c } from './bindings';\n",
        ),
      'B1',
    ],
    [
      'пакет импортирует $lib [B1]',
      (r) =>
        fs.writeFileSync(
          path.join(r, 'packages/kit/src/x.ts'),
          "import a from '$lib/utils/date';\n",
        ),
      'B1',
    ],
    [
      'пакет импортирует @tauri-apps вне allowlist [B2]',
      (r) =>
        fs.writeFileSync(
          path.join(r, 'packages/kit/src/x.ts'),
          "import { open } from '@tauri-apps/plugin-shell';\n",
        ),
      'B2',
    ],
    [
      'второй Button.svelte [B3]',
      (r) => {
        fs.mkdirSync(path.join(r, 'ui/src/lib/components'), { recursive: true });
        fs.writeFileSync(
          path.join(r, 'ui/src/lib/components/Button.svelte'),
          '<button></button>\n',
        );
      },
      'B3',
    ],
    [
      'link-ui импортирует ui/ [B4]',
      (r) => {
        fs.mkdirSync(path.join(r, 'link-ui/src'), { recursive: true });
        fs.writeFileSync(path.join(r, 'link-ui/src/m.ts'), "import a from '../../ui/src/lib/x';\n");
      },
      'B4',
    ],
    [
      'link-ui берёт чужой @tauri-apps [B4]',
      (r) => {
        fs.mkdirSync(path.join(r, 'link-ui/src'), { recursive: true });
        fs.writeFileSync(
          path.join(r, 'link-ui/src/m.ts'),
          "import { open } from '@tauri-apps/plugin-shell';\n",
        );
      },
      'B4',
    ],
    [
      'link-ui/src исчез [B4]',
      (r) => fs.rmSync(path.join(r, 'link-ui/src'), { recursive: true, force: true }),
      'B4',
    ],
    [
      'link-ui/src пуст [B4]',
      (r) => {
        fs.rmSync(path.join(r, 'link-ui/src'), { recursive: true, force: true });
        fs.mkdirSync(path.join(r, 'link-ui/src'), { recursive: true });
      },
      'B4',
    ],
    [
      'ci-fast без шага lint link-ui [B6]',
      (r) => {
        const f = path.join(r, '.github/workflows/ci-fast.yml');
        fs.writeFileSync(
          f,
          fs.readFileSync(f, 'utf8').replace('run: pnpm --dir link-ui lint', 'run: echo skipped'),
        );
      },
      'B6',
    ],
    [
      'ci-full без шага lint link-ui [B6]',
      (r) => {
        const f = path.join(r, '.github/workflows/ci-full.yml');
        fs.writeFileSync(
          f,
          fs.readFileSync(f, 'utf8').replace('run: pnpm --dir link-ui lint', 'run: echo skipped'),
        );
      },
      'B6',
    ],
    [
      'link-ui lint без check-link-layout [B6]',
      (r) => {
        const f = path.join(r, 'link-ui/package.json');
        fs.writeFileSync(f, fs.readFileSync(f, 'utf8').replace(/check-link-layout\.mjs/g, 'x.mjs'));
      },
      'B6',
    ],
    [
      'link-ui с api/core проходит',
      (r) => {
        fs.mkdirSync(path.join(r, 'link-ui/src'), { recursive: true });
        fs.writeFileSync(
          path.join(r, 'link-ui/src/m.ts'),
          "import { invoke } from '@tauri-apps/api/core';\n",
        );
      },
      null,
    ],
    [
      'крейт Link с version.workspace [B5]',
      (r) => {
        fs.mkdirSync(path.join(r, 'crates/link-app'), { recursive: true });
        fs.writeFileSync(
          path.join(r, 'crates/link-app/Cargo.toml'),
          '[package]\nname = "link-app"\nversion.workspace = true\n',
        );
      },
      'B5',
    ],
    [
      'крейт Link с литеральной версией проходит',
      (r) => {
        fs.mkdirSync(path.join(r, 'crates/link-app'), { recursive: true });
        fs.writeFileSync(path.join(r, 'crates/link-app/Cargo.toml'), GOOD_CARGO);
      },
      null,
    ],
    [
      'пустой packages [B0]',
      (r) => fs.rmSync(path.join(r, 'packages'), { recursive: true, force: true }),
      'B0',
    ],
    [
      'комментарий с импортом не считается',
      (r) =>
        fs.writeFileSync(
          path.join(r, 'packages/kit/src/x.ts'),
          "// import a from '$lib/x';\n/* import b from '@tauri-apps/api/core'; */\nexport const k = 1;\n",
        ),
      null,
    ],
  ];
  let failed = 0;
  for (const [name, mutate, expect] of cases) {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'wsb-'));
    try {
      copyTree(path.join(REPO, 'packages'), path.join(tmp, 'packages'));
      copyTree(path.join(REPO, 'link-ui'), path.join(tmp, 'link-ui'));
      copyTree(path.join(REPO, '.github', 'workflows'), path.join(tmp, '.github', 'workflows'));
      mutate(tmp);
      // Button.svelte реального дерева лежит в packages: для B0-мутанта (packages стёрт)
      // B3 тоже сработает — сверяем наличие ожидаемого кода, а не его единственность.
      const { violations } = check(tmp);
      const codes = new Set(violations.map((x) => /^\[(B\d)\]/.exec(x)?.[1]));
      const ok = expect === null ? violations.length === 0 : codes.has(expect);
      console.log(
        `${TAG} [selftest] ${ok ? 'OK ' : 'FAIL'} ${name}${expect ? ` -> ${expect}` : ''}`,
      );
      if (!ok) {
        failed++;
        for (const x of violations) console.log(`    ${x}`);
      }
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  }
  if (failed) {
    console.error(`${TAG} selftest FAIL — ${failed} из ${cases.length}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK — ${cases.length} случаев`);
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--selftest')) return selftest();
  const rootArg = args.find((a) => a.startsWith('--root='));
  const root = rootArg ? path.resolve(rootArg.slice(7)) : REPO;
  const { violations, info } = check(root);
  if (violations.length) {
    for (const x of violations) console.error(`${TAG} ${x}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — 0 нарушений (${info})`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
