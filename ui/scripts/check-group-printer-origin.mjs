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
//       литерала, первый (при `canEdit && canLink`) содержит «добавлен ссылкой», второй
//       нет; оба содержат «входит в состав»; ни один не содержит «Сетевой принтер» и
//       «ссылкой ниже» (запрет D-32 снят решением D-16, Фаза 41.13, план 15);
//       фразы «появятся здесь сами» нет нигде;
//   (E) GroupPrintersList: единственный <ActionMenu> стоит под `{#if canEdit && r.link}`
//       (меню «Убрать ссылку» — только у строк с явной ссылкой).
//   (F) GroupPrintersList, подпись хоста под бейджем «USB» (Фаза 41.13, D-14):
//       (F1) скрипт: оба поля хоста читаются из строки сервера только через
//       `usbHost` (`const usbHost = p.origin === 'usb' ? p : null;`), во втором
//       цикле (ссылки из формы) hostName и hostInventoryNumber равны null;
//       (F2) разметка: `{r.hostName}` стоит внутри `{#if r.origin === 'usb'}` и ПОСЛЕ
//       </Badge> с текстом «USB» (порядок: подпись под бейджем, не над ним);
//       (F3) подпись — ТЕКСТ элемента, а не только значение атрибута title;
//       (F4) в ячейке «Связь» нет вывода device_id, в файле нет {@html};
//       (F5) инв. № хоста: `инв.` и `<span class="tr-mono">{r.hostInventoryNumber}</span>`;
//       (F6) подпись под `{#if r.hostName}`: нет имени — нет подписи, без заглушки;
//       (F7) стиль .host: только var(--tr-text-secondary) и var(--tr-font-size-caption),
//       без hex и новых custom properties, есть перенос (overflow-wrap).
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
  // Единственное разрешённое место сравнения кроме origin: охрана полей хоста (F1).
  const hostGuardStmt =
    /\bconst\s+usbHost\s*=\s*\w+\.origin === 'usb'\s*\?\s*\w+\s*:\s*null\s*;/.exec(script);
  if (hostGuardStmt) rest = rest.replace(hostGuardStmt[0], '');
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
        `(D) ${LIST_FILE}: emptyBody не ветвится по \`canEdit && canLink\`: «добавлен ссылкой» нельзя показывать без поля добавления.`,
      );
    }
    if (lits.length !== 2) {
      violations.push(`(D) ${LIST_FILE}: в emptyBody ${lits.length} литералов, нужно два.`);
    } else {
      if (!lits[0].includes('добавлен ссылкой')) {
        violations.push(
          `(D) ${LIST_FILE}: вариант с полем добавления не содержит «добавлен ссылкой».`,
        );
      }
      if (lits[1].includes('добавлен ссылкой')) {
        violations.push(
          `(D) ${LIST_FILE}: вариант без поля добавления говорит «добавлен ссылкой» — текст про то, чего на экране нет.`,
        );
      }
      for (const l of lits) {
        if (l.includes('Сетевой принтер') || l.includes('ссылкой ниже')) {
          violations.push(
            `(D) ${LIST_FILE}: в пустом состоянии вернулась фраза про сетевой принтер или «ссылкой ниже» — запрет D-32 снят (D-16).`,
          );
        }
        if (!l.includes('входит в состав')) {
          violations.push(`(D) ${LIST_FILE}: вариант пустого состояния без «входит в состав».`);
        }
      }
    }
  }

  scanHostLabel(raw, violations, script, markup, code);
}

