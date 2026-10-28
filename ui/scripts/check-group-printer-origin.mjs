#!/usr/bin/env node
// [check-group-printer-origin] Структурный гейт происхождения строк «Подключённых
// принтеров» в карточке группы (Фаза 41.13, PGR-01, PGR-02).
//
// Почему он существует: UI фильтровал принтеры по литералу 'usb' в четырёх
// местах (GroupPrintersList: отбор строк сервера и запасной поиск;
// GroupPropertiesForm: блок только для чтения и фильтр вторичных свойств).
// Сервер получил третье происхождение `member`; любой оставшийся литерал молча
// прячет принтеры-члены там, где они нужны, и пропускает их туда, где не нужны
// (во вторичные свойства «Ссылки на устройства»). svelte-check, eslint и build
// этого не видят: сравнение со строкой типизируется одинаково для любого значения.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он ловит возврат литералов и порядок веток выбора
// пометки, но НЕ доказывает внешний вид бейджей в обеих темах и в LAN-браузере
// (это проверка глазами) и не доказывает серверный приоритет USB > ссылка > в
// составе (его закрепляют серверные тесты плана 41.13-01).
//
// Правила (над текстом без комментариев):
//   (A) GroupPrintersList: тексты «USB», «Ссылка», «В составе» стоят каждый ровно
//       в одном <Badge>…</Badge>; «USB» с variant="accent", остальные два без него;
//   (B) GroupPrintersList, <script>: нет `=== 'usb'` / `!== 'usb'` (литерал 'usb'
//       допустим только в объявлении типа RowOrigin и в присваивании
//       `const origin: RowOrigin = …`); выражение origin строго
//       `<строка сервера>.origin === 'usb' ? 'usb' : hasLink ? 'link' : 'member'`
//       (приоритет USB > ссылка > в составе, порядок веток закреплён); первый цикл
//       пропускает `origin === 'link'`, запасной поиск ссылок ищет `origin === 'link'`;
//   (C) GroupPropertiesForm: нет usbPrinters, нет `=== 'usb'` / `!== 'usb'`, есть
//       фильтр вторичных свойств `origin === 'link'`, условие блока только для
//       чтения `{#if !hasLinkProperty && …}` не опирается на usb и считает
//       card.printers.length;
//   (D) GroupPrintersList: пустое состояние передаётся как {emptyBody}; в нём два
//       литерала, первый (при `canEdit && canLink`) содержит «ссылкой ниже», второй
//       нет; оба содержат «входит в состав»; фразы «появятся здесь сами» нет нигде;
//   (E) GroupPrintersList: единственный <ActionMenu> стоит под `{#if canEdit && r.link}`
//       (меню «Убрать ссылку» — только у строк с явной ссылкой).
// Файл или блок не разобрать — нарушение, не молчаливый PASS; отсутствие файла —
// красный, не SKIP.
//
// Self-test (`--selftest`): чистый РЕАЛЬНЫЙ исходник обязан пройти; затем мутанты
// по реальному исходнику через mutate() с проверкой ровно одного вхождения якоря
// (иначе мутант мог бы снять гейт у соседнего фрагмента). Каждый мутант обязан
// провалить именно своё правило. Есть мутанты-перестановки, зелёный контроль
// «запретный токен в комментарии» и ловушка `/*` внутри `//`.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-group-printer-origin.mjs              # проверить репозиторий
//   node scripts/check-group-printer-origin.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-group-printer-origin.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-group-printer-origin]';

const LIST_FILE = 'src/features/groups/GroupPrintersList.svelte';
const FORM_FILE = 'src/features/groups/GroupPropertiesForm.svelte';

const SCRIPT_RE = /<script\b[^>]*>([\s\S]*?)<\/script>/;
const USB_COMPARE_RE = /[!=]==?\s*'usb'/;

/** Нормализация пробелов: prettier может переносить выражения по строкам. */
function norm(s) {
  return s.replace(/\s+/g, ' ').trim();
}

