#!/usr/bin/env node
// [check-action-menu-portal] Постоянный гейт «меню действий всегда выводится
// порталом» (фаза 41, план 41-27, GRP-03 / GRP-04).
//
// Почему он существует: UAT фазы 41 (тест 5, GAP-1) — меню «⋯» в таблице
// свойств типа раздвигало прокручиваемый DetailPanel. Корень — не забытый флаг
// в одном вызове, а opt-in дефолт: портал включался пропом, и каждый новый
// вызов по умолчанию получал непортальную панель (`position: absolute` внутри
// контейнера с overflow). Чинится дефолт компонента: портал — единственный
// режим ActionMenu. Компиляционные гейты (svelte-check/eslint/build) этого
// не удерживают: возврат ветки «без портала» остаётся валидным кодом.
//
// Правила (сообщение начинается с кода правила в квадратных скобках):
//   [A] в разметке ActionMenu ровно один элемент role="menu"; его открывающий
//       тег содержит и `use:portal`, и `use:actionMenuPortalPosition`;
//   [B] в script ActionMenu нет `usePortal` и объявления пропа `portal`,
//       в разметке нет `{#if usePortal}`;
//   [C] в style ActionMenu нет `position: absolute`, а правило
//       `.action-menu-panel` содержит `position: fixed`;
//   [D] ни у одного `<ActionMenu` в src/**/*.svelte нет атрибута `portal`
//       (ни голого, ни `portal=...`, ни `portal={false}`); многострочные теги
//       читаются с учётом фигурных скобок и кавычек;
//   [E] защита от вакуумности: найдено не меньше MIN_CALL_SITES вызовов
//       `<ActionMenu`, иначе гейт ничего не видит;
//   [F] файл с правилом `.row-actions { opacity: 0 }` обязан содержать форму
//       `:has(:global([aria-expanded='true']))`. Форма без `:global` мёртвая:
//       триггер `[aria-expanded]` рендерит дочерний компонент, Svelte не видит
//       потомков из других компонентов, помечает селектор неиспользуемым
//       (`css_unused_selector`, лишь warning) и вырезает правило из выдачи.
//
// Комментарии (HTML, JS, SCSS) перед проверкой затираются с сохранением длины
// и переводов строк, поэтому цитаты форм в комментариях гейт не обманывают, а
// номера строк в сообщениях честные.
//
// Self-test (`--selftest`): корректная фикстура в памяти (обязана пройти) и 9
// мутантов, каждый получен заменой с ПРОВЕРКОЙ, что якорь встречается ровно
// один раз. Негативная фикстура обязана провалить гейт ТЕМ правилом, ради
// которого написана.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-action-menu-portal.mjs              # проверить репозиторий
//   node scripts/check-action-menu-portal.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-action-menu-portal.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-action-menu-portal]';

const MENU_FILE = 'src/lib/components/ActionMenu.svelte';
// Сколько вызовов <ActionMenu есть на момент написания гейта (инвентарь плана 41-27).
const MIN_CALL_SITES = 13;
const SELFTEST_MIN_CALL_SITES = 2;

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
}

/** Затирает JS/SCSS-комментарии, не трогая строковые литералы; длина сохраняется. */
function stripComments(src) {
  let out = '';
  let i = 0;
  let mode = 'code';
  while (i < src.length) {
    const c = src[i];
    const next = src[i + 1];
    if (mode === 'code') {
      if (c === '/' && next === '/') {
        mode = 'line';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === '/' && next === '*') {
        mode = 'block';
        out += '  ';
        i += 2;
        continue;
      }
      if (c === "'" || c === '"' || c === '`') mode = c;
      out += c;
      i += 1;
      continue;
    }
    if (mode === 'line') {
      if (c === '\n') {
        mode = 'code';
        out += c;
      } else out += ' ';
      i += 1;
      continue;
    }
    if (mode === 'block') {
      if (c === '*' && next === '/') {
        mode = 'code';
        out += '  ';
        i += 2;
        continue;
      }
      out += c === '\n' ? '\n' : ' ';
      i += 1;
      continue;
    }
    if (c === '\\') {
      out += c + (next ?? '');
      i += 2;
      continue;
    }
    if (c === '\n' && mode !== '`') mode = 'code';
    else if (c === mode) mode = 'code';
    out += c;
    i += 1;
  }
  return out;
}