/** Условия `{#if}`-блоков, охватывающих позицию pos в разметке (ветка else даёт null). */
function enclosingConds(markup, pos) {
  const stack = [];
  const re = /\{(#if|:else if|:else|\/if)\b([^}]*)\}/g;
  let m;
  while ((m = re.exec(markup)) !== null && m.index < pos) {
    if (m[1] === '#if') stack.push(m[2].trim());
    else if (m[1] === ':else if') stack[stack.length - 1] = m[2].trim();
    else if (m[1] === ':else') stack[stack.length - 1] = null;
    else stack.pop();
  }
  return stack;
}

/** Правило (F): подпись хоста под бейджем USB. Вызывается из scanList. */
function scanHostLabel(raw, violations, script, markup, code) {
  // (F1) источник полей.
  const ns = norm(script);
  if (!/const usbHost = (\w+)\.origin === 'usb' \? \1 : null;/.test(ns)) {
    violations.push(
      `(F1) ${LIST_FILE}: нет \`const usbHost = p.origin === 'usb' ? p : null;\`: поля хоста читаются только у USB-строк, иначе подпись приклеится к ссылке и составу.`,
    );
  }
  for (const field of ['usb_host_name', 'usb_host_inventory_number']) {
    const all = [...script.matchAll(new RegExp(`\\b${field}\\b`, 'g'))].length;
    const guarded = [...script.matchAll(new RegExp(`\\busbHost\\?\\.${field}\\b`, 'g'))].length;
    if (all !== 1 || guarded !== 1) {
      violations.push(
        `(F1) ${LIST_FILE}: ${field} читается ${all} раз(а), из них через usbHost? — ${guarded}; нужно ровно одно чтение и только через usbHost?.`,
      );
    }
  }
  for (const [key, want] of [
    ['hostName', 'usbHost?.usb_host_name ?? null'],
    ['hostInventoryNumber', 'usbHost?.usb_host_inventory_number ?? null'],
  ]) {
    const vals = [...ns.matchAll(new RegExp(`\\b${key}: ([^,;]+),`, 'g'))].map((m) => m[1].trim());
    if (vals.length !== 2 || vals[0] !== want || vals[1] !== 'null') {
      violations.push(
        `(F1) ${LIST_FILE}: присваивания ${key} «${vals.join('» и «')}»; ожидается «${want}» для строк сервера и «null» для ссылок из формы.`,
      );
    }
  }

  // (F3) подпись — текст, а не только title: убираем значения атрибутов title.
  const noTitle = markup.replace(/\btitle\s*=\s*(\{[^}]*\}|"[^"]*")/g, 'title=""');
  const textAt = noTitle.indexOf('{r.hostName}');
  if (textAt < 0) {
    violations.push(
      `(F3) ${LIST_FILE}: {r.hostName} не выводится текстом элемента (только в title или нигде): подпись должна читаться без наведения мыши.`,
    );
  } else {
    // (F2) внутри usb-ветки и после бейджа.
    const conds = enclosingConds(noTitle, textAt);
    if (!conds.some((c) => c !== null && /^r\.origin === 'usb'$/.test(c))) {
      violations.push(
        `(F2) ${LIST_FILE}: подпись хоста не лежит внутри \`{#if r.origin === 'usb'}\`: у «Ссылка» и «В составе» подписи быть не должно.`,
      );
    }
    // (F6) охрана по имени.
    if (!conds.some((c) => c !== null && /^r\.hostName$/.test(c))) {
      violations.push(
        `(F6) ${LIST_FILE}: подпись хоста не под \`{#if r.hostName}\`: без имени подписи быть не должно, заглушка с id недопустима.`,
      );
    }
    const badge = /<Badge\b[^>]*>\s*USB\s*<\/Badge>/.exec(noTitle);
    if (!badge || badge.index + badge[0].length > textAt) {
      violations.push(
        `(F2) ${LIST_FILE}: подпись хоста стоит не после <Badge>USB</Badge>: она вторая строка ПОД бейджем.`,
      );
    }
  }

  // (F4) нет device_id в ячейке «Связь» и нет {@html}.
  const cellStart = markup.indexOf("{#if r.origin === 'usb'}");
  const cellEnd = cellStart < 0 ? -1 : markup.indexOf('</td>', cellStart);
  if (cellStart < 0 || cellEnd < 0) {
    violations.push(`(F4) ${LIST_FILE}: ячейка «Связь» (usb-ветка до </td>) не найдена.`);
  } else if (/device_id/.test(markup.slice(cellStart, cellEnd))) {
    violations.push(
      `(F4) ${LIST_FILE}: в ячейке «Связь» выводится device_id: сырой id хоста на экран не попадает.`,
    );
  }
  if (/\{@html\b/.test(code)) {
    violations.push(`(F4) ${LIST_FILE}: {@html} в файле: имя хоста вводят пользователи (XSS).`);
  }

  // (F5) инв. № хоста.
  if (
    !/инв\.\s*<span class="tr-mono">\{r\.hostInventoryNumber\}<\/span>/.test(
      markup.replace(/\s+/g, ' '),
    )
  ) {
    violations.push(
      `(F5) ${LIST_FILE}: нет \`инв. <span class="tr-mono">{r.hostInventoryNumber}</span>\`: формат «Имя · инв. N» с моноширинным номером.`,
    );
  }

  // (F7) стиль .host.
  const styleM = /<style\b[^>]*>([\s\S]*?)<\/style>/.exec(code);
  const hostRule = styleM ? /\.host\s*\{([^}]*)\}/.exec(styleM[1]) : null;
  if (!hostRule) {
    violations.push(`(F7) ${LIST_FILE}: нет стилевого правила .host.`);
  } else {
    const body = hostRule[1];
    if (!/color:\s*var\(--tr-text-secondary\)/.test(body)) {
      violations.push(`(F7) ${LIST_FILE}: .host без color: var(--tr-text-secondary).`);
    }
    if (!/font-size:\s*var\(--tr-font-size-caption\)/.test(body)) {
      violations.push(`(F7) ${LIST_FILE}: .host без font-size: var(--tr-font-size-caption).`);
    }
    if (/#[0-9a-fA-F]{3,8}\b/.test(body) || /^\s*--[\w-]+\s*:/m.test(body)) {
      violations.push(`(F7) ${LIST_FILE}: в .host hex-цвет или новая custom property.`);
    }
    if (!/overflow-wrap:\s*(anywhere|break-word)/.test(body)) {
      violations.push(
        `(F7) ${LIST_FILE}: у .host нет overflow-wrap: длинное название хоста обрежется, а не перенесётся.`,
      );
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

  // Блок подписи хоста целиком (для мутантов-перемещений).
  const HOST_BLOCK = `{#if r.hostName}
              <span class="host" title={hostLabel(r)}
                >{r.hostName}{#if r.hostInventoryNumber}
                  · инв. <span class="tr-mono">{r.hostInventoryNumber}</span>{/if}</span
              >
            {/if}`;
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
        '(10) «добавлен ссылкой» и там, где поля добавления нет',
        "или подключён по USB к устройству состава.',",
        "или подключён по USB к устройству состава или добавлен ссылкой.',",
        ['D'],
      ),
      L(
        '(10b) вернулась фраза «Сетевой принтер добавьте ссылкой ниже.» (D-32)',
        "или добавлен ссылкой.'",
        "или добавлен ссылкой. Сетевой принтер добавьте ссылкой ниже.'",
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
      // --- Правило F: подпись хоста под бейджем USB (D-14) ---
      {
        name: '(20) подпись хоста перенесена в ветку «В составе» (под не-USB бейдж)',
        files: {
          ...real,
          list: mutate(
            mutate(real.list, HOST_BLOCK, '', '(20) вырезать блок'),
            '<Badge>В составе</Badge>',
            `<Badge>В составе</Badge>\n${HOST_BLOCK}`,
            '(20) вставить в else',
          ),
        },
        rules: ['F2'],
      },
      L(
        '(21) условие r.origin === usb вокруг подписи убрано',
        "{#if r.origin === 'usb'}",
        '{#if true}',
        ['F2'],
      ),
      L(
        '(22) {r.hostName} оставлено только в title',
        `title={hostLabel(r)}
                >{r.hostName}{#if`,
        `title={r.hostName}
                >{#if`,
        ['F3'],
      ),
      {
        name: '(23) ПЕРЕСТАНОВКА: подпись стоит ПЕРЕД бейджем USB (внутри usb-ветки)',
        files: {
          ...real,
          list: mutate(
            mutate(real.list, HOST_BLOCK, '', '(23) вырезать блок'),
            '<Badge variant="accent">USB</Badge>',
            `${HOST_BLOCK}\n<Badge variant="accent">USB</Badge>`,
            '(23) вставить выше Badge',
          ),
        },
        rules: ['F2'],
      },
      L(
        '(24) строка сервера читает p.name вместо usb_host_name',
        'hostName: usbHost?.usb_host_name ?? null,',
        'hostName: p.name,',
        ['F1'],
      ),
      L(
        '(25) охрана usbHost снята: поля читаются у любой строки',
        "const usbHost = p.origin === 'usb' ? p : null;",
        'const usbHost = p;',
        ['F1'],
      ),
      L(
        '(26) в ячейке выведен {r.device_id}',
        '<Badge>В составе</Badge>',
        '<Badge>В составе</Badge>{r.device_id}',
        ['F4'],
      ),
      L('(27) {@html r.hostName}', '>{r.hostName}{#if', '>{@html r.hostName}{r.hostName}{#if', [
        'F4',
      ]),
      L(
        '(28a) инв. № хоста без tr-mono',
        '<span class="tr-mono">{r.hostInventoryNumber}</span>',
        '<span>{r.hostInventoryNumber}</span>',
        ['F5'],
      ),
      L('(28b) инв. № хоста без слова «инв.»', '· инв. <span', '· <span', ['F5']),
      L(
        '(29) подпись без охраны {#if r.hostName} (заглушка при пустом имени)',
        '{#if r.hostName}',
        '{#if true}',
        ['F6'],
      ),
      L(
        '(30) ссылка из формы получает hostName',
        'hostName: null,\n        hostInventoryNumber: null,\n        link: true,',
        'hostName: info.name,\n        hostInventoryNumber: null,\n        link: true,',
        ['F1'],
      ),
      L(
        '(31) .host с hex-цветом вместо токена',
        'color: var(--tr-text-secondary);',
        'color: #586074;',
        ['F7'],
      ),
      L('(32) у .host нет переноса', 'overflow-wrap: anywhere;', '', ['F7']),
      L(
        '(33) .host с размером в px вместо токена',
        'font-size: var(--tr-font-size-caption);\n    color',
        'font-size: 12px;\n    color',
        ['F7'],
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
        "  // раньше было p.origin !== 'usb', {@html x}, {r.device_id}, hostName: p.name, usb_host_name и «появятся здесь сами»\n  type RowOrigin =",
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
