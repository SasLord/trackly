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
// Фаза 41.8 (план 28, D-36 — строка при открытом меню и подложка):
//   [G] в разметке ActionMenu ровно один элемент `action-menu-backdrop`: use:portal,
//       стоит раньше role="menu"; в style `position: fixed`, `inset: 0`, фон прозрачный;
//   [H] onDown исключает цель внутри подложки до `open = false` (иначе подложка исчезнет
//       между mousedown и click, и click уйдёт на строку под ней);
//   [I] script ставит и снимает `data-menu-open`; в openAt строка берётся по точке
//       (`elementFromPoint(`);
//   [J] выделение по `[data-menu-open]` в TableRow.svelte только в :global-форме (scoped
//       и `:has(` — мёртвое правило), есть правило удержания `.row-actions` видимым;
//   [K] у `<ActionMenu` нет атрибутов `backdrop`/`highlight` (флаг на вызове запрещён).
//
// Фаза 41.8 (план 31, D-35 — вложенный попап ActionSubmenu):
//   [L] в разметке ActionSubmenu ровно один элемент с `data-action-submenu`: use:portal и
//       role="menu" в его теге;
//   [M] обработчик клика триггера (onTriggerClick) содержит `stopPropagation` (иначе
//       onclick панели закроет всё меню); Escape-ветка обработчика попапа зовёт
//       `preventDefault`;
//   [N] документный onKey ActionMenu содержит `if (e.defaultPrevented ... data-action-submenu
//       ...) return` РАНЬШЕ ветки `close(true)`: слушатель висит на document, stopPropagation
//       подменю до него не защищает (наличие stopPropagation механизмом не считается);
//   [O] onDown ActionMenu ссылается на `data-action-submenu` до `open = false` (клик внутри
//       подменю не закрывает меню);
//   [P] у ActionSubmenu нет `position: absolute`, у `.action-submenu-panel` есть
//       `position: fixed` и z-index строго больше z-index `.action-menu-panel`;
//   [Q] ActionMenu отдаёт контекст (`setContext(` с ACTION_MENU_CONTEXT), ActionSubmenu
//       читает его (`getContext(`).
//
// Комментарии (HTML, JS, SCSS) перед проверкой затираются с сохранением длины
// и переводов строк, поэтому цитаты форм в комментариях гейт не обманывают, а
// номера строк в сообщениях честные.
//
// Self-test (`--selftest`): корректная фикстура в памяти (обязана пройти) и
// мутанты (до 41.8-31 их 19, добавлено 9),// мутантов, каждый получен заменой с ПРОВЕРКОЙ, что якорь встречается ровно
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
import { stripComments } from './_strip.mjs';
import { walk as walkRoots, UI_ROOT as ROOTS_UI } from './_roots.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-action-menu-portal]';