/** Делит .svelte на markup / script / style (каждый — той же длины, остальное затёрто). */
function splitSvelte(src) {
  const blocks = [];
  const collect = (re, kind) => {
    for (const m of src.matchAll(re)) {
      blocks.push({ kind, start: m.index, end: m.index + m[0].length });
    }
  };
  collect(/<script\b[^>]*>[\s\S]*?<\/script>/g, 'script');
  collect(/<style\b[^>]*>[\s\S]*?<\/style>/g, 'style');
  const only = (kind) => {
    let out = blank(src);
    for (const b of blocks.filter((x) => x.kind === kind)) {
      out = out.slice(0, b.start) + src.slice(b.start, b.end) + out.slice(b.end);
    }
    return stripComments(out);
  };
  let markup = src;
  for (const b of blocks) {
    markup = markup.slice(0, b.start) + blank(src.slice(b.start, b.end)) + markup.slice(b.end);
  }
  markup = markup.replace(/<!--[\s\S]*?-->/g, (m) => blank(m));
  return { markup, script: only('script'), style: only('style') };
}

/** Конец открывающего тега: `>` вне фигурных скобок и кавычек. Возвращает индекс `>`. */
function tagEnd(src, start) {
  let depth = 0;
  let quote = null;
  for (let i = start + 1; i < src.length; i++) {
    const c = src[i];
    if (quote) {
      if (c === quote) quote = null;
      continue;
    }
    if (depth === 0 && (c === '"' || c === "'")) quote = c;
    else if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return i;
  }
  return -1;
}

/** Тег с затёртым содержимым кавычек и фигурных скобок — остаются только имена атрибутов. */
function attributeSkeleton(tag) {
  let out = '';
  let depth = 0;
  let quote = null;
  for (const c of tag) {
    if (quote) {
      if (c === quote) quote = null;
      out += c === '\n' ? '\n' : ' ';
      continue;
    }
    if (depth > 0) {
      if (c === '{') depth++;
      else if (c === '}') depth--;
      out += c === '\n' ? '\n' : ' ';
      continue;
    }
    if (c === '"' || c === "'") {
      quote = c;
      out += ' ';
    } else if (c === '{') {
      depth = 1;
      out += ' ';
    } else out += c;
  }
  return out;
}

/** Тело парных скобок, начиная с индекса открывающей. */
function extractBalanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return { body: src.slice(openIdx + 1, i), start: openIdx + 1 };
    }
  }
  return null;
}

function lineOf(src, idx) {
  return src.slice(0, idx).split('\n').length;
}

/**
 * Чистая функция сканирования: `files` — Map «POSIX-путь от ui/ -> текст».
 * Переиспользуется и реальным сканированием, и self-testом.
 */