/** Разбор файла: текст без комментариев, <script> и разметка без <script>. */
function parts(raw) {
  const code = stripComments(raw);
  const m = SCRIPT_RE.exec(code);
  if (!m) return { code, script: null, markup: code };
  const blanked = code.slice(0, m.index) + code.slice(m.index + m[0].length);
  return { code, script: m[1], markup: blanked };
}

/** Статический объект выражения вида `const emptyBody = $derived( … );` или null. */
function derivedExpr(script, name) {
  const m = new RegExp(`\\bconst\\s+${name}\\s*=\\s*\\$derived\\(`).exec(script);
  if (!m) return null;
  const open = script.indexOf('(', m.index + m[0].length - 1);
  let depth = 0;
  for (let i = open; i < script.length; i++) {
    const c = script[i];
    if (c === '(') depth++;
    else if (c === ')') {
      depth--;
      if (depth === 0) return script.slice(open + 1, i);
    }
  }
  return null;
}

function scanList(raw, violations) {
  const { code, script, markup } = parts(raw);

  // (A) Badge с текстом происхождения.
  const badges = [...markup.matchAll(/<Badge\b([^>]*)>([\s\S]*?)<\/Badge>/g)].map((m) => ({
    attrs: m[1],
    text: m[2].trim(),
  }));
  for (const [text, wantAccent] of [
    ['USB', true],
    ['Ссылка', false],
    ['В составе', false],
  ]) {
    const found = badges.filter((b) => b.text === text);
    if (found.length !== 1) {
      violations.push(
        `(A) ${LIST_FILE}: текст «${text}» стоит в ${found.length} <Badge>, нужен ровно один: пометка — текст внутри Badge, не иконка и не цвет.`,
      );
      continue;
    }
    const accent = /\bvariant\s*=\s*["']accent["']/.test(found[0].attrs);
    if (accent !== wantAccent) {
      violations.push(
        `(A) ${LIST_FILE}: Badge «${text}» ${wantAccent ? 'обязан быть' : 'не должен быть'} variant="accent" (акцент только у «USB»).`,
      );
    }
  }

  // (E) меню — только под {#if canEdit && r.link}.
  const menus = [...markup.matchAll(/<ActionMenu\b/g)].length;
  if (menus !== 1) {
    violations.push(`(E) ${LIST_FILE}: <ActionMenu> встречается ${menus} раз, нужен ровно один.`);
  }
  if (!/\{#if\s+canEdit\s*&&\s*r\.link\s*\}\s*<ActionMenu\b/.test(markup)) {
    violations.push(
      `(E) ${LIST_FILE}: <ActionMenu> не стоит непосредственно под \`{#if canEdit && r.link}\`: меню «Убрать ссылку» только у строк с явной ссылкой.`,
    );
  }

  // (D) фразы пустого состояния по всему файлу без комментариев.
  if (code.includes('появятся здесь сами')) {
    violations.push(
      `(D) ${LIST_FILE}: вернулась фраза «появятся здесь сами»: пустое состояние описывает и состав, и USB, и ссылку.`,
    );
  }
  if (!/\{emptyBody\}/.test(markup)) {
    violations.push(`(D) ${LIST_FILE}: <Table> не получает \`{emptyBody}\`.`);
  }

  if (script === null) {
    violations.push(`(B) ${LIST_FILE}: нет блока <script> — гейт не может проверить origin.`);
    return;
  }

  // (B) нет сравнений с 'usb' вне разрешённых мест.
  const originStmt = /\bconst\s+origin\s*:\s*RowOrigin\s*=\s*([^;]+);/.exec(script);
  const typeStmt = /\btype\s+RowOrigin\s*=\s*[^;]+;/.exec(script);
  if (!originStmt) {
    violations.push(
      `(B) ${LIST_FILE}: нет \`const origin: RowOrigin = …;\` — выбор пометки не разобрать.`,
    );
  }
  if (!typeStmt) {
    violations.push(`(B) ${LIST_FILE}: нет объявления \`type RowOrigin = …;\`.`);
  }
  let rest = script;
  if (originStmt) rest = rest.replace(originStmt[0], '');
  if (typeStmt) rest = rest.replace(typeStmt[0], '');
  if (USB_COMPARE_RE.test(rest)) {
    violations.push(
      `(B) ${LIST_FILE}: сравнение с литералом 'usb' вне присваивания origin. Строки сервера отбираются по origin === 'link' (пропуск), а не по usb: иначе принтеры-члены пропадают.`,
    );
  }
  if (!/'member'/.test(script)) {
    violations.push(
      `(B) ${LIST_FILE}: нет литерала 'member' — третье происхождение не обработано.`,
    );
  }
  if (originStmt) {
    const expr = norm(originStmt[1]);
    if (!/^\w+\.origin === 'usb' \? 'usb' : hasLink \? 'link' : 'member'$/.test(expr)) {
      violations.push(
        `(B) ${LIST_FILE}: выражение origin «${expr}» не совпадает с приоритетом USB > ссылка > в составе (ожидается: p.origin === 'usb' ? 'usb' : hasLink ? 'link' : 'member').`,
      );
    }
  }
  if (!/if\s*\(\s*\w+\.origin === 'link'\s*\|\|\s*seen\.has\(/.test(norm(script))) {
    violations.push(
      `(B) ${LIST_FILE}: первый цикл rows не пропускает \`origin === 'link'\`: строки-ссылки других свойств попадут в список, а члены с USB потеряются.`,
    );
  }
  if (!/printers\.find\(\s*\(p\)\s*=>[^)]*\.origin === 'link'\s*\)/.test(norm(script))) {
    violations.push(
      `(B) ${LIST_FILE}: запасной поиск сведений о ссылке в printers не ищет \`origin === 'link'\`.`,
    );
  }

  // (D) пустое состояние: два литерала с нужным содержимым.
  const body = derivedExpr(script, 'emptyBody');
  if (body === null) {
    violations.push(`(D) ${LIST_FILE}: нет \`const emptyBody = $derived(…)\`.`);
  } else {
    const lits = [...body.matchAll(/'([^']*)'/g)].map((m) => m[1]);
    if (!/canEdit\s*&&\s*canLink\s*\?/.test(body)) {
      violations.push(
        `(D) ${LIST_FILE}: emptyBody не ветвится по \`canEdit && canLink\`: «ссылкой ниже» нельзя показывать без поля добавления.`,
      );
    }
    if (lits.length !== 2) {
      violations.push(`(D) ${LIST_FILE}: в emptyBody ${lits.length} литералов, нужно два.`);
    } else {
      if (!lits[0].includes('ссылкой ниже')) {
        violations.push(`(D) ${LIST_FILE}: вариант с полем добавления не содержит «ссылкой ниже».`);
      }
      if (lits[1].includes('ссылкой ниже')) {
        violations.push(
          `(D) ${LIST_FILE}: вариант без поля добавления ссылается на «ссылкой ниже» — текст про то, чего на экране нет.`,
        );
      }
      for (const l of lits) {
        if (!l.includes('входит в состав')) {
          violations.push(`(D) ${LIST_FILE}: вариант пустого состояния без «входит в состав».`);
        }
      }
    }
  }
}

function scanForm(raw, violations) {
  const { code } = parts(raw);
  if (/\busbPrinters\b/.test(code)) {
    violations.push(
      `(C) ${FORM_FILE}: вернулась константа usbPrinters: блок только для чтения и пустая форма обязаны считать все принтеры карточки.`,
    );
  }
  if (USB_COMPARE_RE.test(code)) {
    violations.push(
      `(C) ${FORM_FILE}: сравнение с литералом 'usb'. Принтеры-члены (origin = member) не должны зависеть от него.`,
    );
  }
  if (!/\.filter\(\s*\(\w+\)\s*=>\s*\w+\.origin === 'link'\s*\)/.test(norm(code))) {
    violations.push(
      `(C) ${FORM_FILE}: нет фильтра вторичных свойств \`.filter((x) => x.origin === 'link')\`: члены и USB протекут в чужие списки ссылок.`,
    );
  }
  const cond = /\{#if\s+!hasLinkProperty\s*&&([^}]*)\}/.exec(code);
  if (!cond) {
    violations.push(
      `(C) ${FORM_FILE}: нет условия блока только для чтения \`{#if !hasLinkProperty && …}\`.`,
    );
  } else {
    if (/usb/i.test(cond[1])) {
      violations.push(`(C) ${FORM_FILE}: условие блока только для чтения опирается на usb.`);
    }
    if (!/card\.printers\.length\s*>\s*0/.test(cond[1])) {
      violations.push(
        `(C) ${FORM_FILE}: условие блока только для чтения не считает \`card.printers.length > 0\`.`,
      );
    }
  }
}

function scanSources(files, violations) {
  if (files.list === undefined) {
    violations.push(`(A) ${LIST_FILE}: файл не читается; отсутствие файла — нарушение, не SKIP.`);
  } else {
    scanList(files.list, violations);
  }
  if (files.form === undefined) {
    violations.push(`(C) ${FORM_FILE}: файл не читается; отсутствие файла — нарушение, не SKIP.`);
  } else {
    scanForm(files.form, violations);
  }
}

function readFiles(root) {
  const read = (f) => {
    try {
      return fs.readFileSync(path.join(root, f), 'utf8');
    } catch {
      return undefined;
    }
  };
  return { list: read(LIST_FILE), form: read(FORM_FILE) };
}

/** Замена с проверкой, что якорь встречается ровно один раз (D-26). */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
  }
  return source.replace(from, () => to);
}

function runSelfTest() {
  const real = readFiles(UI_ROOT);
  if (real.list === undefined || real.form === undefined) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники не читаются.`);
    return false;
  }
  const clean = [];
  scanSources(real, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники обязаны проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  const L = (name, from, to, rules) => ({
    name,
    files: { ...real, list: mutate(real.list, from, to, name) },
    rules,
  });
  const F = (name, from, to, rules) => ({
    name,
    files: { ...real, form: mutate(real.form, from, to, name) },
    rules,
  });

  let mutants;
  try {
    mutants = [
      L(
        '(1) отбор строк сервера только по usb (члены пропадают)',
        "if (p.origin === 'link' || seen.has(p.device_id)) continue;",
        "if (p.origin !== 'usb' || seen.has(p.device_id)) continue;",
        ['B'],
      ),
      L(
        "(2) ПЕРЕСТАНОВКА: 'member' перекрывает ссылку",
        "hasLink ? 'link' : 'member'",
        "hasLink ? 'member' : 'link'",
        ['B'],
      ),
      L(
        '(3) ПЕРЕСТАНОВКА: ссылка выше USB',
        "p.origin === 'usb' ? 'usb' : hasLink ? 'link' : 'member'",
        "hasLink ? 'link' : p.origin === 'usb' ? 'usb' : 'member'",
        ['B'],
      ),
      L(
        "(4) запасной поиск ссылки снова по !== 'usb'",
        "p.device_id === linkId && p.origin === 'link'",
        "p.device_id === linkId && p.origin !== 'usb'",
        ['B'],
      ),
      L('(5) текст «В составе» убран из Badge', '<Badge>В составе</Badge>', '<Badge></Badge>', [
        'A',
      ]),
      L(
        '(6) «В составе» перекрашен в accent',
        '<Badge>В составе</Badge>',
        '<Badge variant="accent">В составе</Badge>',
        ['A'],
      ),
      L('(7) «USB» потерял accent', '<Badge variant="accent">USB</Badge>', '<Badge>USB</Badge>', [
        'A',
      ]),
      L('(8) «Ссылка» вне Badge', '<Badge>Ссылка</Badge>', 'Ссылка', ['A']),
      L(
        '(9) вернулась фраза «появятся здесь сами»',
        "'Принтер появится здесь, если он входит в состав или подключён",
        "'USB-принтеры появятся здесь сами, если он входит в состав или подключён",
        ['D'],
      ),
      L(
        '(10) «ссылкой ниже» и там, где поля добавления нет',
        "или подключён по USB к устройству состава.',",
        "или подключён по USB к устройству состава. Сетевой принтер добавьте ссылкой ниже.',",
        ['D'],
      ),
      L(
        '(11) Table не получает {emptyBody}',
        '    {emptyBody}\n',
        '    emptyBody="Принтеров нет"\n',
        ['D'],
      ),
      L(
        '(12) меню под условием по origin, а не по r.link',
        '{#if canEdit && r.link}',
        "{#if canEdit && r.origin !== 'member'}",
        ['E'],
      ),
      L('(13) меню без условия по ссылке', '{#if canEdit && r.link}', '{#if canEdit}', ['E']),
      F(
        "(14) фильтр вторичных свойств снова по !== 'usb'",
        "x.origin === 'link'",
        "x.origin !== 'usb'",
        ['C'],
      ),
      F(
        '(15) вернулась константа usbPrinters',
        'const hasLinkProperty = $derived(linkPropertyId !== null);\n',
        "const hasLinkProperty = $derived(linkPropertyId !== null);\n  const usbPrinters = $derived(card.printers.filter((p) => p.origin === 'usb'));\n",
        ['C'],
      ),
      F(
        '(16) блок только для чтения опирается на usb',
        '{#if !hasLinkProperty && card.printers.length > 0}',
        "{#if !hasLinkProperty && card.printers.some((p) => p.origin === 'usb')}",
        ['C'],
      ),
      {
        // WR-01 (ревью 41.12): `/*` внутри строчного комментария не открывает блок;
        // блочно-первая вычистка стирала бы нарушение между `templates/*.html` и
        // ближайшим `*/` ниже, гейт молчал бы.
        name: '(17) `/*` в строчном комментарии не прячет usbPrinters',
        files: {
          ...real,
          form: mutate(
            real.form,
            'const hasLinkProperty = $derived(linkPropertyId !== null);\n',
            'const hasLinkProperty = $derived(linkPropertyId !== null);\n' +
              '  // шаблоны лежат в templates/*.html\n' +
              '  const usbPrinters = 1;\n' +
              '  /* настоящий блочный комментарий */\n',
            'comment-trap',
          ),
        },
        rules: ['C'],
      },
      {
        name: '(18) нет блока <script> в списке',
        files: {
          ...real,
          list: mutate(real.list, '<script lang="ts">', '<div lang="ts">', 'no-script').replace(
            '</script>',
            '</div>',
          ),
        },
        rules: ['B'],
      },
      {
        name: '(19) файл списка отсутствует',
        files: { ...real, list: undefined },
        rules: ['A'],
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const m of mutants) {
    const v = [];
    scanSources(m.files, v);
    const hit = m.rules.every((r) => v.some((line) => line.startsWith(`(${r})`)));
    console.error(
      `${TAG} [selftest] ${hit ? 'PASS' : 'FAIL'} — ${m.name} (ожидалось ${m.rules.join('+')}, получено ${v.length})`,
    );
    if (!hit) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  // Зелёный контроль: запретные токены внутри КОММЕНТАРИЯ не краснят.
  try {
    const files = {
      list: mutate(
        real.list,
        '  type RowOrigin =',
        "  // раньше было p.origin !== 'usb' и «появятся здесь сами»\n  type RowOrigin =",
        'comment-ok-list',
      ),
      form: mutate(
        real.form,
        'const hasLinkProperty = $derived(linkPropertyId !== null);\n',
        "const hasLinkProperty = $derived(linkPropertyId !== null);\n  // раньше был usbPrinters и x.origin !== 'usb'\n",
        'comment-ok-form',
      ),
    };
    const v = [];
    scanSources(files, v);
    const ok = v.length === 0;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — запретные токены в комментарии не краснят (ожидалось 0, получено ${v.length})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(
    `${TAG} [selftest] PASS — реальные исходники чисты, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-group-printer-origin.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  scanSources(readFiles(SRC_ROOT), violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