const MENU_FILE = 'src/lib/components/ActionMenu.svelte';
const TABLE_ROW_FILE = 'src/lib/components/TableRow.svelte';
const GLOBAL_SCSS_FILE = 'src/styles/global.scss';
const SUBMENU_FILE = 'src/lib/components/ActionSubmenu.svelte';
// Сколько вызовов <ActionMenu есть в src/**/*.svelte (инвентарь плана 41.1-16):
// 21 = сумма `grep -rc '<ActionMenu' src --include='*.svelte'` по файлам
// (в плане 41-27 было 13; 41.1-16/41.1-02: 15; 41.8-13 пересчитан от репозитория: 21 —
// строки состава ушли на DeviceRowMenu, добавились меню призрака и вложенной группы).
const MIN_CALL_SITES = 21;
const SELFTEST_MIN_CALL_SITES = 2;

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
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

  let menuZ = null;

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

    // [G] подложка
    const bdMatches = [...markup.matchAll(/action-menu-backdrop/g)];
    if (bdMatches.length !== 1) {
      add(
        MENU_FILE,
        0,
        'G',
        `в разметке ${bdMatches.length} вхождений action-menu-backdrop, ожидается ровно одно (подложка D-36).`,
      );
    } else {
      const bIdx = bdMatches[0].index;
      const bStart = markup.lastIndexOf('<', bIdx);
      const bEnd = tagEnd(markup, bStart);
      const bTag = bEnd === -1 ? '' : markup.slice(bStart, bEnd + 1);
      if (!/\buse:portal\b/.test(bTag)) {
        add(MENU_FILE, lineOf(markup, bIdx), 'G', 'у подложки нет `use:portal`.');
      }
      if (roleMatches.length === 1 && bIdx > roleMatches[0].index) {
        add(MENU_FILE, lineOf(markup, bIdx), 'G', 'подложка стоит позже панели role="menu".');
      }
    }
    const bdRuleM = /\.action-menu-backdrop\s*\{/.exec(style);
    const bdRule = bdRuleM
      ? extractBalanced(style, bdRuleM.index + bdRuleM[0].length - 1, '{', '}')
      : null;
    if (!bdRule) {
      add(MENU_FILE, 0, 'G', 'не найдено правило `.action-menu-backdrop` в style.');
    } else {
      if (!/position\s*:\s*fixed/.test(bdRule.body)) {
        add(MENU_FILE, 0, 'G', 'в `.action-menu-backdrop` нет `position: fixed`.');
      }
      if (!/inset\s*:\s*0\b/.test(bdRule.body)) {
        add(
          MENU_FILE,
          0,
          'G',
          'в `.action-menu-backdrop` нет `inset: 0` — подложка не покрывает экран.',
        );
      }
      const bg = /background(?:-color)?\s*:\s*([^;]+);/.exec(bdRule.body);
      if (bg && !/^\s*transparent\s*$/.test(bg[1])) {
        add(MENU_FILE, 0, 'G', `у подложки непрозрачный фон (${bg[1].trim()}), нужен transparent.`);
      }
    }

    // [H] onDown исключает подложку до закрытия
    const odM = /function\s+onDown\s*\(/.exec(script);
    const odBraceIdx = odM ? script.indexOf('{', odM.index) : -1;
    const od = odBraceIdx === -1 ? null : extractBalanced(script, odBraceIdx, '{', '}');
    if (!od) {
      add(
        MENU_FILE,
        0,
        'H',
        'не найден обработчик onDown — гейт не может проверить исключение подложки.',
      );
    } else {
      const bdRef = od.body.search(/backdrop/i);
      const closeIdx = od.body.search(/\bopen\s*=\s*false\b/);
      if (bdRef === -1 || (closeIdx !== -1 && bdRef > closeIdx)) {
        add(
          MENU_FILE,
          lineOf(script, od.start),
          'H',
          'onDown не исключает цель внутри подложки до `open = false`: mousedown закроет меню, click уйдёт на строку под подложкой.',
        );
      }
    }

    // [I] пометка строки
    if (!/setAttribute\(\s*'data-menu-open'/.test(script)) {
      add(
        MENU_FILE,
        0,
        'I',
        "в script нет `setAttribute('data-menu-open'` — строка-владелец не помечается.",
      );
    }
    if (!/removeAttribute\(\s*'data-menu-open'/.test(script)) {
      add(
        MENU_FILE,
        0,
        'I',
        "в script нет `removeAttribute('data-menu-open'` — пометка не снимается при закрытии.",
      );
    }
    const oaM = /function\s+openAt\s*\(/.exec(script);
    const oaBraceIdx = oaM ? script.indexOf('{', oaM.index) : -1;
    const oa = oaBraceIdx === -1 ? null : extractBalanced(script, oaBraceIdx, '{', '}');
    if (!oa || !/elementFromPoint\(/.test(oa.body)) {
      add(
        MENU_FILE,
        0,
        'I',
        'в openAt нет `elementFromPoint(` — строка правого клика не определяется по точке.',
      );
    }

    // [O] клик внутри подменю не закрывает меню
    if (od) {
      const smRef = od.body.indexOf('data-action-submenu');
      const closeIdx = od.body.search(/\bopen\s*=\s*false\b/);
      if (smRef === -1 || (closeIdx !== -1 && smRef > closeIdx)) {
        add(
          MENU_FILE,
          lineOf(script, od.start),
          'O',
          'onDown не исключает цель внутри `[data-action-submenu]` до `open = false`: клик в подменю закроет меню раньше обработчика варианта.',
        );
      }
    }

    // [N] защита Esc подменю живёт в документном onKey
    const okM = /function\s+onKey\s*\(/.exec(script);
    const okBraceIdx = okM ? script.indexOf('{', okM.index) : -1;
    const ok = okBraceIdx === -1 ? null : extractBalanced(script, okBraceIdx, '{', '}');
    if (!ok) {
      add(
        MENU_FILE,
        0,
        'N',
        'не найден документный onKey — гейт не может проверить защиту Esc подменю.',
      );
    } else {
      const guard =
        /\bif\s*\(\s*e\.defaultPrevented[\s\S]*?data-action-submenu[\s\S]*?\)\s*return\b/.exec(
          ok.body,
        );
      const closeIdx = ok.body.search(/\bclose\(\s*true\s*\)/);
      if (!guard || (closeIdx !== -1 && guard.index > closeIdx)) {
        add(
          MENU_FILE,
          lineOf(script, ok.start),
          'N',
          'onKey не возвращается раньше ветки `close(true)` при `e.defaultPrevented`/цели внутри `[data-action-submenu]`: Esc в подменю закроет всё меню (stopPropagation на document не защищает).',
        );
      }
    }

    // [Q] контекст (сторона ActionMenu)
    if (!/\bsetContext\s*(<[^>]*>)?\s*\(\s*ACTION_MENU_CONTEXT\b/.test(script)) {
      add(
        MENU_FILE,
        0,
        'Q',
        'ActionMenu не отдаёт контекст: нет `setContext(ACTION_MENU_CONTEXT`.',
      );
    }

    const zM = /\.action-menu-panel\s*\{[^}]*?z-index\s*:\s*(\d+)/.exec(style);
    menuZ = zM ? Number(zM[1]) : null;
  }

  // --- ActionSubmenu: правила L, M, P, Q ---------------------------------------
  const smSrc = files.get(SUBMENU_FILE);
  if (smSrc === undefined) {
    add(SUBMENU_FILE, 0, 'L', 'файл не найден — гейт не может проверить подменю.');
  } else {
    const sm = splitSvelte(smSrc);
    const dm = [...sm.markup.matchAll(/data-action-submenu/g)];
    if (dm.length !== 1) {
      add(
        SUBMENU_FILE,
        0,
        'L',
        `в разметке ${dm.length} вхождений data-action-submenu, ожидается ровно одно (портальный попап).`,
      );
    } else {
      const st = sm.markup.lastIndexOf('<', dm[0].index);
      const en = tagEnd(sm.markup, st);
      const tag = en === -1 ? '' : sm.markup.slice(st, en + 1);
      if (!/\buse:portal\b/.test(tag)) {
        add(
          SUBMENU_FILE,
          lineOf(sm.markup, dm[0].index),
          'L',
          'у попапа подменю нет `use:portal`.',
        );
      }
      if (!/role\s*=\s*"menu"/.test(tag)) {
        add(SUBMENU_FILE, lineOf(sm.markup, dm[0].index), 'L', 'у попапа подменю нет role="menu".');
      }
    }

    const tcM = /function\s+onTriggerClick\s*\(/.exec(sm.script);
    const tcBrace = tcM ? sm.script.indexOf('{', tcM.index) : -1;
    const tc = tcBrace === -1 ? null : extractBalanced(sm.script, tcBrace, '{', '}');
    if (!tc || !/\bstopPropagation\(\)/.test(tc.body)) {
      add(
        SUBMENU_FILE,
        tc ? lineOf(sm.script, tc.start) : 0,
        'M',
        'обработчик клика триггера (onTriggerClick) не зовёт `stopPropagation()`: onclick панели закроет всё меню.',
      );
    }
    const pkM = /function\s+onPopupKeydown\s*\(/.exec(sm.script);
    const pkBrace = pkM ? sm.script.indexOf('{', pkM.index) : -1;
    const pk = pkBrace === -1 ? null : extractBalanced(sm.script, pkBrace, '{', '}');
    const escIdx = pk ? pk.body.indexOf("'Escape'") : -1;
    if (!pk || escIdx === -1 || !/preventDefault\(\)/.test(pk.body.slice(escIdx))) {
      add(
        SUBMENU_FILE,
        0,
        'M',
        'в обработчике клавиш попапа ветка Escape не зовёт `preventDefault()` (сигнал `defaultPrevented` для ActionMenu).',
      );
    }

    const abs2 = /position\s*:\s*absolute/.exec(sm.style);
    if (abs2) {
      add(SUBMENU_FILE, lineOf(sm.style, abs2.index), 'P', '`position: absolute` в style подменю.');
    }
    const prM = /\.action-submenu-panel\s*\{/.exec(sm.style);
    const pr = prM ? extractBalanced(sm.style, prM.index + prM[0].length - 1, '{', '}') : null;
    if (!pr || !/position\s*:\s*fixed/.test(pr.body)) {
      add(SUBMENU_FILE, 0, 'P', 'в `.action-submenu-panel` нет `position: fixed`.');
    }
    const pz = pr ? /z-index\s*:\s*(\d+)/.exec(pr.body) : null;
    if (!pz || menuZ === null || Number(pz[1]) <= menuZ) {
      add(
        SUBMENU_FILE,
        0,
        'P',
        `z-index попапа (${pz ? pz[1] : 'нет'}) не больше z-index панели ActionMenu (${menuZ ?? 'нет'}): подменю окажется под панелью.`,
      );
    }

    if (!/\bgetContext\s*(<[^>]*>)?\s*\(\s*ACTION_MENU_CONTEXT\b/.test(sm.script)) {
      add(
        SUBMENU_FILE,
        0,
        'Q',
        'ActionSubmenu не читает контекст: нет `getContext(ACTION_MENU_CONTEXT`.',
      );
    }
  }

  // --- [J] выделение строки: TableRow.svelte + global.scss --------------------
  const trSrc = files.get(TABLE_ROW_FILE);
  const globalSrc = files.get(GLOBAL_SCSS_FILE);
  if (trSrc === undefined) {
    add(TABLE_ROW_FILE, 0, 'J', 'файл не найден — гейт не может проверить выделение строки.');
  } else {
    const trStyle = splitSvelte(trSrc).style;
    const hits = [...trStyle.matchAll(/data-menu-open/g)];
    if (hits.length === 0) {
      add(
        TABLE_ROW_FILE,
        0,
        'J',
        'в style TableRow нет правила по `[data-menu-open]` — строка-владелец не выделяется.',
      );
    }
    for (const h of hits) {
      const boundary = Math.max(
        trStyle.lastIndexOf('{', h.index),
        trStyle.lastIndexOf('}', h.index),
        trStyle.lastIndexOf(';', h.index),
      );
      const before = trStyle.slice(boundary + 1, h.index);
      if (!/:global\([^)]*$/.test(before) && !/:global\([^)]*\([^)]*$/.test(before)) {
        add(
          TABLE_ROW_FILE,
          lineOf(trStyle, h.index),
          'J',
          '`[data-menu-open]` вне `:global(...)`: атрибут ставит ActionMenu снаружи, Svelte вырежет scoped-селектор (мёртвое правило).',
        );
      }
    }
    if (/:has\([^)]*data-menu-open/.test(trStyle)) {
      add(TABLE_ROW_FILE, 0, 'J', '`:has(` с `data-menu-open` — Svelte вырежет такой селектор.');
    }
  }
  const keepSrc =
    (trSrc === undefined ? '' : splitSvelte(trSrc).style) +
    '\n' +
    (globalSrc === undefined ? '' : stripComments(globalSrc));
  let keeps = false;
  for (const m of keepSrc.matchAll(/\[data-menu-open\][^{};]*\.row-actions\s*\{/g)) {
    const r = extractBalanced(keepSrc, m.index + m[0].length - 1, '{', '}');
    if (r && /opacity\s*:\s*1\b/.test(r.body)) keeps = true;
  }
  if (!keeps) {
    add(
      GLOBAL_SCSS_FILE,
      0,
      'J',
      'нет правила `[data-menu-open] .row-actions { opacity: 1 }` — кебаб строки исчезнет при правом клике (подложка снимает :hover).',
    );
  }

  // --- вызовы: D, E; видимость триггера: F -----------------------------------
  let callSites = 0;
  for (const [file, src] of files) {
    if (!file.endsWith('.svelte')) continue;
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
        if (/(^|\s)(backdrop|highlight)(?=\s|=|\/|>|$)/.test(attributeSkeleton(tag))) {
          add(
            file,
            lineOf(markup, m.index),
            'K',
            'у вызова <ActionMenu атрибут `backdrop`/`highlight` — подложка и выделение общие, флаг на вызове запрещён (D-36).',
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

function checkRepo(root, violations) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) {
    violations.push(`src: каталог не найден (${srcDir}).`);
    return;
  }
  const files = new Map();
  for (const abs of walkRoots({
    exts: ['.svelte'],
    roots: path.resolve(root) === ROOTS_UI ? null : [srcDir],
  })) {
    files.set(path.relative(root, abs).split(path.sep).join('/'), fs.readFileSync(abs, 'utf8'));
  }
  const globalPath = path.join(root, GLOBAL_SCSS_FILE);
  if (fs.existsSync(globalPath)) files.set(GLOBAL_SCSS_FILE, fs.readFileSync(globalPath, 'utf8'));
  scanSources(files, MIN_CALL_SITES, violations);
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

// В комментариях фикстуры нарочно процитированы запрещённые формы: гейт обязан
// их игнорировать.
const CLEAN_MENU = `<script lang="ts">
  import { portal } from '$lib/utils/portal';
  import { setContext } from 'svelte';
  import { ACTION_MENU_CONTEXT } from './actionMenuContext';
  // раньше: portal: usePortal = false
  interface Props {
    /** раньше было portal?: boolean — проп удалён */
    label?: string;
    disabled?: boolean;
    children: Snippet;
  }
  const { label = 'Действия', disabled = false, children }: Props = $props();
  let open = $state(false);
  let backdropEl = $state(null);
  let ownerRow = null;
  function openAt(x, y) {
    ownerRow = document.elementFromPoint(x, y)?.closest('tr') ?? null;
    open = true;
  }
  $effect(() => {
    if (!open) return;
    const row = ownerRow;
    row.setAttribute('data-menu-open', 'true');
    return () => {
      row.removeAttribute('data-menu-open');
    };
  });
  $effect(() => {
    function onDown(e) {
      const insideBackdrop = backdropEl?.contains(e.target) ?? false;
      const insideSubmenu = !!e.target?.closest?.('[data-action-submenu]');
      if (open && !insideBackdrop && !insideSubmenu) open = false;
    }
    function onKey(e) {
      if (e.defaultPrevented || e.target?.closest?.('[data-action-submenu]')) return;
      if (e.key !== 'Escape' || !open) return;
      close(true);
    }
  });
  setContext(ACTION_MENU_CONTEXT, { close });
</script>

<div class="action-menu" bind:this={rootEl}>
  <button type="button" aria-expanded={open} aria-label={label} {disabled}>
    {#if icon}
      {@render icon()}
    {/if}
  </button>
  {#if open}
    <div
      class="action-menu-backdrop"
      role="presentation"
      bind:this={backdropEl}
      use:portal
      onclick={() => close(false)}
    ></div>
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

  .action-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
    background: transparent;
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

const CLEAN_SUBMENU = `<script lang="ts">
  import { getContext } from 'svelte';
  import { portal } from '$lib/utils/portal';
  import { ACTION_MENU_CONTEXT } from './actionMenuContext';
  const menu = getContext(ACTION_MENU_CONTEXT);
  function onTriggerClick(e) {
    e.stopPropagation();
  }
  function onPopupKeydown(e) {
    if (e.key === 'ArrowLeft' || e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
    }
  }
</script>

<button type="button" role="menuitem" aria-haspopup="menu" onclick={onTriggerClick}>x</button>
{#if open}
  <div
    class="action-submenu-panel"
    role="menu"
    data-action-submenu
    use:portal
    onkeydown={onPopupKeydown}
  >
    {@render children()}
  </div>
{/if}

<style lang="scss">
  // раньше: position: absolute
  .action-submenu-panel {
    position: fixed;
    z-index: 1001;
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

const CLEAN_TABLE_ROW = `<script lang="ts">
  const { children } = $props();
</script>

<tr class="tr-row">{@render children?.()}</tr>

<style lang="scss">
  .tr-row {
    &:hover {
      background: var(--tr-row-hover);
    }
  }

  // раньше: .tr-row[data-menu-open] без :global (мёртвое правило)
  :global(.tr-row[data-menu-open]:not(.selected)) {
    background: var(--tr-row-hover);
  }
</style>
`;

const CLEAN_GLOBAL = `.tr-row[data-menu-open] .row-actions {
  opacity: 1;
  visibility: visible;
}
`;

const PATH_SUBMENU = SUBMENU_FILE;
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
    [TABLE_ROW_FILE, over.tableRow ?? CLEAN_TABLE_ROW],
    [GLOBAL_SCSS_FILE, over.global ?? CLEAN_GLOBAL],
    [PATH_SUBMENU, over.submenu ?? CLEAN_SUBMENU],
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
      files: fixtureFiles({
        menu: mutate(
          CLEAN_MENU,
          '      use:portal\n      use:actionMenuPortalPosition',
          '      use:actionMenuPortalPosition',
        ),
      }),
      codes: ['A'],
    });
    fixtures.push({
      name: 'M4 вернуть position: absolute в .action-menu-panel',
      files: fixtureFiles({
        menu: mutate(
          CLEAN_MENU,
          'position: fixed;\n    z-index: 1000;',
          'position: absolute;\n    z-index: 1000;',
        ),
      }),
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
    fixtures.push({
      // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок.
      // Блочно-первая вычистка стирала бы `position: absolute` между
      // `templates/*.html` и ближайшим `*/` ниже — правило C молчало бы.
      name: 'M10 `/*` в строчном комментарии не прячет position: absolute',
      files: fixtureFiles({
        menu: mutate(
          CLEAN_MENU,
          'position: fixed;\n    z-index: 1000;',
          'position: fixed;\n    z-index: 1000;\n    // шаблоны лежат в templates/*.html\n    position: absolute;\n    /* настоящий блочный комментарий */',
        ),
      }),
      codes: ['C'],
    });
    fixtures.push({
      name: 'M11 [data-menu-open] переписано в scoped-форму без :global (мёртвое правило)',
      files: fixtureFiles({
        tableRow: mutate(
          CLEAN_TABLE_ROW,
          ':global(.tr-row[data-menu-open]:not(.selected))',
          '.tr-row[data-menu-open]:not(.selected)',
        ),
      }),
      codes: ['J'],
      mention: /вне `:global/,
    });
    fixtures.push({
      name: 'M12 удалено правило .row-actions для [data-menu-open]',
      files: fixtureFiles({
        global: mutate(
          CLEAN_GLOBAL,
          '.tr-row[data-menu-open] .row-actions {',
          '.tr-row[data-menu-open] .something-else {',
        ),
      }),
      codes: ['J'],
      mention: /row-actions/,
    });
    fixtures.push({
      name: 'M13 подложка без use:portal',
      files: fixtureFiles({
        menu: mutate(
          CLEAN_MENU,
          '      bind:this={backdropEl}\n      use:portal\n',
          '      bind:this={backdropEl}\n',
        ),
      }),
      codes: ['G'],
      mention: /подложки нет `use:portal`/,
    });
    fixtures.push({
      name: 'M14 подложка без inset: 0',
      files: fixtureFiles({
        menu: mutate(CLEAN_MENU, '    inset: 0;\n    z-index: 999;', '    z-index: 999;'),
      }),
      codes: ['G'],
      mention: /inset: 0/,
    });
    fixtures.push({
      name: 'M15 onDown без исключения подложки',
      files: fixtureFiles({
        menu: mutate(
          mutate(
            CLEAN_MENU,
            '      const insideBackdrop = backdropEl?.contains(e.target) ?? false;\n',
            '',
          ),
          'if (open && !insideBackdrop && !insideSubmenu) open = false;',
          'if (open && !insideSubmenu) open = false;',
        ),
      }),
      codes: ['H'],
    });
    fixtures.push({
      name: 'M16 снятие data-menu-open удалено',
      files: fixtureFiles({
        menu: mutate(CLEAN_MENU, "      row.removeAttribute('data-menu-open');\n", ''),
      }),
      codes: ['I'],
      mention: /removeAttribute/,
    });
    fixtures.push({
      name: 'M17 TableRow без правила [data-menu-open]',
      files: fixtureFiles({
        tableRow: mutate(
          CLEAN_TABLE_ROW,
          ':global(.tr-row[data-menu-open]:not(.selected)) {\n    background: var(--tr-row-hover);\n  }',
          '',
        ),
      }),
      codes: ['J'],
      mention: /нет правила по/,
    });
    fixtures.push({
      name: 'M18 на вызове <ActionMenu появился атрибут backdrop',
      files: fixtureFiles({
        caller1: mutate(CALLER_1, 'label="Первое"', 'label="Первое" backdrop'),
      }),
      codes: ['K'],
    });
    fixtures.push({
      name: 'M19 в openAt убран elementFromPoint',
      files: fixtureFiles({
        menu: mutate(CLEAN_MENU, "document.elementFromPoint(x, y)?.closest('tr') ?? null", 'null'),
      }),
      codes: ['I'],
      mention: /elementFromPoint/,
    });
    fixtures.push({
      name: 'M20 попап подменю без use:portal',
      files: fixtureFiles({
        submenu: mutate(
          CLEAN_SUBMENU,
          '    data-action-submenu\n    use:portal\n',
          '    data-action-submenu\n',
        ),
      }),
      codes: ['L'],
      mention: /нет `use:portal`/,
    });
    fixtures.push({
      name: 'M21 триггер подменю без stopPropagation',
      files: fixtureFiles({
        submenu: mutate(
          CLEAN_SUBMENU,
          '  function onTriggerClick(e) {\n    e.stopPropagation();\n  }',
          '  function onTriggerClick(e) {\n    void e;\n  }',
        ),
      }),
      codes: ['M'],
      mention: /onTriggerClick/,
    });
    fixtures.push({
      name: 'M22 onKey без защиты Esc подменю (оператор с defaultPrevented удалён)',
      files: fixtureFiles({
        menu: mutate(
          CLEAN_MENU,
          "      if (e.defaultPrevented || e.target?.closest?.('[data-action-submenu]')) return;\n",
          '',
        ),
      }),
      codes: ['N'],
    });
    fixtures.push({
      name: 'M23 защита Esc в onKey перенесена после ветки close',
      files: fixtureFiles({
        menu: mutate(
          mutate(
            CLEAN_MENU,
            "      if (e.defaultPrevented || e.target?.closest?.('[data-action-submenu]')) return;\n",
            '',
          ),
          '      close(true);\n    }\n  });\n  setContext',
          "      close(true);\n      if (e.defaultPrevented || e.target?.closest?.('[data-action-submenu]')) return;\n    }\n  });\n  setContext",
        ),
      }),
      codes: ['N'],
    });
    fixtures.push({
      name: 'M24 onDown без исключения подменю',
      files: fixtureFiles({
        menu: mutate(
          mutate(
            CLEAN_MENU,
            "      const insideSubmenu = !!e.target?.closest?.('[data-action-submenu]');\n",
            '',
          ),
          'if (open && !insideBackdrop && !insideSubmenu) open = false;',
          'if (open && !insideBackdrop) open = false;',
        ),
      }),
      codes: ['O'],
    });
    fixtures.push({
      name: 'M25 z-index попапа ниже панели',
      files: fixtureFiles({
        submenu: mutate(CLEAN_SUBMENU, 'z-index: 1001;', 'z-index: 900;'),
      }),
      codes: ['P'],
      mention: /z-index попапа/,
    });
    fixtures.push({
      name: 'M26 ActionSubmenu без getContext',
      files: fixtureFiles({
        submenu: mutate(
          CLEAN_SUBMENU,
          '  const menu = getContext(ACTION_MENU_CONTEXT);\n',
          '  const menu = null;\n',
        ),
      }),
      codes: ['Q'],
    });
    fixtures.push({
      name: 'M27 ActionMenu без setContext',
      files: fixtureFiles({
        menu: mutate(CLEAN_MENU, '  setContext(ACTION_MENU_CONTEXT, { close });', ''),
      }),
      codes: ['Q'],
    });
    fixtures.push({
      name: 'M28 Escape попапа без preventDefault',
      files: fixtureFiles({
        submenu: mutate(
          CLEAN_SUBMENU,
          "    if (e.key === 'ArrowLeft' || e.key === 'Escape') {\n      e.preventDefault();\n",
          "    if (e.key === 'ArrowLeft' || e.key === 'Escape') {\n",
        ),
      }),
      codes: ['M'],
      mention: /preventDefault/,
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
      const got = new Set(v.map((line) => /— \[([A-Q])\]/.exec(line)?.[1]));
      ok = f.codes.every((c) => got.has(c)) && [...got].every((c) => f.codes.includes(c));
      if (ok && f.mention) ok = v.some((line) => f.mention.test(line));
      expected = `правила ${f.codes.join('+')}${f.mention ? `, текст ${f.mention}` : ''}`;
    }
    const gotCodes =
      [...new Set(v.map((line) => /— \[([A-Q])\]/.exec(line)?.[1]))].join('+') || '—';
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