function scanSources(files, minCallSites, violations) {
  const add = (file, line, rule, msg) =>
    violations.push(`${file}${line ? `:${line}` : ''} — [${rule}] ${msg}`);

  // --- ActionMenu: правила A, B, C ------------------------------------------
  const menuSrc = files.get(MENU_FILE);
  if (menuSrc === undefined) {
    add(MENU_FILE, 0, 'A', 'файл не найден — гейт не может проверить компонент.');
  } else {
    const { markup, script, style } = splitSvelte(menuSrc);

    const roleMatches = [...markup.matchAll(/role\s*=\s*"menu"/g)];
    if (roleMatches.length !== 1) {
      add(
        MENU_FILE,
        roleMatches[1] ? lineOf(markup, roleMatches[1].index) : 0,
        'A',
        `в разметке ${roleMatches.length} элементов role="menu", ожидается ровно один (единственная портальная панель).`,
      );
    } else {
      const idx = roleMatches[0].index;
      const start = markup.lastIndexOf('<', idx);
      const end = tagEnd(markup, start);
      const tag = end === -1 ? '' : markup.slice(start, end + 1);
      if (!/\buse:portal\b/.test(tag)) {
        add(MENU_FILE, lineOf(markup, idx), 'A', 'у панели role="menu" нет `use:portal`.');
      }
      if (!/\buse:actionMenuPortalPosition\b/.test(tag)) {
        add(
          MENU_FILE,
          lineOf(markup, idx),
          'A',
          'у панели role="menu" нет `use:actionMenuPortalPosition`.',
        );
      }
    }

    const up = /\busePortal\b/.exec(script);
    if (up)
      add(
        MENU_FILE,
        lineOf(script, up.index),
        'B',
        '`usePortal` в script — режим без портала вернулся.',
      );
    const pp = /\bportal\??\s*:\s*boolean/.exec(script);
    if (pp)
      add(
        MENU_FILE,
        lineOf(script, pp.index),
        'B',
        'проп `portal` снова объявлен — портал должен быть единственным режимом.',
      );
    const ip = /\{#if\s+usePortal\b/.exec(markup);
    if (ip)
      add(
        MENU_FILE,
        lineOf(markup, ip.index),
        'B',
        'в разметке `{#if usePortal}` — непортальная ветка вернулась.',
      );

    const abs = /position\s*:\s*absolute/.exec(style);
    if (abs)
      add(
        MENU_FILE,
        lineOf(style, abs.index),
        'C',
        '`position: absolute` в style — панель снова привязана к контейнеру.',
      );
    const ruleM = /\.action-menu-panel\s*\{/.exec(style);
    const rule = ruleM ? extractBalanced(style, ruleM.index + ruleM[0].length - 1, '{', '}') : null;
    if (!rule) {
      add(
        MENU_FILE,
        0,
        'C',
        'не найдено правило `.action-menu-panel` — гейт не может проверить `position: fixed`.',
      );
    } else if (!/position\s*:\s*fixed/.test(rule.body)) {
      add(
        MENU_FILE,
        lineOf(style, ruleM.index),
        'C',
        'в `.action-menu-panel` нет `position: fixed`.',
      );
    }
  }

  // --- вызовы: D, E; видимость триггера: F -----------------------------------
  let callSites = 0;
  for (const [file, src] of files) {
    const { markup, style } = splitSvelte(src);

    if (file !== MENU_FILE) {
      for (const m of markup.matchAll(/<ActionMenu(?=[\s/>])/g)) {
        callSites++;
        const end = tagEnd(markup, m.index);
        const tag = end === -1 ? markup.slice(m.index) : markup.slice(m.index, end + 1);
        if (/(^|\s)portal(?=\s|=|\/|>|$)/.test(attributeSkeleton(tag))) {
          add(
            file,
            lineOf(markup, m.index),
            'D',
            'у вызова <ActionMenu атрибут `portal` — проп удалён, портал и так единственный режим.',
          );
        }
      }
    }

    for (const m of style.matchAll(/\.row-actions\s*\{/g)) {
      const rule = extractBalanced(style, m.index + m[0].length - 1, '{', '}');
      if (!rule || !/(^|[;{}\s])opacity\s*:\s*0\s*;/.test(rule.body)) continue;
      const line = lineOf(style, m.index);
      if (/:has\(\s*:global\(\s*\[aria-expanded\s*=\s*(['"])true\1\s*\]\s*\)\s*\)/.test(rule.body))
        continue;
      if (/:has\(\s*\[aria-expanded/.test(rule.body)) {
        add(
          file,
          line,
          'F',
          "форма `:has([aria-expanded=…])` без `:global` мёртвая: триггер рендерит дочерний ActionMenu, Svelte вырежет селектор. Нужно `:has(:global([aria-expanded='true']))`.",
        );
      } else {
        add(
          file,
          line,
          'F',
          "`.row-actions { opacity: 0 }` без `&:has(:global([aria-expanded='true']))` — триггер строки исчезнет, пока меню открыто.",
        );
      }
    }
  }

  if (callSites < minCallSites) {
    add(
      'src',
      0,
      'E',
      `найдено ${callSites} вызовов <ActionMenu, ожидается не меньше ${minCallSites} — гейт ничего не видит (переименован компонент или изменился разбор).`,
    );
  }
}

function walk(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.name.endsWith('.svelte')) out.push(p);
  }
  return out;
}

function checkRepo(root, violations) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) {
    violations.push(`src: каталог не найден (${srcDir}).`);
    return;
  }
  const files = new Map();
  for (const abs of walk(srcDir)) {
    files.set(path.relative(root, abs).split(path.sep).join('/'), fs.readFileSync(abs, 'utf8'));
  }
  scanSources(files, MIN_CALL_SITES, violations);
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

// В комментариях фикстуры нарочно процитированы запрещённые формы: гейт обязан
// их игнорировать.
const CLEAN_MENU = `<script lang="ts">
  import { portal } from '$lib/utils/portal';
  // раньше: portal: usePortal = false
  interface Props {
    /** раньше было portal?: boolean — проп удалён */
    label?: string;
    disabled?: boolean;
    children: Snippet;
  }
  const { label = 'Действия', disabled = false, children }: Props = $props();
  let open = $state(false);
</script>

<div class="action-menu" bind:this={rootEl}>
  <button type="button" aria-expanded={open} aria-label={label} {disabled}>
    {#if icon}
      {@render icon()}
    {/if}
  </button>
  {#if open}
    <div
      class="action-menu-panel"
      role="menu"
      tabindex="-1"
      use:portal
      use:actionMenuPortalPosition={triggerEl}
      onkeydown={onPanelKeydown}
      onclick={() => close(true)}
    >
      {@render children()}
    </div>
  {/if}
</div>

<!-- раньше: {#if usePortal} <div role="menu"> без use:portal -->

<style lang="scss">
  .action-menu {
    display: inline-flex;
  }

  // раньше: position: absolute
  .action-menu-panel {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
  }

  .action-menu-panel :global(button) {
    width: 100%;
  }
</style>
`;

const CALLER_1 = `<script lang="ts">
  import ActionMenu from '$lib/components/ActionMenu.svelte';
</script>

<!-- в комментарии: <ActionMenu portal> — не вызов -->
<ActionMenu variant="ghost-sm" label="Первое">
  <button role="menuitem">Один</button>
</ActionMenu>
<ActionMenu
  variant="ghost-sm"
  label={\`Действия: \${name}\`}
  onOpenChange={(o) => (open = o)}
>
  <button role="menuitem">Два</button>
</ActionMenu>
`;

const CALLER_2 = `<script lang="ts">
  import ActionMenu from '$lib/components/ActionMenu.svelte';
</script>

<div class="row-actions-host">
  <ActionMenu label="Второе">
    <button role="menuitem">Три</button>
  </ActionMenu>
</div>

<style lang="scss">
  .row-actions {
    flex: none;
    opacity: 0;

    .tree-row:hover &,
    .tree-row:focus-within & {
      opacity: 1;
    }

    &:has(:global([aria-expanded='true'])) {
      opacity: 1;
    }
  }
</style>
`;

const PATH_CALLER_1 = 'src/features/a/First.svelte';
const PATH_CALLER_2 = 'src/features/b/Second.svelte';

/** Мутация с проверкой уникальности якоря: иначе мутант мог бы задеть соседа. */
function mutate(src, from, to) {
  const count = src.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`якорь мутации встречается ${count} раз(а), ожидалось ровно 1: ${from.trim()}`);
  }
  return src.replace(from, to);
}

function fixtureFiles(over = {}) {
  return new Map([
    [MENU_FILE, over.menu ?? CLEAN_MENU],
    [PATH_CALLER_1, over.caller1 ?? CALLER_1],
    [PATH_CALLER_2, over.caller2 ?? CALLER_2],
  ]);
}

function runSelfTest() {
  const fixtures = [];
  try {
    fixtures.push({
      name: 'корректные ActionMenu, вызовы и .row-actions (позитив)',
      files: fixtureFiles(),
      codes: [],
    });
    fixtures.push({
      name: 'M1 вернуть ветку {#if usePortal} с непортальной панелью',
      files: fixtureFiles({
        menu: mutate(
          mutate(
            CLEAN_MENU,
            '{#if open}',
            '{#if open}\n    {#if usePortal}<div role="menu" tabindex="-1" use:portal use:actionMenuPortalPosition={triggerEl}>x</div>{:else}',
          ),
          '    </div>\n  {/if}\n</div>',
          '    </div>{/if}\n  {/if}\n</div>',
        ),
      }),
      codes: ['A', 'B'],
    });
    fixtures.push({
      name: 'M2 вернуть portal: usePortal = false (проп и деструктуризация)',
      files: fixtureFiles({
        menu: mutate(
          mutate(
            CLEAN_MENU,
            '    disabled?: boolean;\n',
            '    disabled?: boolean;\n    portal?: boolean;\n',
          ),
          "label = 'Действия',",
          "label = 'Действия', portal: usePortal = false,",
        ),
      }),
      codes: ['B'],
    });
    fixtures.push({
      name: 'M3 убрать use:portal из тега панели',
      files: fixtureFiles({ menu: mutate(CLEAN_MENU, '      use:portal\n', '') }),
      codes: ['A'],
    });
    fixtures.push({
      name: 'M4 вернуть position: absolute в .action-menu-panel',
      files: fixtureFiles({ menu: mutate(CLEAN_MENU, 'position: fixed;', 'position: absolute;') }),
      codes: ['C'],
    });
    fixtures.push({
      name: 'M5 добавить на вызов portal={false}',
      files: fixtureFiles({
        caller1: mutate(CALLER_1, 'label="Первое"', 'label="Первое" portal={false}'),
      }),
      codes: ['D'],
    });
    fixtures.push({
      name: 'M6 добавить на вызов голый portal',
      files: fixtureFiles({ caller2: mutate(CALLER_2, 'label="Второе"', 'portal label="Второе"') }),
      codes: ['D'],
    });
    fixtures.push({
      name: 'M7 убрать все вызовы <ActionMenu',
      files: fixtureFiles({
        caller1: CALLER_1.replace(/<ActionMenu[\s\S]*<\/ActionMenu>\n(?=<ActionMenu)/, '').replace(
          /<ActionMenu[\s\S]*<\/ActionMenu>\n$/,
          '',
        ),
        caller2: mutate(
          CALLER_2,
          '  <ActionMenu label="Второе">\n    <button role="menuitem">Три</button>\n  </ActionMenu>\n',
          '',
        ),
      }),
      codes: ['E'],
    });
    fixtures.push({
      name: "M8 убрать :has(:global([aria-expanded='true'])) из .row-actions",
      files: fixtureFiles({
        caller2: mutate(
          CALLER_2,
          "\n    &:has(:global([aria-expanded='true'])) {\n      opacity: 1;\n    }\n",
          '',
        ),
      }),
      codes: ['F'],
    });
    fixtures.push({
      name: 'M9 заменить рабочую форму на мёртвую :has([aria-expanded]) без :global',
      files: fixtureFiles({
        caller2: mutate(
          CALLER_2,
          "&:has(:global([aria-expanded='true']))",
          "&:has([aria-expanded='true'])",
        ),
      }),
      codes: ['F'],
      mention: /мёртвая/,
    });
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSources(f.files, SELFTEST_MIN_CALL_SITES, v);
    let ok;
    let expected;
    if (f.codes.length === 0) {
      ok = v.length === 0;
      expected = '0 нарушений';
    } else {
      const got = new Set(v.map((line) => /— \[([A-F])\]/.exec(line)?.[1]));
      ok = f.codes.every((c) => got.has(c)) && [...got].every((c) => f.codes.includes(c));
      if (ok && f.mention) ok = v.some((line) => f.mention.test(line));
      expected = `правила ${f.codes.join('+')}${f.mention ? `, текст ${f.mention}` : ''}`;
    }
    const gotCodes =
      [...new Set(v.map((line) => /— \[([A-F])\]/.exec(line)?.[1]))].join('+') || '—';
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (ожидалось: ${expected}; поймано: ${gotCodes})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} из ${fixtures.length} фикстур не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — все ${fixtures.length} фикстур совпали с ожиданием.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-action-menu-portal.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  checkRepo(root, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
