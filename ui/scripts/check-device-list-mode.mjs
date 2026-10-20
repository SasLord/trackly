#!/usr/bin/env node
// [check-device-list-mode] Гейт режимов списка «Устройства» (фаза 41.1, план 19;
// GRD-02, GRD-03, GRP-10; D-10, D-11, D-34).
//
// Два слоя.
//
// 1. ИСПОЛНЯЮЩИЙ golden-гейт чистого модуля `src/features/devices/deviceListMode.ts`.
//    Модуль транспилируется пакетом `typescript` и гоняется по фикстуре
//    `scripts/fixtures/device-list-mode/cases.json`: таблица истинности скрытия
//    членов якорных групп, флаги по виду списка, формула подписи футера, а также
//    (план 24, GAP-1) groupIdsKey — ключ состава псевдогруппы, от которого зависит
//    валидность кеша детей DeviceGroupRow, — и visibleChildren — фильтр детей по
//    актуальному group.ids. Кейсы visibleChildren различаются тем, КАКОЙ id покинул
//    состав (первый, средний, последний): три действия над ребёнком свёртки
//    («Замещение группой», «На разбор», «Добавить в группу» с якорной группой).
//    Каждый кейс подобран так, чтобы потеря одного условия или перестановка флага
//    меняла результат.
//
//    ОБЩЕЙ ФИКСТУРЫ С RUST У МОДУЛЯ НЕТ, и это сознательно: модуль НЕ зеркалит
//    серверное правило «какие устройства скрыть» (оно целиком в SQL плана 12,
//    его проверяют тесты devices_anchor_flags / devices_anchor_list). Модуль
//    решает только, КОГДА страница включает флаг, и как подписать число K, которое
//    пришло с сервера. Не защищено этим гейтом: что сервер при включённом флаге
//    действительно скрывает именно членов якорей и что K равно числу скрытых —
//    это держат серверные тесты.
//
// 2. СТРУКТУРНЫЕ правила на исходниках `src/` (комментарии вырезаются):
//    [A] DevicesPage.svelte собирает фильтр ТОЛЬКО через deviceListFlags(...):
//        вызовы для всех четырёх видов ('flat', 'grouped', 'anchors', 'members'),
//        в контексте каждого вызова есть ключи searchActive, statusFilter и
//        focusId (потеря focusId возвращает регресс W3: переход «Перейти к
//        устройству» к детали-члену якоря не находил строки для подсветки),
//        hideActive считается через shouldHideAnchorMembers(, а собственных
//        выражений hide_anchor_members / anchors_only / exclude_anchors в
//        странице нет;
//    [B] DeviceList.svelte подпись футера берёт из footerText( и не содержит
//        собственного литерала «Показано »;
//    [D] opt-in флаги (hide_anchor_members, anchors_only, exclude_anchors,
//        member_of_group_id) не встречаются в коде НИ В ОДНОМ другом файле src/.
//        Набор файлов выводится обходом, не ручным списком: новый вызывающий
//        (пикер акта, модалка массового добавления), поставивший флаг, краснит
//        гейт, а не меняет поведение молча;
//    [G] DeviceGroupRow.svelte (план 24, GAP-1; файл обязателен всегда, не SKIP):
//        [G1] импортирует groupIdsKey и visibleChildren из './deviceListMode';
//        [G2] не рисует `{#each children` по сырому кешу (только visibleChildren);
//        [G3] есть $effect, читающий expanded и cacheKey и НЕ читающий
//             `children === null` (до-фиксовый эффект ключа состава не знал);
//        [G4] вложенный <DeviceListRow> получает onEdit= и onDelete= (путь
//             «действие из меню -> refresh()» не рвётся);
//        [G5] (CR-02, план 41.11-02) ключ кеша детей cacheKey объявлен как $derived
//             от шаблонной строки с idsKey И listGen, а loadChildren( зовётся с
//             cacheKey: правка поля вне ключа группировки поднимает номер поколения
//             и перечитывает детей;
//        [G6] handleEdit и handleDelete не содержат loadedKey (второго пути
//             инвалидации кеша нет: он перечитывал детей ДО сохранения правки);
//        [G7] (WR-08) подлежащее первого {#each — переменная, объявленная как
//             $derived(visibleChildren(, а не сырой кеш children.
//        Вакуумная проверка: самотест читает замороженную до-фиксовую копию
//        scripts/fixtures/device-list-mode/DeviceGroupRow.pre-fix.svelte.txt и
//        требует от неё [G1], [G2], [G3]; на реальном файле ноль нарушений.
//
//    [H] DevicesPage.svelte — защита от устаревшего ответа (план 29, WR-04; ПОРЯДОК
//        важен: счётный гейт слеп к порядку, охрана ПОСЛЕ присваивания прошла бы по счёту):
//        [H1] в refresh() `const seq = ++refreshSeq` стоит раньше первого await;
//        [H2] первая охрана `if (seq !== refreshSeq) return` после последнего await
//             стоит ДО первого присваивания items/groups/total/anchorItems/
//             hiddenMemberCount (удалённая охрана и охрана после присваивания краснят);
//        [H3] fetchList() не присваивает эти поля и loading (только запросы);
//        [H4] первый оператор catch в refresh() — охрана поколения, а сброс
//             loading в finally стоит под условием seq === refreshSeq;
//        [H5] reloadExpandedAnchors(): `const aSeq = ++anchorsSeq` раньше первого
//             await, охрана aSeq !== anchorsSeq после await и раньше первого
//             `anchorMembers =` и вызова failAnchor(.
//        Живое поведение гонки (быстрое переключение «Свернуть одинаковые») этим
//        гейтом не проверяется: нужна проверка в запущенном приложении (план 32).
//        Вакуумная проверка (WR-05, план 41.11-10): самотест читает замороженную
//        до-фиксовую копию scripts/fixtures/device-list-mode/DevicesPage.pre-fix.svelte.txt
//        (`git show a7c25aeb^:ui/src/features/devices/DevicesPage.svelte`), требует от неё
//        [H1], [H2], [H5] и защиту от подмены (в копии нет `refreshSeq`, есть
//        `async function refresh`); реальная страница даёт ноль нарушений, а мутанты
//        [H1]/[H2]/[H5] строятся на НАСТОЯЩЕМ DevicesPage.svelte, не на синтетике.
//
//    [H7] (CR-02, план 41.11-10; порядковое правило, мутант — ПЕРЕСТАНОВКА): номер
//        поколения списка listGen:
//        - в DevicesPage ровно одно приращение, в refresh(), ПОСЛЕ охраны
//          `if (seq !== refreshSeq) return` и ДО `void reloadExpandedAnchors(`;
//        - <DeviceList в DevicesPage и <DeviceGroupRow в DeviceList передают {listGen}
//          или listGen={listGen}: значение пропа обязано быть ИДЕНТИФИКАТОРОМ listGen,
//          литерал (listGen={0}) и чужое имя (other={listGen}) красные, потому что
//          литерал замораживает ключ кеша idsKey@listGen;
//        - в Props DeviceList и DeviceGroupRow `listGen: number` ОБЯЗАТЕЛЕН (без `?`
//          и без значения по умолчанию: необязательный проп молча возвращает CR-02).
//
//    Правила [A] и [B] при PAGE_MANDATORY = false дают явный SKIP для страницы,
//    которая модуль ещё не подключила (так было в задаче 1 плана 19). Задача 3
//    подключила страницу и перевела константу в true: отсутствие файла или
//    вызовов после этого нарушение, а не SKIP (SKIP-able файл молчал бы вечно).
//    Self-test проверяет оба режима.
//
// Self-test (`--selftest`): мутанты РЕАЛЬНОГО deviceListMode.ts (якорь каждого
// встречается ровно один раз, иначе replace молча отключил бы соседний фрагмент)
// и мутанты структурных правил на фикстурах в памяти (у каждой «до» зелёная,
// «после» красная с указанным кодом). Только node:fs/node:path/node:url/node:module.
//
// Usage:
//   node scripts/check-device-list-mode.mjs               # проверить репозиторий
//   node scripts/check-device-list-mode.mjs --src=<root>  # КОРЕНЬ копии с каталогом src/ внутри
//   node scripts/check-device-list-mode.mjs --impl=<path> # копия deviceListMode.ts для golden-слоя
//   node scripts/check-device-list-mode.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-list-mode]';

const MODULE_FILE = 'src/features/devices/deviceListMode.ts';
const PAGE_FILE = 'src/features/devices/DevicesPage.svelte';
const LIST_FILE = 'src/features/devices/DeviceList.svelte';
const GROUP_ROW_FILE = 'src/features/devices/DeviceGroupRow.svelte';
const PRE_FIX_GROUP_ROW = path.join(
  __dirname,
  'fixtures/device-list-mode/DeviceGroupRow.pre-fix.svelte.txt',
);
const PRE_FIX_PAGE = path.join(
  __dirname,
  'fixtures/device-list-mode/DevicesPage.pre-fix.svelte.txt',
);
const FIXTURE = path.join(__dirname, 'fixtures/device-list-mode/cases.json');
const MIN_CASES = 18;
// Сколько .svelte/.ts обязан увидеть структурный проход: меньше — гейт слеп.
const MIN_SCANNED_FILES = 50;
const SELFTEST_MIN_SCANNED_FILES = 1;

// Задача 3 плана 19 подключила страницу: после этого SKIP для [A]/[B] недопустим.
const PAGE_MANDATORY = true;

const KINDS = ['flat', 'grouped', 'anchors', 'members'];
const CTX_KEYS = ['searchActive', 'statusFilter', 'focusId'];
const FLAG_RE = /\b(?:hide_anchor_members|anchors_only|exclude_anchors)\b/;
const MEMBER_RE = /\bmember_of_group_id\b/;
// Файлы, где флаги законно упоминаются: сгенерированные привязки и сам модуль.
const FLAG_HOMES = new Set(['src/bindings.ts', MODULE_FILE]);
const MEMBER_HOMES = new Set(['src/bindings.ts', MODULE_FILE, PAGE_FILE]);

// ---------------------------------------------------------------------------
// Слой 1: исполняемый golden-гейт
// ---------------------------------------------------------------------------

function loadTypescript() {
  const require = createRequire(import.meta.url);
  try {
    return require('typescript');
  } catch {
    console.error(
      `${TAG} FAIL — не разрешается пакет \`typescript\` (devDependency ui/package.json); запусти \`pnpm install\` в ui/.`,
    );
    process.exit(1);
  }
}

async function loadModuleFromTs(tsSource, label) {
  const ts = loadTypescript();
  const js = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: label,
    reportDiagnostics: false,
  }).outputText;
  // data:-URL: модуль самодостаточен (ни одного import, кроме типов).
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

function loadFixture() {
  let cases;
  try {
    cases = JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
  } catch (e) {
    console.error(
      `${TAG} FAIL — фикстура ${path.relative(UI_ROOT, FIXTURE)} не читается: ${e.message}`,
    );
    process.exit(1);
  }
  if (!Array.isArray(cases) || cases.length < MIN_CASES) {
    console.error(
      `${TAG} FAIL — в фикстуре ${Array.isArray(cases) ? cases.length : 'не массив'} кейсов, ожидалось не меньше ${MIN_CASES}. Её урезали вместо починки функции?`,
    );
    process.exit(1);
  }
  return cases;
}

function canonical(v) {
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v);
}

const FUNCTIONS = [
  'shouldHideAnchorMembers',
  'deviceListFlags',
  'footerText',
  'groupIdsKey',
  'visibleChildren',
];

/** Возвращает список текстов провалов (пустой — всё в порядке). */
function runCases(mod, cases) {
  const failures = [];
  for (const fn of FUNCTIONS) {
    if (typeof mod[fn] !== 'function') failures.push(`модуль не экспортирует функцию ${fn}`);
  }
  if (failures.length) return failures;

  const covered = new Set();
  // Сколько раз кейсы дали true / false для скрытия: фикстура обязана видеть оба исхода.
  const hideOutcomes = new Set();
  let focusOnlyCase = false;
  // groupIdsKey / visibleChildren (план 24, GAP-1): фикстура обязана видеть и перестановку
  // тех же id, и случай, где ребёнок покинул состав (иначе тождественный возврат проходил бы).
  const keyCases = [];
  let shrinkingChildrenCase = false;
  for (const c of cases) {
    covered.add(c.fn);
    try {
      let got;
      if (c.fn === 'shouldHideAnchorMembers') {
        got = mod.shouldHideAnchorMembers(JSON.parse(JSON.stringify(c.input)));
        hideOutcomes.add(c.expected);
        if (
          c.input.searchActive === false &&
          c.input.statusFilter === null &&
          typeof c.input.focusId === 'number' &&
          c.expected === false
        ) {
          focusOnlyCase = true;
        }
      } else if (c.fn === 'deviceListFlags') {
        got = mod.deviceListFlags(c.input.kind, JSON.parse(JSON.stringify(c.input.ctx)));
      } else if (c.fn === 'footerText') {
        got = mod.footerText(JSON.parse(JSON.stringify(c.input)));
      } else if (c.fn === 'groupIdsKey') {
        const ids = JSON.parse(JSON.stringify(c.input.ids));
        got = mod.groupIdsKey(ids);
        // Входной массив не мутируется (сортировка идёт по копии).
        if (canonical(ids) !== canonical(c.input.ids)) {
          failures.push(`${c.name}: groupIdsKey изменила входной массив`);
        }
        keyCases.push(c.input.ids);
      } else if (c.fn === 'visibleChildren') {
        got = mod.visibleChildren(
          JSON.parse(JSON.stringify(c.input.children)),
          JSON.parse(JSON.stringify(c.input.ids)),
        );
        if (c.expected.length < c.input.children.length) shrinkingChildrenCase = true;
      } else {
        failures.push(`${c.name}: неизвестная функция «${c.fn}» в фикстуре`);
        continue;
      }
      if (canonical(got) !== canonical(c.expected)) {
        failures.push(`${c.name}: получено ${canonical(got)}, ожидалось ${canonical(c.expected)}`);
      }
    } catch (e) {
      failures.push(`${c.name}: исключение ${e.message}`);
    }
  }
  for (const fn of FUNCTIONS) {
    if (!covered.has(fn)) failures.push(`в фикстуре нет ни одного кейса для ${fn}`);
  }
  if (!hideOutcomes.has(true) || !hideOutcomes.has(false)) {
    failures.push('в фикстуре shouldHideAnchorMembers нет обоих исходов (true и false)');
  }
  if (!focusOnlyCase) {
    failures.push(
      'в фикстуре нет кейса «ничего не включено, кроме focusId = число» -> false (регресс W3)',
    );
  }
  const permutedKey = keyCases.some((a, i) =>
    keyCases.some(
      (b, j) =>
        i !== j &&
        canonical(a) !== canonical(b) &&
        canonical([...a].sort((x, y) => x - y)) === canonical([...b].sort((x, y) => x - y)),
    ),
  );
  if (!permutedKey) {
    failures.push('в фикстуре нет двух кейсов groupIdsKey с перестановкой тех же id');
  }
  if (!shrinkingChildrenCase) {
    failures.push(
      'в фикстуре нет кейса visibleChildren, где ребёнок покинул состав (результат короче children)',
    );
  }
  // Каждый вид списка обязан быть в фикстуре при обоих исходах скрытия.
  for (const kind of KINDS) {
    for (const hide of [true, false]) {
      const has = cases.some(
        (c) =>
          c.fn === 'deviceListFlags' &&
          c.input.kind === kind &&
          (c.input.ctx.searchActive === false &&
          c.input.ctx.statusFilter === null &&
          c.input.ctx.focusId === null
            ? true
            : false) === hide,
      );
      if (!has) failures.push(`в фикстуре нет кейса deviceListFlags(${kind}) при hide=${hide}`);
    }
  }
  return failures;
}

// ---------------------------------------------------------------------------
// Слой 2: разбор исходников
// ---------------------------------------------------------------------------

/** Пробелы вместо всех символов, кроме переводов строк (длина сохраняется). */
function blank(s) {
  return s.replace(/[^\n]/g, ' ');
}

/** Вырезает HTML-, блочные и строчные комментарии, сохраняя нумерацию строк. */
export function stripComments(src) {
  return src
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[\s;{(,])\/\/[^\n]*/g, (m, pre, offset, whole) => {
      // `//` внутри строкового литерала на той же строке — не комментарий.
      const lineStart = whole.lastIndexOf('\n', offset) + 1;
      const before = whole.slice(lineStart, offset + pre.length);
      for (const q of ['"', "'", '`']) {
        const n = (before.match(new RegExp(`(?<!\\\\)\\${q}`, 'g')) ?? []).length;
        if (n % 2 === 1) return m;
      }
      return pre + ' '.repeat(m.length - pre.length);
    });
}

/** Индекс закрывающей скобки для открывающей в text[openIdx]; -1 — не нашлось. */
function readBalanced(text, openIdx, open, close) {
  let depth = 0;
  let quote = null;
  for (let i = openIdx; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === '\\') i++;
      else if (c === quote) quote = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      quote = c;
      continue;
    }
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function lineOf(text, idx) {
  return text.slice(0, idx).split('\n').length;
}

/** Литерал объекта, которым объявлен идентификатор (`const NAME = $derived<T>({ ... })`). */
function objectLiteralOf(name, text) {
  const decl = new RegExp(`(?:const|let|var)\\s+${name.replace(/\$/g, '\\$')}\\b`).exec(text);
  if (!decl) return null;
  const eq = text.indexOf('=', decl.index);
  if (eq === -1) return null;
  const semi = text.indexOf(';', eq);
  const brace = text.indexOf('{', eq);
  // Объявление без литерала (`= makeCtx();`): `{` дальше по файлу — уже чужой код.
  if (brace === -1 || (semi !== -1 && semi < brace)) return null;
  const end = readBalanced(text, brace, '{', '}');
  return end === -1 ? null : text.slice(brace, end + 1);
}

function hasKey(literal, key) {
  // `key:` или shorthand `key,` / `key }` на верхнем уровне литерала.
  return new RegExp(`(?:^|[{,\\s])${key}\\s*(?::|,|\\})`).test(literal);
}

/** Аргументы вызова `fn(` на позиции open: возвращает массив строк-аргументов верхнего уровня. */
function splitArgs(text, openIdx) {
  const close = readBalanced(text, openIdx, '(', ')');
  if (close === -1) return null;
  const inner = text.slice(openIdx + 1, close);
  const args = [];
  let depth = 0;
  let quote = null;
  let cur = '';
  for (let i = 0; i < inner.length; i++) {
    const c = inner[i];
    if (quote) {
      cur += c;
      if (c === '\\') {
        cur += inner[++i] ?? '';
      } else if (c === quote) quote = null;
      continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      quote = c;
      cur += c;
      continue;
    }
    if (c === '(' || c === '{' || c === '[') depth++;
    if (c === ')' || c === '}' || c === ']') depth--;
    if (c === ',' && depth === 0) {
      args.push(cur.trim());
      cur = '';
    } else cur += c;
  }
  if (cur.trim()) args.push(cur.trim());
  return args;
}

/** Проверка контекста одного вызова: литерал или идентификатор, разрешаемый в литерал. */
function checkCtxArg(arg, text, where) {
  const out = [];
  if (arg === undefined) {
    out.push({ code: 'A', msg: `${where}: нет аргумента-контекста` });
    return out;
  }
  let literal = null;
  if (arg.startsWith('{')) literal = arg;
  else if (/^[A-Za-z_$][\w$]*$/.test(arg)) literal = objectLiteralOf(arg, text);
  if (literal === null) {
    out.push({
      code: 'A',
      msg: `${where}: контекст «${arg}» не разрешается в литерал с ключами ${CTX_KEYS.join(', ')}`,
    });
    return out;
  }
  for (const k of CTX_KEYS) {
    if (!hasKey(literal, k)) out.push({ code: 'A', msg: `${where}: в контексте нет ключа ${k}` });
  }
  return out;
}

/** Правило [A]: DevicesPage.svelte. */
export function checkPage(src) {
  const text = stripComments(src);
  const out = [];
  const seenKinds = new Set();
  const callRe = /\bdeviceListFlags\s*\(/g;
  let m;
  while ((m = callRe.exec(text)) !== null) {
    const open = m.index + m[0].length - 1;
    const args = splitArgs(text, open);
    const line = lineOf(text, m.index);
    if (args === null) {
      out.push({ code: 'A', msg: `строка ${line}: вызов deviceListFlags( не закрыт` });
      continue;
    }
    const kindMatch = /^(['"`])(\w+)\1$/.exec(args[0] ?? '');
    if (!kindMatch || !KINDS.includes(kindMatch[2])) {
      out.push({
        code: 'A',
        msg: `строка ${line}: первый аргумент deviceListFlags( должен быть литералом одного из ${KINDS.join(', ')}`,
      });
    } else seenKinds.add(kindMatch[2]);
    for (const v of checkCtxArg(args[1], text, `строка ${line}: deviceListFlags(`)) out.push(v);
  }
  for (const k of KINDS) {
    if (!seenKinds.has(k)) out.push({ code: 'A', msg: `нет вызова deviceListFlags('${k}', ...)` });
  }
  const hideRe = /\bshouldHideAnchorMembers\s*\(/g;
  let hides = 0;
  while ((m = hideRe.exec(text)) !== null) {
    hides++;
    const open = m.index + m[0].length - 1;
    const args = splitArgs(text, open);
    const line = lineOf(text, m.index);
    for (const v of checkCtxArg(args?.[0], text, `строка ${line}: shouldHideAnchorMembers(`)) {
      out.push(v);
    }
  }
  if (hides === 0) {
    out.push({
      code: 'A',
      msg: 'hideActive футера не считается через shouldHideAnchorMembers( (иначе подпись и запрос расходятся)',
    });
  }
  const own = FLAG_RE.exec(text);
  if (own) {
    out.push({
      code: 'A',
      msg: `строка ${lineOf(text, own.index)}: собственное выражение ${own[0]} вне deviceListFlags (флаги ставит только модуль)`,
    });
  }
  // K и признак скрытия доходят до DeviceList (иначе футер молча остаётся прежним).
  for (const prop of ['hideActive', 'hiddenMemberCount', 'anchorItems']) {
    if (!new RegExp(`\\{${prop}\\}|\\b${prop}\\s*=\\s*\\{`).test(text)) {
      out.push({ code: 'A', msg: `DeviceList не получает ${prop} ({${prop}} или ${prop}={...})` });
    }
  }
  if (!MEMBER_RE.test(text)) {
    out.push({ code: 'A', msg: 'нет member_of_group_id: раскрытие состава якоря не загружается' });
  }
  return out;
}

/** Тело функции `name` (от `{` до парной `}` включительно) или null. */
function functionBody(text, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(text);
  if (!m) return null;
  const parenOpen = m.index + m[0].length - 1;
  const parenClose = readBalanced(text, parenOpen, '(', ')');
  if (parenClose === -1) return null;
  const open = text.indexOf('{', parenClose);
  if (open === -1) return null;
  const close = readBalanced(text, open, '{', '}');
  if (close === -1) return null;
  return text.slice(open, close + 1);
}

const STATE_ASSIGN_RE = /(?<![\w.])(?:items|groups|total|anchorItems|hiddenMemberCount)\s*=(?!=)/;
const REFRESH_GUARD_RE = /if\s*\(\s*seq\s*!==\s*refreshSeq\s*\)\s*return\b/;
const ANCHORS_GUARD_RE = /if\s*\(\s*aSeq\s*!==\s*anchorsSeq\s*\)\s*return\b/;

function lastIndexOfRe(text, re) {
  const g = new RegExp(re.source, re.flags.includes('g') ? re.flags : `${re.flags}g`);
  let idx = -1;
  let m;
  while ((m = g.exec(text)) !== null) idx = m.index;
  return idx;
}

/** Правило [H]: DevicesPage.svelte, защита поколения refresh() и reloadExpandedAnchors(). */
export function checkPageRace(src) {
  const text = stripComments(src);
  const out = [];
  const refresh = functionBody(text, 'refresh');
  if (refresh === null) {
    out.push({ code: 'H1', msg: 'нет функции refresh() (защита поколения не проверяется)' });
  } else {
    const inc =
      /\b(?:const\s+seq\s*=\s*\+\+\s*refreshSeq|refreshSeq\s*\+\+|refreshSeq\s*\+=\s*1)/.exec(
        refresh,
      );
    const firstAwait = refresh.search(/\bawait\b/);
    if (!inc || !/\bconst\s+seq\s*=/.test(refresh)) {
      out.push({ code: 'H1', msg: 'refresh(): нет `const seq = ++refreshSeq`' });
    } else if (firstAwait !== -1 && inc.index > firstAwait) {
      out.push({ code: 'H1', msg: 'refresh(): номер поколения берётся ПОСЛЕ первого await' });
    }
    const lastAwait = lastIndexOfRe(refresh, /\bawait\b/);
    const assign = STATE_ASSIGN_RE.exec(refresh);
    const afterAwait = refresh.slice(lastAwait === -1 ? 0 : lastAwait);
    const guardRel = afterAwait.search(REFRESH_GUARD_RE);
    const guardAt = guardRel === -1 ? -1 : (lastAwait === -1 ? 0 : lastAwait) + guardRel;
    if (lastAwait === -1) {
      out.push({ code: 'H2', msg: 'refresh(): нет await (загрузка не ждёт ответа)' });
    } else if (guardAt === -1) {
      out.push({
        code: 'H2',
        msg: 'refresh(): после await нет `if (seq !== refreshSeq) return` (устаревший ответ применяется)',
      });
    } else if (!assign) {
      out.push({ code: 'H2', msg: 'refresh(): нет присваиваний состояния списка' });
    } else if (guardAt > assign.index) {
      out.push({
        code: 'H2',
        msg: `refresh(): охрана поколения стоит ПОСЛЕ присваивания «${assign[0]}» (устаревший ответ успевает записать состояние)`,
      });
    }
    // [H4]: catch и finally.
    const catchM = /\bcatch\b\s*(?:\([^)]*\))?\s*\{/.exec(refresh);
    if (!catchM) {
      out.push({ code: 'H4', msg: 'refresh(): нет блока catch' });
    } else {
      const open = catchM.index + catchM[0].length - 1;
      const close = readBalanced(refresh, open, '{', '}');
      const inner = close === -1 ? refresh.slice(open + 1) : refresh.slice(open + 1, close);
      if (!new RegExp(`^\\s*${REFRESH_GUARD_RE.source}`).test(inner)) {
        out.push({
          code: 'H4',
          msg: 'refresh(): первым оператором catch нет охраны поколения (устаревшая ошибка покажет тост)',
        });
      }
    }
    const finM = /\bfinally\s*\{/.exec(refresh);
    if (!finM) {
      out.push({ code: 'H4', msg: 'refresh(): нет блока finally' });
    } else {
      const open = finM.index + finM[0].length - 1;
      const close = readBalanced(refresh, open, '{', '}');
      const inner = close === -1 ? refresh.slice(open + 1) : refresh.slice(open + 1, close);
      const cond = /if\s*\(\s*seq\s*===\s*refreshSeq\s*\)\s*loading\s*=\s*false\b/;
      if (!cond.test(inner) || /(?<![\w.])loading\s*=(?!=)/.test(inner.replace(cond, ''))) {
        out.push({
          code: 'H4',
          msg: 'refresh(): в finally loading сбрасывается не под условием seq === refreshSeq',
        });
      }
    }
  }
  const fetchList = functionBody(text, 'fetchList');
  if (fetchList === null) {
    out.push({ code: 'H3', msg: 'нет функции fetchList()' });
  } else {
    const a = STATE_ASSIGN_RE.exec(fetchList) ?? /(?<![\w.])loading\s*=(?!=)/.exec(fetchList);
    if (a) {
      out.push({
        code: 'H3',
        msg: `fetchList(): присваивание «${a[0]}» (загрузка только запрашивает, состояние пишет refresh() после охраны)`,
      });
    }
  }
  const reload = functionBody(text, 'reloadExpandedAnchors');
  if (reload === null) {
    out.push({ code: 'H5', msg: 'нет функции reloadExpandedAnchors()' });
  } else {
    const inc = /\bconst\s+aSeq\s*=\s*\+\+\s*anchorsSeq\b/.exec(reload);
    const firstAwait = reload.search(/\bawait\b/);
    const lastAwait = lastIndexOfRe(reload, /\bawait\b/);
    if (!inc) {
      out.push({ code: 'H5', msg: 'reloadExpandedAnchors(): нет `const aSeq = ++anchorsSeq`' });
    } else if (firstAwait !== -1 && inc.index > firstAwait) {
      out.push({ code: 'H5', msg: 'reloadExpandedAnchors(): aSeq берётся ПОСЛЕ первого await' });
    }
    const guardRel = reload.slice(lastAwait === -1 ? 0 : lastAwait).search(ANCHORS_GUARD_RE);
    const guardAt = guardRel === -1 ? -1 : (lastAwait === -1 ? 0 : lastAwait) + guardRel;
    const am = /(?<![\w.])anchorMembers\s*=(?!=)/.exec(reload);
    const fa = /\bfailAnchor\s*\(/.exec(reload);
    if (lastAwait === -1 || guardAt === -1) {
      out.push({
        code: 'H5',
        msg: 'reloadExpandedAnchors(): после await нет `if (aSeq !== anchorsSeq) return`',
      });
    } else if ((am && guardAt > am.index) || (fa && guardAt > fa.index)) {
      out.push({
        code: 'H5',
        msg: 'reloadExpandedAnchors(): охрана стоит ПОСЛЕ записи anchorMembers / вызова failAnchor(',
      });
    }
    // [H6] Перечитывание обязано обесценить одиночные загрузки тех же якорей,
    // иначе отставший ответ loadAnchorMembers ляжет поверх свежего состава.
    const bump = /\bbumpAnchorLoad\s*\(/.exec(reload);
    if (!bump) {
      out.push({
        code: 'H6',
        msg: 'reloadExpandedAnchors(): нет bumpAnchorLoad( для перечитываемых якорей',
      });
    } else if (firstAwait !== -1 && bump.index > firstAwait) {
      out.push({
        code: 'H6',
        msg: 'reloadExpandedAnchors(): bumpAnchorLoad( вызван ПОСЛЕ первого await',
      });
    }
  }
  out.push(...checkAnchorMembersLoad(text));
  return out;
}

const LIST_GEN_INC_RE = /(?<![\w.])listGen\s*(?:\+=\s*1|\+\+|=\s*listGen\s*\+\s*1)/g;

/**
 * Правило [H7] (CR-02, D-09): порядок и проводка номера поколения списка listGen.
 * Порядковое правило, не счётное: инкремент до охраны устаревшего ответа перезапускал
 * бы загрузку детей под старым составом (счётный гейт слеп к порядку).
 *   1. в DevicesPage ровно ОДНО приращение listGen, оно в refresh(), ПОСЛЕ охраны
 *      `if (seq !== refreshSeq) return` (последней после последнего await) и ДО
 *      `void reloadExpandedAnchors(`;
 *   2. <DeviceList в DevicesPage и <DeviceGroupRow в DeviceList передают {listGen}
 *      или listGen={listGen} (идентификатор, не литерал и не чужое имя пропа);
 *   3. в Props DeviceList и DeviceGroupRow listGen объявлен ОБЯЗАТЕЛЬНЫМ
 *      (`listGen: number`, без `?` и без значения по умолчанию): необязательный
 *      проп молча возвращает CR-02, svelte-check зелёный.
 */
export function checkListGen(pageSrc, listSrc, rowSrc) {
  const out = [];
  const page = stripComments(pageSrc);
  const incs = [...page.matchAll(LIST_GEN_INC_RE)];
  const refresh = functionBody(page, 'refresh');
  if (incs.length !== 1) {
    out.push({
      code: 'H7',
      file: PAGE_FILE,
      msg: `приращений listGen в DevicesPage ${incs.length}, нужно ровно одно (в refresh(), после охраны поколения)`,
    });
  } else if (refresh === null) {
    out.push({
      code: 'H7',
      file: PAGE_FILE,
      msg: 'нет функции refresh() (порядок listGen не проверяется)',
    });
  } else {
    const inc = [...refresh.matchAll(LIST_GEN_INC_RE)];
    if (inc.length !== 1) {
      out.push({ code: 'H7', file: PAGE_FILE, msg: 'приращение listGen стоит не в refresh()' });
    } else {
      const incAt = inc[0].index;
      const lastAwait = lastIndexOfRe(refresh, /\bawait\b/);
      const from = lastAwait === -1 ? 0 : lastAwait;
      const guardRel = refresh.slice(from).search(REFRESH_GUARD_RE);
      const guardAt = guardRel === -1 ? -1 : from + guardRel;
      const reloadAt = refresh.search(/\bvoid\s+reloadExpandedAnchors\s*\(/);
      if (guardAt === -1) {
        out.push({
          code: 'H7',
          file: PAGE_FILE,
          msg: 'refresh(): нет охраны поколения после await (порядок listGen не проверяется)',
        });
      } else if (incAt < guardAt) {
        out.push({
          code: 'H7',
          file: PAGE_FILE,
          msg: 'refresh(): listGen поднимается ДО охраны поколения (устаревший ответ перезапустит загрузку детей)',
        });
      }
      if (reloadAt === -1) {
        out.push({
          code: 'H7',
          file: PAGE_FILE,
          msg: 'refresh(): нет `void reloadExpandedAnchors(`',
        });
      } else if (incAt > reloadAt) {
        out.push({
          code: 'H7',
          file: PAGE_FILE,
          msg: 'refresh(): listGen поднимается ПОСЛЕ `void reloadExpandedAnchors(` (порядок: охрана, присваивания, listGen, перечитывание якорей)',
        });
      }
    }
  }
  const wiring = (src, tagName, file) => {
    const text = stripComments(src);
    const tags = [];
    let from = 0;
    for (;;) {
      const at = text.indexOf(`<${tagName}`, from);
      if (at === -1) break;
      const next = text[at + tagName.length + 1];
      from = at + 1;
      if (next !== undefined && /[\w.]/.test(next)) continue;
      const end = tagEnd(text, at);
      tags.push(end === -1 ? text.slice(at) : text.slice(at, end + 1));
    }
    if (!tags.length) {
      out.push({
        code: 'H7',
        file,
        msg: `нет тега <${tagName}> (проводка listGen не проверяется)`,
      });
    }
    for (const tag of tags) {
      // Допустимы ровно две формы, обе с ИДЕНТИФИКАТОРОМ listGen в значении:
      // `{listGen}` (после пробельного символа, иначе `other={listGen}` ложно
      // пройдёт) и `listGen={listGen}`. Литерал `listGen={0}` замораживает ключ
      // кеша `idsKey@listGen` и молча возвращает CR-02.
      if (!/(?<=\s)(?:\{\s*listGen\s*\}|listGen\s*=\s*\{\s*listGen\s*\})(?![\w$])/.test(tag)) {
        out.push({
          code: 'H7',
          file,
          msg: `<${tagName} не получает идентификатор listGen (литерал или другое имя пропа замораживает ключ кеша idsKey@listGen): цепочка номера поколения разорвана`,
        });
      }
    }
  };
  wiring(pageSrc, 'DeviceList', PAGE_FILE);
  wiring(listSrc, 'DeviceGroupRow', LIST_FILE);
  for (const [src, file] of [
    [listSrc, LIST_FILE],
    [rowSrc, GROUP_ROW_FILE],
  ]) {
    const text = stripComments(src);
    if (!/\blistGen\s*:\s*number\b/.test(text)) {
      out.push({ code: 'H7', file, msg: 'в Props нет обязательного `listGen: number`' });
    }
    // Полная запись атрибута `listGen={listGen}` (проводка в шаблоне) не есть
    // значение по умолчанию: её вырезаем до проверки, иначе законная форма краснела.
    // Литерал `listGen={0}` и `listGen = 0` в деструктуризации остаются красными.
    const propsText = text.replace(/\blistGen\s*=\s*\{\s*listGen\s*\}/g, '');
    if (/\blistGen\s*\?/.test(propsText) || /\blistGen\s*=(?!=)/.test(propsText)) {
      out.push({
        code: 'H7',
        file,
        msg: 'listGen необязателен или имеет значение по умолчанию (молча возвращает CR-02)',
      });
    }
  }
  return out;
}

/**
 * Правило [H6]: `loadAnchorMembers()` идёт по той же границе «сервер → UI», что и
 * `reloadExpandedAnchors()`, и тоже может вернуться не в порядке запросов
 * (T-41.1-G29-01). Поколение снимается ДО первого await; каждая запись состояния
 * ПОСЛЕ await — и успешная, и `failAnchor(` в catch — должна стоять за охраной.
 * Предшествующая await оптимистичная запись `{ loading: true }` охраны не требует.
 */
function checkAnchorMembersLoad(text) {
  const out = [];
  const body = functionBody(text, 'loadAnchorMembers');
  if (body === null) {
    out.push({ code: 'H6', msg: 'нет функции loadAnchorMembers()' });
    return out;
  }
  const gen = /\bconst\s+myGen\s*=\s*bumpAnchorLoad\s*\(/.exec(body);
  const seq = /\bconst\s+aSeq\s*=\s*anchorsSeq\b/.exec(body);
  const firstAwait = body.search(/\bawait\b/);
  if (!gen) {
    out.push({ code: 'H6', msg: 'loadAnchorMembers(): нет `const myGen = bumpAnchorLoad(`' });
  } else if (firstAwait !== -1 && gen.index > firstAwait) {
    out.push({ code: 'H6', msg: 'loadAnchorMembers(): myGen берётся ПОСЛЕ первого await' });
  }
  if (!seq) {
    out.push({ code: 'H6', msg: 'loadAnchorMembers(): нет `const aSeq = anchorsSeq`' });
  } else if (firstAwait !== -1 && seq.index > firstAwait) {
    out.push({ code: 'H6', msg: 'loadAnchorMembers(): aSeq берётся ПОСЛЕ первого await' });
  }
  // Охрана обязана сверять все три условия: своё поколение якоря, поколение
  // перечитывания и то, что якорь всё ещё раскрыт. Ослабление любого — красное.
  const currentFn = /\bconst\s+current\s*=\s*\(\s*\)\s*=>([\s\S]*?);/.exec(body);
  if (!currentFn) {
    out.push({ code: 'H6', msg: 'loadAnchorMembers(): нет охраны `const current = () =>`' });
  } else {
    const c = currentFn[1];
    if (!/anchorLoadSeq\.get\(\s*groupId\s*\)\s*===\s*myGen/.test(c)) {
      out.push({ code: 'H6', msg: 'current(): не сверяет anchorLoadSeq.get(groupId) === myGen' });
    }
    if (!/\baSeq\s*===\s*anchorsSeq\b/.test(c)) {
      out.push({ code: 'H6', msg: 'current(): не сверяет aSeq === anchorsSeq' });
    }
    if (!/expandedAnchors\.has\(\s*groupId\s*\)/.test(c)) {
      out.push({ code: 'H6', msg: 'current(): не сверяет expandedAnchors.has(groupId)' });
    }
  }
  // Порядок: после первого await каждая запись состояния идёт за охраной.
  if (firstAwait !== -1) {
    const tail = body.slice(firstAwait);
    const guardRe = /if\s*\(\s*!\s*current\s*\(\s*\)\s*\)\s*return\b/g;
    const guards = [...tail.matchAll(guardRe)].map((m) => m.index);
    const writes = [
      ...[...tail.matchAll(/(?<![\w.])anchorMembers\s*=(?!=)/g)].map((m) => m.index),
      ...[...tail.matchAll(/\bfailAnchor\s*\(/g)].map((m) => m.index),
    ];
    // Каждой записи нужна СВОЯ охрана: ближайшая слева, без другой записи между
    // ними. Иначе страж успешной ветки «прикрыл» бы и голый failAnchor( в catch.
    writes.sort((a, b) => a - b);
    for (const w of writes) {
      const nearestGuard = Math.max(-1, ...guards.filter((g) => g < w));
      const writeBetween = writes.some((o) => o > nearestGuard && o < w);
      if (nearestGuard === -1 || writeBetween) {
        out.push({
          code: 'H6',
          msg: 'loadAnchorMembers(): запись состояния после await не прикрыта своей `if (!current()) return`',
        });
        break;
      }
    }
    if (writes.length === 0) {
      out.push({
        code: 'H6',
        msg: 'loadAnchorMembers(): после await нет записи состояния — правило стало вакуумным',
      });
    }
  }
  return out;
}

/** Правило [B]: DeviceList.svelte. */
export function checkList(src) {
  const text = stripComments(src);
  const out = [];
  if (!/\bfooterText\s*\(/.test(text)) {
    out.push({ code: 'B', msg: 'подпись футера не берётся из footerText(' });
  }
  const lit = /Показано /.exec(text);
  if (lit) {
    out.push({
      code: 'B',
      msg: `строка ${lineOf(text, lit.index)}: собственный литерал «Показано » вне footerText`,
    });
  }
  return out;
}

/** Конец открывающего тега `<Name ...>` (первый `>` вне фигурных скобок); -1 — не нашлось. */
function tagEnd(text, from) {
  let depth = 0;
  let quote = null;
  for (let i = from; i < text.length; i++) {
    const c = text[i];
    if (quote) {
      if (c === '\\') i++;
      else if (c === quote) quote = null;
      continue;
    }
    if (depth > 0 && (c === '"' || c === "'" || c === '`')) {
      quote = c;
      continue;
    }
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0 && text[i - 1] !== '=') return i;
  }
  return -1;
}

/** Правило [G]: DeviceGroupRow.svelte (кеш детей псевдогруппы привязан к составу group.ids). */
export function checkGroupRow(src) {
  const text = stripComments(src);
  const out = [];
  const imp = /import\s*\{([^}]*)\}\s*from\s*['"]\.\/deviceListMode['"]/.exec(text);
  const names = imp ? imp[1].split(',').map((n) => n.trim()) : [];
  for (const need of ['groupIdsKey', 'visibleChildren']) {
    if (!names.includes(need)) {
      out.push({ code: 'G1', msg: `нет импорта ${need} из './deviceListMode'` });
    }
  }
  const raw = /\{#each\s+children\b/.exec(text);
  if (raw) {
    out.push({
      code: 'G2',
      msg: `строка ${lineOf(text, raw.index)}: {#each} по сырому кешу children; рисовать надо visibleChildren(children, group.ids)`,
    });
  }
  let keyedEffect = false;
  const effRe = /\$effect\s*\(\s*\(\s*\)\s*=>\s*\{/g;
  let m;
  while ((m = effRe.exec(text)) !== null) {
    const open = m.index + m[0].length - 1;
    const close = readBalanced(text, open, '{', '}');
    if (close === -1) continue;
    const body = text.slice(open, close + 1);
    if (
      /\bexpanded\b/.test(body) &&
      /\bcacheKey\b/.test(body) &&
      !/children\s*===\s*null/.test(body)
    ) {
      keyedEffect = true;
    }
  }
  if (!keyedEffect) {
    out.push({
      code: 'G3',
      msg: 'нет $effect, читающего expanded и cacheKey без условия children === null (кеш детей должен быть привязан к ключу кеша: состав + номер поколения)',
    });
  }
  // G5: ключ кеша = состав + поколение списка.
  const keyDecl = /\b(?:const|let)\s+cacheKey\s*=\s*\$derived\(\s*`([^`]*)`\s*\)/.exec(text);
  if (!keyDecl || !/\bidsKey\b/.test(keyDecl[1]) || !/\blistGen\b/.test(keyDecl[1])) {
    out.push({
      code: 'G5',
      msg: 'нет объявления cacheKey = $derived(`...idsKey...listGen...`): ключ кеша детей обязан содержать номер поколения списка',
    });
  }
  if (!/\bloadChildren\(\s*cacheKey\s*\)/.test(text)) {
    out.push({ code: 'G5', msg: 'loadChildren( не вызывается с cacheKey' });
  }
  // G6: второго пути инвалидации в обработчиках нет (функция не найдена = нарушение).
  for (const fn of ['handleEdit', 'handleDelete']) {
    const body = functionBody(text, fn);
    if (body === null) {
      out.push({ code: 'G6', msg: `функция ${fn} не найдена` });
    } else if (/\bloadedKey\b/.test(body)) {
      out.push({
        code: 'G6',
        msg: `${fn} трогает loadedKey: номер поколения — единственный владелец инвалидации кеша детей`,
      });
    }
  }
  // G7: подлежащее первого {#each — результат visibleChildren(...).
  const eachM = /\{#each\s+([A-Za-z_$][\w$]*)\b/.exec(text);
  if (!eachM) {
    out.push({ code: 'G7', msg: 'нет {#each} по детям свёртки' });
  } else if (
    !new RegExp(
      `\\b(?:const|let)\\s+${eachM[1].replace(/\$/g, '\\$')}\\s*=\\s*\\$derived\\(\\s*visibleChildren\\(`,
    ).test(text)
  ) {
    out.push({
      code: 'G7',
      msg: `{#each ${eachM[1]}}: подлежащее не объявлено как $derived(visibleChildren(...), дети рисуются не из отфильтрованного списка`,
    });
  }
  const rowAt = text.indexOf('<DeviceListRow');
  if (rowAt === -1) {
    out.push({ code: 'G4', msg: 'нет вложенной строки <DeviceListRow>' });
  } else {
    const end = tagEnd(text, rowAt);
    const tag = end === -1 ? text.slice(rowAt) : text.slice(rowAt, end + 1);
    for (const attr of ['onDelete', 'onEdit']) {
      if (!new RegExp(`\\b${attr}\\s*=`).test(tag)) {
        out.push({ code: 'G4', msg: `у <DeviceListRow> нет атрибута ${attr}=` });
      }
    }
  }
  return out;
}

/** Правило [D]: opt-in флаги не встречаются в других файлах (обход, не список). */
export function checkFlagHomes(files) {
  const out = [];
  for (const [rel, src] of files) {
    if (!rel.endsWith('.svelte') && !rel.endsWith('.ts')) continue;
    const text = stripComments(src);
    if (!FLAG_HOMES.has(rel)) {
      const m = FLAG_RE.exec(text);
      if (m) {
        out.push({
          code: 'D',
          file: rel,
          msg: `строка ${lineOf(text, m.index)}: opt-in флаг ${m[0]} вне deviceListMode.ts; ставит его только DevicesPage через deviceListFlags`,
        });
      }
    }
    if (!MEMBER_HOMES.has(rel)) {
      const m = MEMBER_RE.exec(text);
      if (m) {
        out.push({
          code: 'D',
          file: rel,
          msg: `строка ${lineOf(text, m.index)}: member_of_group_id ставит только DevicesPage (раскрытие якоря)`,
        });
      }
    }
  }
  return out;
}

export function scanStructural(files, opts = {}) {
  const pageMandatory = opts.pageMandatory ?? PAGE_MANDATORY;
  const minScanned = opts.minScanned ?? MIN_SCANNED_FILES;
  const violations = [];
  const notes = [];
  if (files.size < minScanned) {
    violations.push({
      code: 'E',
      file: 'src',
      msg: `просмотрено ${files.size} файлов, ожидается не меньше ${minScanned}: гейт слеп (изменилась структура каталогов?)`,
    });
  }
  const moduleSrc = files.get(MODULE_FILE);
  if (moduleSrc === undefined) {
    violations.push({ code: 'C', file: MODULE_FILE, msg: 'файл модуля не найден' });
  } else if (/^\s*import\s+(?!type\b)/m.test(stripComments(moduleSrc))) {
    violations.push({
      code: 'C',
      file: MODULE_FILE,
      msg: 'модуль обязан быть без import (кроме import type)',
    });
  }

  for (const [rel, check, label] of [
    [PAGE_FILE, checkPage, 'A'],
    [LIST_FILE, checkList, 'B'],
  ]) {
    const src = files.get(rel);
    if (src === undefined) {
      if (pageMandatory)
        violations.push({ code: label, file: rel, msg: 'обязательный файл не найден' });
      else notes.push(`SKIP [${label}] ${rel}: файла нет`);
      continue;
    }
    const used = /\b(?:deviceListFlags|shouldHideAnchorMembers|footerText)\s*\(/.test(
      stripComments(src),
    );
    if (!used && !pageMandatory) {
      notes.push(`SKIP [${label}] ${rel}: модуль deviceListMode ещё не подключён`);
      continue;
    }
    for (const v of check(src)) violations.push({ ...v, file: rel });
    if (rel === PAGE_FILE && (pageMandatory || used)) {
      for (const v of checkPageRace(src)) violations.push({ ...v, file: rel });
    }
  }
  // [G]: файл обязателен всегда (SKIP-able файл молчал бы вечно).
  const rowSrc = files.get(GROUP_ROW_FILE);
  if (rowSrc === undefined) {
    violations.push({ code: 'G', file: GROUP_ROW_FILE, msg: 'обязательный файл не найден' });
  } else {
    for (const v of checkGroupRow(rowSrc)) violations.push({ ...v, file: GROUP_ROW_FILE });
  }
  // [H7]: порядок и проводка listGen по трём файлам (все обязательны, как и [G]).
  const pageSrcH7 = files.get(PAGE_FILE);
  const listSrcH7 = files.get(LIST_FILE);
  const pageUsesModule =
    pageSrcH7 !== undefined &&
    /\b(?:deviceListFlags|shouldHideAnchorMembers|footerText)\s*\(/.test(stripComments(pageSrcH7));
  if (
    pageSrcH7 !== undefined &&
    listSrcH7 !== undefined &&
    rowSrc !== undefined &&
    (pageMandatory || pageUsesModule)
  ) {
    for (const v of checkListGen(pageSrcH7, listSrcH7, rowSrc)) violations.push(v);
  }
  for (const v of checkFlagHomes(files)) violations.push(v);
  return { violations, notes };
}

function walkSrc(srcDir, out = []) {
  for (const e of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const p = path.join(srcDir, e.name);
    if (e.isDirectory()) {
      if (e.name === 'node_modules' || e.name === 'dist') continue;
      walkSrc(p, out);
    } else if (e.name.endsWith('.svelte') || e.name.endsWith('.ts')) out.push(p);
  }
  return out;
}

function readTree(root) {
  const srcDir = path.join(root, 'src');
  if (!fs.existsSync(srcDir)) return null;
  const files = new Map();
  for (const abs of walkSrc(srcDir)) {
    files.set(path.relative(root, abs).split(path.sep).join('/'), fs.readFileSync(abs, 'utf8'));
  }
  return files;
}

function formatViolation(v) {
  return `[${v.code}] ${v.file ? `${v.file}: ` : ''}${v.msg}`;
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

/** Замена с проверкой, что якорь встречается ровно один раз. */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

const SOURCE_MUTANTS = [
  {
    name: 'скрытие без !searchActive',
    from: '!ctx.searchActive && ',
    to: '',
  },
  {
    name: 'скрытие без statusFilter === null',
    from: ' && ctx.statusFilter === null',
    to: '',
  },
  {
    name: 'скрытие без focusId === null (регресс W3)',
    from: ' && ctx.focusId === null',
    to: '',
  },
  {
    name: "'grouped' без exclude_anchors",
    from: `    case 'grouped':
      return {
        hide_anchor_members: shouldHideAnchorMembers(ctx),
        anchors_only: false,
        exclude_anchors: true,`,
    to: `    case 'grouped':
      return {
        hide_anchor_members: shouldHideAnchorMembers(ctx),
        anchors_only: false,
        exclude_anchors: false,`,
  },
  {
    name: "'flat' скрывает всегда",
    from: `    case 'flat':
      return {
        hide_anchor_members: shouldHideAnchorMembers(ctx),`,
    to: `    case 'flat':
      return {
        hide_anchor_members: true,`,
  },
  {
    name: "'grouped': флаги переставлены (anchors_only вместо exclude_anchors)",
    from: `        anchors_only: false,
        exclude_anchors: true,`,
    to: `        anchors_only: true,
        exclude_anchors: false,`,
  },
  {
    name: "'anchors' без anchors_only",
    from: 'return { hide_anchor_members: false, anchors_only: true, exclude_anchors: false };',
    to: 'return { hide_anchor_members: false, anchors_only: false, exclude_anchors: false };',
  },
  {
    name: "'members' с hide_anchor_members",
    from: `    case 'members':
      return { hide_anchor_members: false, anchors_only: false, exclude_anchors: false };`,
    to: `    case 'members':
      return { hide_anchor_members: true, anchors_only: false, exclude_anchors: false };`,
  },
  {
    name: 'хвост футера без условия hideActive',
    from: 'input.hideActive && input.hidden > 0',
    to: 'input.hidden > 0',
  },
  {
    name: 'хвост футера без условия hidden > 0',
    from: 'input.hideActive && input.hidden > 0',
    to: 'input.hideActive',
  },
  {
    name: 'хвост футера: иная формулировка',
    from: '} в составе групп)`',
    to: '} в группах)`',
  },
  {
    name: 'футер: shown и total переставлены',
    from: '`Показано ${input.shown} из ${input.total}`',
    to: '`Показано ${input.total} из ${input.shown}`',
  },
  {
    name: 'visibleChildren возвращает children без фильтра (до-фиксовое поведение)',
    from: 'return children.filter((c) => inGroup.has(c.id));',
    to: 'return children;',
  },
  {
    name: 'visibleChildren: фильтр инвертирован',
    from: 'inGroup.has(c.id)',
    to: '!inGroup.has(c.id)',
  },
  {
    name: 'groupIdsKey без сортировки',
    from: '[...ids].sort((a, b) => a - b).join',
    to: '[...ids].join',
  },
  {
    name: 'groupIdsKey без числового компаратора (лексикографически)',
    from: '.sort((a, b) => a - b)',
    to: '.sort()',
  },
  {
    name: 'groupIdsKey с join без разделителя',
    from: ".join(',')",
    to: ".join('')",
  },
  {
    name: 'хвост футера показывает не K, а shown',
    from: '(+${input.hidden} в составе',
    to: '(+${input.shown} в составе',
  },
];

const CLEAN_PAGE = `<script lang="ts">
  import { deviceListFlags, shouldHideAnchorMembers } from './deviceListMode';
  const listCtx = $derived({ searchActive, statusFilter, focusId: highlightId });
  const baseFilter = $derived({
    type_id: 1,
    ...deviceListFlags(grouped ? 'grouped' : 'flat', listCtx),
  });
  const hideActive = $derived(shouldHideAnchorMembers(listCtx));
  async function a() {
    await devices.list({ ...baseFilter, ...deviceListFlags('anchors', listCtx) }, p);
  }
  async function m(groupId) {
    await devices.list({ type_id: null, ...deviceListFlags('members', listCtx), member_of_group_id: groupId }, p);
  }
</script>
<DeviceList {listGen} {anchorItems} {hiddenMemberCount} {hideActive} />
`.replace(
  '</script>',
  `  let refreshSeq = 0;
  let anchorsSeq = 0;
  async function fetchList() {
    const resp = await devices.fetchAll(p);
    return { items: resp.items, groups: [], total: resp.total, anchorItems: [], hiddenMemberCount: 0 };
  }
  async function refresh() {
    const seq = ++refreshSeq;
    loading = true;
    try {
      const snap = await fetchList();
      if (seq !== refreshSeq) return;
      items = snap.items;
      groups = snap.groups;
      total = snap.total;
      anchorItems = snap.anchorItems;
      hiddenMemberCount = snap.hiddenMemberCount;
      listGen += 1;
      void reloadExpandedAnchors();
    } catch (e) {
      if (seq !== refreshSeq) return;
      pushToast('error', 'x');
    } finally {
      if (seq === refreshSeq) loading = false;
    }
  }
  function bumpAnchorLoad(groupId) {
    const next = (anchorLoadSeq.get(groupId) ?? 0) + 1;
    anchorLoadSeq.set(groupId, next);
    return next;
  }
  async function loadAnchorMembers(groupId) {
    const myGen = bumpAnchorLoad(groupId);
    const aSeq = anchorsSeq;
    const current = () =>
      anchorLoadSeq.get(groupId) === myGen && aSeq === anchorsSeq && expandedAnchors.has(groupId);
    anchorMembers = { ...anchorMembers, [groupId]: { loading: true, items: [], total: 0 } };
    try {
      const state = await fetchAnchorMembers(groupId);
      if (!current()) return;
      anchorMembers = { ...anchorMembers, [groupId]: state };
    } catch {
      if (!current()) return;
      failAnchor([groupId]);
    }
  }
  async function reloadExpandedAnchors() {
    const aSeq = ++anchorsSeq;
    for (const id of ids) bumpAnchorLoad(id);
    const results = await Promise.all(ids.map((id) => fetchAnchorMembers(id)));
    if (aSeq !== anchorsSeq) return;
    anchorMembers = {};
    failAnchor([]);
  }
</script>`,
);

// Фикстура страницы с видами в тернарнике не годится для проверки «все четыре вида»:
// ниже литералы разнесены по отдельным вызовам.
const CLEAN_PAGE_FULL = CLEAN_PAGE.replace(
  "deviceListFlags(grouped ? 'grouped' : 'flat', listCtx)",
  "deviceListFlags('grouped', listCtx),\n    ...deviceListFlags('flat', listCtx)",
);

const CLEAN_LIST = `<script lang="ts">
  import { footerText } from './deviceListMode';
  interface Props {
    listGen: number;
    items: unknown[];
  }
  const { listGen, items }: Props = $props();
</script>
<DeviceGroupRow {group} {listGen} />
<span>{footerText({ shown: items.length, total, hidden: hiddenMemberCount, hideActive })}</span>
`;

const CLEAN_GROUP_ROW = `<script lang="ts">
  import { groupIdsKey, visibleChildren } from './deviceListMode';
  interface Props {
    listGen: number;
  }
  const { listGen }: Props = $props();
  const idsKey = $derived(groupIdsKey(group.ids));
  const cacheKey = $derived(\`\${idsKey}@\${listGen}\`);
  const shownChildren = $derived(visibleChildren(children ?? [], group.ids));
  $effect(() => {
    if (expanded && loadedKey !== cacheKey && loadingKey !== cacheKey) loadChildren(cacheKey);
  });
  function handleEdit(d) {
    onEdit(d);
  }
  function handleDelete() {
    onDelete();
  }
</script>
{#each shownChildren as child, i (child.id)}
  <DeviceListRow device={child} onEdit={handleEdit} onDelete={handleDelete} isLastInGroup={i === shownChildren.length - 1} />
{/each}
`;

function fixtureMap(extra = {}) {
  const m = new Map([
    [MODULE_FILE, 'export function x() { return 1; }\n'],
    [PAGE_FILE, CLEAN_PAGE_FULL],
    [LIST_FILE, CLEAN_LIST],
    [GROUP_ROW_FILE, CLEAN_GROUP_ROW],
    [
      'src/bindings.ts',
      'export type F = { hide_anchor_members?: boolean; member_of_group_id?: number };\n',
    ],
    ['src/features/acts/Picker.svelte', '<script lang="ts">const f = { type_id: 1 };</script>\n'],
  ]);
  for (const [k, v] of Object.entries(extra)) {
    if (v === null) m.delete(k);
    else m.set(k, v);
  }
  return m;
}

const SELFTEST_OPTS = { minScanned: SELFTEST_MIN_SCANNED_FILES, pageMandatory: true };

/** Структурные мутанты: у каждого «до» зелёная, «после» красная с кодами expect. */
function structuralMutants() {
  const page = (from, to, label) => mutate(CLEAN_PAGE_FULL, from, to, label);
  const list = (from, to, label) => mutate(CLEAN_LIST, from, to, label);
  const row = (from, to, label) => mutate(CLEAN_GROUP_ROW, from, to, label);
  const EFFECT_LINE =
    'if (expanded && loadedKey !== cacheKey && loadingKey !== cacheKey) loadChildren(cacheKey);';
  return [
    {
      name: "[A] нет вызова deviceListFlags('anchors'",
      files: fixtureMap({
        [PAGE_FILE]: page("...deviceListFlags('anchors', listCtx)", '...{}', 'anchors'),
      }),
      expect: ['A'],
    },
    {
      name: "[A] нет вызова deviceListFlags('members'",
      files: fixtureMap({
        [PAGE_FILE]: page("...deviceListFlags('members', listCtx), ", '', 'members'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] из контекста убран focusId',
      files: fixtureMap({
        [PAGE_FILE]: page(', focusId: highlightId', '', 'focus'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] из контекста убран searchActive',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '{ searchActive, statusFilter, focusId: highlightId }',
          '{ statusFilter, focusId: highlightId }',
          'search',
        ),
      }),
      expect: ['A'],
    },
    {
      name: '[A] контекст не разрешается в литерал',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'const listCtx = $derived({ searchActive, statusFilter, focusId: highlightId });',
          'const listCtx = makeCtx();',
          'ctxlit',
        ),
      }),
      expect: ['A'],
    },
    {
      name: '[A] собственный hide_anchor_members вне вызова',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '    type_id: 1,\n',
          '    type_id: 1,\n    hide_anchor_members: true,\n',
          'own',
        ),
      }),
      expect: ['A'],
    },
    {
      name: '[A] hideActive не через shouldHideAnchorMembers',
      files: fixtureMap({
        [PAGE_FILE]: page('shouldHideAnchorMembers(listCtx)', '!searchActive', 'hideActive'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] контекст shouldHideAnchorMembers без focusId',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'shouldHideAnchorMembers(listCtx)',
          'shouldHideAnchorMembers({ searchActive, statusFilter })',
          'hideCtx',
        ),
      }),
      expect: ['A'],
    },
    {
      name: '[A] нет member_of_group_id',
      files: fixtureMap({
        [PAGE_FILE]: page(', member_of_group_id: groupId', '', 'member'),
      }),
      expect: ['A'],
    },
    {
      name: '[A] DeviceList не получает hideActive',
      files: fixtureMap({ [PAGE_FILE]: page(' {hideActive} />', ' />', 'hideActiveProp') }),
      expect: ['A'],
    },
    {
      name: '[A] DeviceList не получает hiddenMemberCount',
      files: fixtureMap({ [PAGE_FILE]: page(' {hiddenMemberCount}', '', 'hiddenProp') }),
      expect: ['A'],
    },
    {
      name: '[B] футер со своим литералом «Показано »',
      files: fixtureMap({
        [LIST_FILE]: list(
          '<span>{footerText({ shown: items.length, total, hidden: hiddenMemberCount, hideActive })}</span>',
          '<span>Показано {items.length} из {total}</span>',
          'literal',
        ),
      }),
      expect: ['B'],
    },
    {
      name: '[D] пикер акта поставил hide_anchor_members',
      files: fixtureMap({
        'src/features/acts/Picker.svelte':
          '<script lang="ts">const f = { type_id: 1, hide_anchor_members: true };</script>\n',
      }),
      expect: ['D'],
    },
    {
      name: '[D] новый файл поставил anchors_only',
      files: fixtureMap({
        'src/features/new/Brand.svelte':
          '<script lang="ts">const f = { anchors_only: true };</script>\n',
      }),
      expect: ['D'],
    },
    {
      name: '[D] новый файл поставил member_of_group_id',
      files: fixtureMap({
        'src/features/new/Brand.ts': 'export const f = { member_of_group_id: 3 };\n',
      }),
      expect: ['D'],
    },
    {
      name: '[A] страницы нет (обязательный файл)',
      files: fixtureMap({ [PAGE_FILE]: null }),
      expect: ['A'],
    },
    {
      name: '[G1] убран импорт из deviceListMode',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          "  import { groupIdsKey, visibleChildren } from './deviceListMode';\n",
          '',
          'g1-none',
        ),
      }),
      expect: ['G1'],
    },
    {
      name: '[G1] импортирован только groupIdsKey',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row('{ groupIdsKey, visibleChildren }', '{ groupIdsKey }', 'g1-part'),
      }),
      expect: ['G1'],
    },
    {
      name: '[G2] {#each} по сырому children',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row('{#each shownChildren as', '{#each children as', 'g2'),
      }),
      expect: ['G2'],
    },
    {
      name: '[G3] эффект на children === null вместо ключа состава',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          EFFECT_LINE,
          'if (expanded && children === null) loadChildren(group.ids);',
          'g3-null',
        ),
      }),
      expect: ['G3'],
    },
    {
      name: '[G3] перестановка: cacheKey заменён на group.ids и добавлено children === null',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          EFFECT_LINE,
          'if (expanded && loadedKey !== group.ids && children === null) loadChildren(group.ids);',
          'g3-perm',
        ),
      }),
      expect: ['G3'],
    },
    {
      name: '[G3] ключ состава есть, но в эффект добавлено children === null',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          EFFECT_LINE,
          'if (expanded && children === null && loadedKey !== cacheKey) loadChildren(cacheKey);',
          'g3-both',
        ),
      }),
      expect: ['G3'],
    },
    {
      name: '[G5] ключ кеша без listGen',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row('@${listGen}', '', 'g5-nogen'),
      }),
      expect: ['G5'],
    },
    {
      name: '[G5] loadChildren зовётся с idsKey, а не с cacheKey',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row('loadChildren(cacheKey);', 'loadChildren(idsKey);', 'g5-call'),
      }),
      expect: ['G5'],
    },
    {
      name: '[G6] handleEdit помечает кеш устаревшим',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          'function handleEdit(d) {\n    onEdit(d);',
          'function handleEdit(d) {\n    loadedKey = null;\n    onEdit(d);',
          'g6-edit',
        ),
      }),
      expect: ['G6'],
    },
    {
      name: '[G6] handleDelete помечает кеш устаревшим',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          'function handleDelete() {\n    onDelete();',
          'function handleDelete() {\n    loadedKey = null;\n    onDelete();',
          'g6-delete',
        ),
      }),
      expect: ['G6'],
    },
    {
      name: '[G6] handleEdit отсутствует',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          'function handleEdit(d) {',
          'function handleEditX(d) {',
          'g6-missing',
        ),
      }),
      expect: ['G6'],
    },
    {
      name: '[G7] shownChildren не из visibleChildren',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(
          'const shownChildren = $derived(visibleChildren(children ?? [], group.ids));',
          'const shownChildren = $derived(children ?? []);',
          'g7',
        ),
      }),
      expect: ['G7'],
    },
    {
      name: '[G4] у <DeviceListRow> убран onDelete',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(' onDelete={handleDelete}', '', 'g4-delete'),
      }),
      expect: ['G4'],
    },
    {
      name: '[G4] у <DeviceListRow> убран onEdit',
      files: fixtureMap({
        [GROUP_ROW_FILE]: row(' onEdit={handleEdit}', '', 'g4-edit'),
      }),
      expect: ['G4'],
    },
    {
      name: '[G] DeviceGroupRow.svelte отсутствует (обязательный файл)',
      files: fixtureMap({ [GROUP_ROW_FILE]: null }),
      expect: ['G'],
    },
    // [H]: защита поколения; охрана обязана стоять ДО присваиваний (перестановка краснит).
    {
      name: '[H2] охрана поколения после await удалена',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'const snap = await fetchList();\n      if (seq !== refreshSeq) return;',
          'const snap = await fetchList();',
          'h2-remove',
        ),
      }),
      expect: ['H2'],
    },
    {
      name: '[H2] охрана перенесена ПОСЛЕ всех присваиваний (перестановка)',
      files: fixtureMap({
        [PAGE_FILE]: mutate(
          page(
            'if (seq !== refreshSeq) return;\n      items = snap.items;',
            'items = snap.items;',
            'h2-move-1',
          ),
          'hiddenMemberCount = snap.hiddenMemberCount;\n',
          'hiddenMemberCount = snap.hiddenMemberCount;\n      if (seq !== refreshSeq) return;\n',
          'h2-move-2',
        ),
      }),
      expect: ['H2'],
    },
    {
      name: '[H2] охрана перенесена между присваиваниями (после первого)',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'if (seq !== refreshSeq) return;\n      items = snap.items;',
          'items = snap.items;\n      if (seq !== refreshSeq) return;',
          'h2-between',
        ),
      }),
      expect: ['H2'],
    },
    {
      name: '[H1] refresh() не берёт номер поколения',
      files: fixtureMap({
        [PAGE_FILE]: page('const seq = ++refreshSeq;', 'const seq = refreshSeq;', 'h1-noinc'),
      }),
      expect: ['H1'],
    },
    {
      name: '[H1] номер поколения увеличивается после await',
      files: fixtureMap({
        [PAGE_FILE]: mutate(
          page('const seq = ++refreshSeq;', 'const seq = refreshSeq;', 'h1-late-1'),
          'const snap = await fetchList();',
          'const snap = await fetchList();\n      refreshSeq++;',
          'h1-late-2',
        ),
      }),
      expect: ['H1'],
    },
    {
      name: '[H1] функция refresh() переименована (правило не вакуумно)',
      files: fixtureMap({
        [PAGE_FILE]: page('async function refresh() {', 'async function reload() {', 'h1-rename'),
      }),
      expect: ['H1'],
    },
    {
      name: '[H3] fetchList() присваивает items',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'const resp = await devices.fetchAll(p);',
          'const resp = await devices.fetchAll(p);\n    items = resp.items;',
          'h3-items',
        ),
      }),
      expect: ['H3'],
    },
    {
      name: '[H3] fetchList() присваивает loading',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'const resp = await devices.fetchAll(p);',
          'const resp = await devices.fetchAll(p);\n    loading = false;',
          'h3-loading',
        ),
      }),
      expect: ['H3'],
    },
    {
      name: '[H4] finally сбрасывает loading без условия',
      files: fixtureMap({
        [PAGE_FILE]: page(
          'if (seq === refreshSeq) loading = false;',
          'loading = false;',
          'h4-finally',
        ),
      }),
      expect: ['H4'],
    },
    {
      name: '[H4] catch без охраны поколения',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '} catch (e) {\n      if (seq !== refreshSeq) return;',
          '} catch (e) {',
          'h4-catch',
        ),
      }),
      expect: ['H4'],
    },
    {
      name: '[H4] охрана catch перенесена после тоста (перестановка)',
      files: fixtureMap({
        [PAGE_FILE]: page(
          "if (seq !== refreshSeq) return;\n      pushToast('error', 'x');",
          "pushToast('error', 'x');\n      if (seq !== refreshSeq) return;",
          'h4-catch-order',
        ),
      }),
      expect: ['H4'],
    },
    {
      name: '[H5] reloadExpandedAnchors без номера поколения',
      files: fixtureMap({
        [PAGE_FILE]: page('const aSeq = ++anchorsSeq;', 'const aSeq = anchorsSeq;', 'h5-noinc'),
      }),
      expect: ['H5'],
    },
    {
      name: '[H5] охрана aSeq удалена',
      files: fixtureMap({
        [PAGE_FILE]: page('    if (aSeq !== anchorsSeq) return;\n', '', 'h5-remove'),
      }),
      expect: ['H5'],
    },
    {
      name: '[H5] охрана aSeq стоит после записи anchorMembers (перестановка)',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '    if (aSeq !== anchorsSeq) return;\n    anchorMembers = {};',
          '    anchorMembers = {};\n    if (aSeq !== anchorsSeq) return;',
          'h5-order',
        ),
      }),
      expect: ['H5'],
    },
    {
      name: '[H6] loadAnchorMembers без поколения якоря',
      files: fixtureMap({
        [PAGE_FILE]: page('    const myGen = bumpAnchorLoad(groupId);\n', '', 'h6-nogen'),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] loadAnchorMembers без снимка anchorsSeq',
      files: fixtureMap({
        [PAGE_FILE]: page('    const aSeq = anchorsSeq;\n', '', 'h6-noaseq'),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] current() перестал сверять поколение якоря',
      files: fixtureMap({
        [PAGE_FILE]: page('anchorLoadSeq.get(groupId) === myGen && ', '', 'h6-current-nogen'),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] current() перестал сверять поколение перечитывания',
      files: fixtureMap({
        [PAGE_FILE]: page('aSeq === anchorsSeq && ', '', 'h6-current-noaseq'),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] current() перестал сверять раскрытость якоря',
      files: fixtureMap({
        [PAGE_FILE]: page(' && expandedAnchors.has(groupId)', '', 'h6-current-noexp'),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] охрана успешной ветки удалена',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '      if (!current()) return;\n      anchorMembers = { ...anchorMembers, [groupId]: state };',
          '      anchorMembers = { ...anchorMembers, [groupId]: state };',
          'h6-guard-remove',
        ),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] охрана стоит ПОСЛЕ записи состава (перестановка)',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '      if (!current()) return;\n      anchorMembers = { ...anchorMembers, [groupId]: state };',
          '      anchorMembers = { ...anchorMembers, [groupId]: state };\n      if (!current()) return;',
          'h6-guard-order',
        ),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] ветка catch вызывает failAnchor без охраны',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '      if (!current()) return;\n      failAnchor([groupId]);',
          '      failAnchor([groupId]);',
          'h6-catch-unguarded',
        ),
      }),
      expect: ['H6'],
    },
    {
      name: '[H6] перечитывание не обесценивает одиночные загрузки',
      files: fixtureMap({
        [PAGE_FILE]: page(
          '    for (const id of ids) bumpAnchorLoad(id);\n',
          '',
          'h6-reload-nobump',
        ),
      }),
      expect: ['H6'],
    },
    {
      name: '[C] в модуле появился import',
      files: fixtureMap({ [MODULE_FILE]: "import { x } from './y';\nexport const a = x;\n" }),
      expect: ['C'],
    },
  ];
}

async function selftest(cases) {
  let tsSource;
  try {
    tsSource = fs.readFileSync(path.join(UI_ROOT, MODULE_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${MODULE_FILE}`);
    process.exit(1);
  }
  // 1. Эталон проходит фикстуру.
  const goodFailures = runCases(await loadModuleFromTs(tsSource, 'good.ts'), cases);
  if (goodFailures.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальный исходник не прошёл фикстуру:\n  - ${goodFailures.join('\n  - ')}`,
    );
    process.exit(1);
  }
  // 2. Мутанты исходника ловятся.
  for (const m of SOURCE_MUTANTS) {
    const mutated = mutate(tsSource, m.from, m.to, m.name);
    const failures = runCases(await loadModuleFromTs(mutated, 'mutant.ts'), cases);
    if (!failures.length) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» прошёл фикстуру: гейт его не ловит`);
      process.exit(1);
    }
  }
  // 3. Структурные правила: чистая карта зелёная, мутанты красные нужным кодом.
  const clean = scanStructural(fixtureMap(), SELFTEST_OPTS);
  if (clean.violations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — чистая фикстура структурных правил красная:\n  - ${clean.violations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  const mutants = structuralMutants();
  for (const m of mutants) {
    const r = scanStructural(m.files, SELFTEST_OPTS);
    const codes = new Set(r.violations.map((v) => v.code));
    for (const code of m.expect) {
      if (!codes.has(code)) {
        console.error(
          `${TAG} SELFTEST FAIL — «${m.name}»: ожидалось нарушение [${code}], получено [${[...codes].join(',') || 'ничего'}]`,
        );
        process.exit(1);
      }
    }
  }
  // 3б. Вакуумная проверка [G]: замороженная до-фиксовая DeviceGroupRow обязана краснеть
  // на [G1], [G2], [G3], а реальная (исправленная) — быть чистой.
  let preFix;
  try {
    preFix = fs.readFileSync(PRE_FIX_GROUP_ROW, 'utf8');
  } catch {
    console.error(
      `${TAG} SELFTEST FAIL — нет замороженной до-фиксовой копии ${path.relative(UI_ROOT, PRE_FIX_GROUP_ROW)}`,
    );
    process.exit(1);
  }
  if (!/children\s*===\s*null/.test(preFix) || preFix.includes('groupIdsKey')) {
    console.error(
      `${TAG} SELFTEST FAIL — замороженная до-фиксовая копия подменена исправленной (в ней должно быть children === null и не должно быть groupIdsKey)`,
    );
    process.exit(1);
  }
  const preFixCodes = new Set(checkGroupRow(preFix).map((v) => v.code));
  for (const code of ['G1', 'G2', 'G3']) {
    if (!preFixCodes.has(code)) {
      console.error(
        `${TAG} SELFTEST FAIL — до-фиксовая DeviceGroupRow не дала [${code}] (получено [${[...preFixCodes].join(',') || 'ничего'}]): гейт вакуумен`,
      );
      process.exit(1);
    }
  }
  let realRow;
  try {
    realRow = fs.readFileSync(path.join(UI_ROOT, GROUP_ROW_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${GROUP_ROW_FILE}`);
    process.exit(1);
  }
  const realRowViolations = checkGroupRow(realRow);
  if (realRowViolations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальная DeviceGroupRow краснеет:\n  - ${realRowViolations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  // 3в. Мутанты по РЕАЛЬНОМУ DeviceGroupRow.svelte (D-25): якорь ровно один раз,
  // каждая из трёх регрессий CR-02/WR-08 краснит своё правило.
  const realMutants = [
    {
      name: '[G5] ключ кеша детей без listGen (реальный файл)',
      from: 'const cacheKey = $derived(`${idsKey}@${listGen}`);',
      to: 'const cacheKey = $derived(`${idsKey}`);',
      expect: 'G5',
    },
    {
      name: '[G6] handleEdit снова ставит loadedKey = null (реальный файл)',
      from: 'function handleEdit(d: DeviceDto) {\n    onEdit(d);',
      to: 'function handleEdit(d: DeviceDto) {\n    loadedKey = null;\n    onEdit(d);',
      expect: 'G6',
    },
    {
      name: '[G6] handleDelete снова ставит loadedKey = null (реальный файл)',
      from: 'function handleDelete() {\n    onDelete();',
      to: 'function handleDelete() {\n    loadedKey = null;\n    onDelete();',
      expect: 'G6',
    },
    {
      name: '[G7] shownChildren из сырого кеша (реальный файл)',
      from: 'const shownChildren = $derived(visibleChildren(children ?? [], group.ids));',
      to: 'const shownChildren = $derived(children ?? []);',
      expect: 'G7',
    },
  ];
  for (const m of realMutants) {
    const mutated = mutate(realRow, m.from, m.to, m.name);
    const codes = new Set(checkGroupRow(mutated).map((v) => v.code));
    if (!codes.has(m.expect)) {
      console.error(
        `${TAG} SELFTEST FAIL — «${m.name}»: ожидалось [${m.expect}], получено [${[...codes].join(',') || 'ничего'}]`,
      );
      process.exit(1);
    }
  }
  // 3г. Вакуумная проверка [H] (WR-05, D-23): замороженная до-фиксовая DevicesPage
  // (версия до плана 29, `git show a7c25aeb^:...`) обязана дать H1, H2, H5, а реальная
  // страница, чьи мутанты ниже строятся на НАСТОЯЩЕЙ форме refresh(), — ноль нарушений.
  let preFixPage;
  try {
    preFixPage = fs.readFileSync(PRE_FIX_PAGE, 'utf8');
  } catch {
    console.error(
      `${TAG} SELFTEST FAIL — нет замороженной до-фиксовой копии ${path.relative(UI_ROOT, PRE_FIX_PAGE)}`,
    );
    process.exit(1);
  }
  if (/refreshSeq/.test(preFixPage) || !/async\s+function\s+refresh\b/.test(preFixPage)) {
    console.error(
      `${TAG} SELFTEST FAIL — замороженная до-фиксовая DevicesPage подменена исправленной или пуста (в ней не должно быть refreshSeq и должна быть async function refresh)`,
    );
    process.exit(1);
  }
  const preFixPageCodes = new Set(checkPageRace(preFixPage).map((v) => v.code));
  for (const code of ['H1', 'H2', 'H5']) {
    if (!preFixPageCodes.has(code)) {
      console.error(
        `${TAG} SELFTEST FAIL — до-фиксовая DevicesPage не дала [${code}] (получено [${[...preFixPageCodes].join(',') || 'ничего'}]): правило [H] вакуумно на настоящей форме страницы`,
      );
      process.exit(1);
    }
  }
  let realPage;
  try {
    realPage = fs.readFileSync(path.join(UI_ROOT, PAGE_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${PAGE_FILE}`);
    process.exit(1);
  }
  const realPageViolations = checkPageRace(realPage);
  if (realPageViolations.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальная DevicesPage краснеет:\n  - ${realPageViolations.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  // 3д. Мутанты [H] по РЕАЛЬНОМУ DevicesPage.svelte (D-23, D-26): якорь ровно один раз
  // (голая `if (seq !== refreshSeq) return;` встречается дважды и не используется).
  const realPageMutants = [
    {
      name: '[H2] присваивание items раньше охраны (реальный файл)',
      from: '      if (seq !== refreshSeq) return;\n      items = snap.items;\n',
      to: '      items = snap.items;\n      if (seq !== refreshSeq) return;\n',
      expect: 'H2',
    },
    {
      name: '[H1] refresh() не поднимает refreshSeq (реальный файл)',
      from: 'const seq = ++refreshSeq;',
      to: 'const seq = refreshSeq;',
      expect: 'H1',
    },
    {
      name: '[H5] reloadExpandedAnchors() не поднимает anchorsSeq (реальный файл)',
      from: 'const aSeq = ++anchorsSeq;',
      to: 'const aSeq = anchorsSeq;',
      expect: 'H5',
    },
  ];
  for (const m of realPageMutants) {
    const mutated = mutate(realPage, m.from, m.to, m.name);
    const codes = new Set(checkPageRace(mutated).map((v) => v.code));
    if (!codes.has(m.expect)) {
      console.error(
        `${TAG} SELFTEST FAIL — «${m.name}»: ожидалось [${m.expect}], получено [${[...codes].join(',') || 'ничего'}]`,
      );
      process.exit(1);
    }
  }
  // 3е. Правило [H7] (CR-02, D-09) на РЕАЛЬНЫХ файлах: чистая тройка зелёная, пять
  // мутантов красные. Главный мутант — ПЕРЕСТАНОВКА (приращение переносится ДО охраны,
  // а не удаляется): счётный гейт слеп к порядку, `listGen += 1` по-прежнему встречается
  // ровно один раз. Якоря уникальны (D-26): голая охрана встречается дважды и не используется.
  let realList;
  try {
    realList = fs.readFileSync(path.join(UI_ROOT, LIST_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${LIST_FILE}`);
    process.exit(1);
  }
  const realH7 = checkListGen(realPage, realList, realRow);
  if (realH7.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальная проводка listGen краснеет:\n  - ${realH7.map(formatViolation).join('\n  - ')}`,
    );
    process.exit(1);
  }
  const GUARD_ITEMS = '      if (seq !== refreshSeq) return;\n      items = snap.items;\n';
  const INC_AFTER_ASSIGN =
    '      hiddenMemberCount = snap.hiddenMemberCount;\n      listGen += 1;\n      void reloadExpandedAnchors();\n';
  const h7Mutants = [
    {
      name: '[H7] listGen += 1 перенесён ДО охраны поколения (перестановка, реальные файлы)',
      page: (t) =>
        mutate(
          mutate(
            t,
            INC_AFTER_ASSIGN,
            '      hiddenMemberCount = snap.hiddenMemberCount;\n      void reloadExpandedAnchors();\n',
            'h7-reorder-remove',
          ),
          GUARD_ITEMS,
          '      listGen += 1;\n' + GUARD_ITEMS,
          'h7-reorder-insert',
        ),
    },
    {
      name: '[H7] listGen += 1 перенесён ПОСЛЕ void reloadExpandedAnchors( (реальные файлы)',
      page: (t) =>
        mutate(
          t,
          INC_AFTER_ASSIGN,
          '      hiddenMemberCount = snap.hiddenMemberCount;\n      void reloadExpandedAnchors();\n      listGen += 1;\n',
          'h7-after-reload',
        ),
    },
    {
      name: '[H7] <DeviceList в DevicesPage не получает {listGen} (реальные файлы)',
      page: (t) =>
        mutate(t, '    <DeviceList\n      {listGen}\n', '    <DeviceList\n', 'h7-page-wiring'),
    },
    {
      name: '[H7] <DeviceGroupRow в DeviceList не получает {listGen} (реальные файлы)',
      list: (t) =>
        mutate(
          t,
          '        <DeviceGroupRow\n          {group}\n          {listGen}\n',
          '        <DeviceGroupRow\n          {group}\n',
          'h7-list-wiring',
        ),
    },
    {
      name: '[H7] <DeviceList в DevicesPage получает литерал listGen={0} (реальные файлы)',
      wiringMsg: true,
      page: (t) =>
        mutate(
          t,
          '    <DeviceList\n      {listGen}\n',
          '    <DeviceList\n      listGen={0}\n',
          'h7-page-literal',
        ),
    },
    {
      name: '[H7] <DeviceGroupRow в DeviceList получает литерал listGen={0} (реальные файлы)',
      wiringMsg: true,
      list: (t) =>
        mutate(
          t,
          '        <DeviceGroupRow\n          {group}\n          {listGen}\n',
          '        <DeviceGroupRow\n          {group}\n          listGen={0}\n',
          'h7-list-literal',
        ),
    },
    {
      name: '[H7] <DeviceList в DevicesPage получает other={listGen} вместо {listGen} (реальные файлы)',
      wiringMsg: true,
      page: (t) =>
        mutate(
          t,
          '    <DeviceList\n      {listGen}\n',
          '    <DeviceList\n      other={listGen}\n',
          'h7-page-wrong-name',
        ),
    },
    {
      name: '[H7] listGen?: number в Props DeviceGroupRow (реальные файлы)',
      row: (t) => mutate(t, '    listGen: number;\n', '    listGen?: number;\n', 'h7-optional'),
    },
    {
      name: '[H7] значение по умолчанию listGen = 0 в деструктуризации DeviceList (реальные файлы)',
      list: (t) => mutate(t, '    listGen,\n', '    listGen = 0,\n', 'h7-default-value'),
    },
    {
      name: '[H7] второе приращение listGen в catch (реальные файлы)',
      page: (t) =>
        mutate(
          t,
          '    } catch (e: unknown) {\n      if (seq !== refreshSeq) return;\n',
          '    } catch (e: unknown) {\n      if (seq !== refreshSeq) return;\n      listGen += 1;\n',
          'h7-catch-inc',
        ),
    },
  ];
  for (const m of h7Mutants) {
    const found = checkListGen(
      m.page ? m.page(realPage) : realPage,
      m.list ? m.list(realList) : realList,
      m.row ? m.row(realRow) : realRow,
    );
    const codes = new Set(found.map((v) => v.code));
    // wiringMsg: нарушение обязано прийти именно от правила проводки. Литерал
    // listGen={0} в DeviceList краснит ещё и проверка Props (побочно), и без этой
    // привязки мутант остался бы зелёным при полностью ослабленной регулярке проводки.
    if (m.wiringMsg && !found.some((v) => v.msg.includes('не получает идентификатор listGen'))) {
      console.error(
        `${TAG} SELFTEST FAIL — «${m.name}»: нарушение пришло не от правила проводки [H7]`,
      );
      process.exit(1);
    }
    if (!codes.has('H7')) {
      console.error(
        `${TAG} SELFTEST FAIL — «${m.name}»: ожидалось [H7], получено [${[...codes].join(',') || 'ничего'}]`,
      );
      process.exit(1);
    }
  }
  // Зелёные формы: полная запись listGen={listGen} законна. Без этой проверки
  // строгая регулярка могла бы стать слишком узкой и покраснеть на законном.
  const h7Green = [
    {
      name: 'listGen={listGen} у <DeviceList',
      page: (t) =>
        mutate(
          t,
          '    <DeviceList\n      {listGen}\n',
          '    <DeviceList\n      listGen={listGen}\n',
          'h7-page-green-full',
        ),
    },
    {
      name: 'listGen={listGen} у <DeviceGroupRow',
      list: (t) =>
        mutate(
          t,
          '        <DeviceGroupRow\n          {group}\n          {listGen}\n',
          '        <DeviceGroupRow\n          {group}\n          listGen={listGen}\n',
          'h7-list-green-full',
        ),
    },
  ];
  for (const g of h7Green) {
    const bad = checkListGen(
      g.page ? g.page(realPage) : realPage,
      g.list ? g.list(realList) : realList,
      realRow,
    ).filter((v) => v.code === 'H7');
    if (bad.length) {
      console.error(
        `${TAG} SELFTEST FAIL — законная форма «${g.name}» краснит [H7]:\n  - ${bad.map(formatViolation).join('\n  - ')}`,
      );
      process.exit(1);
    }
  }
  // 4. Режим «страница ещё не подключена»: явный SKIP, не молчание и не красное.
  const unconnected = scanStructural(
    fixtureMap({ [PAGE_FILE]: '<script lang="ts">let x = 1;</script>\n' }),
    { ...SELFTEST_OPTS, pageMandatory: false },
  );
  if (
    unconnected.violations.length ||
    !unconnected.notes.some((n) => n.startsWith('SKIP') && n.includes(PAGE_FILE))
  ) {
    console.error(`${TAG} SELFTEST FAIL — неподключённая страница обязана давать явный SKIP`);
    process.exit(1);
  }
  // 5. Страница обязательна: тот же файл в режиме pageMandatory краснит [A], а не молчит.
  const mandatoryUnconnected = scanStructural(
    fixtureMap({ [PAGE_FILE]: '<script lang="ts">let x = 1;</script>\n' }),
    SELFTEST_OPTS,
  );
  if (!mandatoryUnconnected.violations.some((v) => v.code === 'A')) {
    console.error(`${TAG} SELFTEST FAIL — обязательная страница без вызовов не краснит [A]`);
    process.exit(1);
  }
  // 6. Защита от вакуумности: слишком мало файлов — [E].
  const vac = scanStructural(fixtureMap(), { ...SELFTEST_OPTS, minScanned: 1000 });
  if (!vac.violations.some((v) => v.code === 'E')) {
    console.error(`${TAG} SELFTEST FAIL — малое число просмотренных файлов не краснит [E]`);
    process.exit(1);
  }
  console.log(
    `${TAG} selftest OK — эталон проходит ${cases.length} кейсов, ${SOURCE_MUTANTS.length} мутантов исходника и ${mutants.length} структурных мутантов ловятся; неподключённая страница даёт явный SKIP`,
  );
}

// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { impl: path.join(UI_ROOT, MODULE_FILE), root: UI_ROOT, selftest: false };
  for (const a of argv) {
    if (a === '--selftest') args.selftest = true;
    else if (a.startsWith('--impl=')) args.impl = path.resolve(process.cwd(), a.slice(7));
    else if (a.startsWith('--src=')) {
      args.root = path.resolve(process.cwd(), a.slice(6));
      args.impl = path.join(args.root, MODULE_FILE);
    }
  }
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cases = loadFixture();
  if (args.selftest) return selftest(cases);

  let tsSource;
  try {
    tsSource = fs.readFileSync(args.impl, 'utf8');
  } catch {
    console.error(`${TAG} FAIL — не удалось прочитать ${args.impl}`);
    process.exit(1);
  }
  const failures = runCases(await loadModuleFromTs(tsSource, path.basename(args.impl)), cases);
  const files = readTree(args.root);
  if (files === null) {
    console.error(`${TAG} FAIL — в ${args.root} нет каталога src/ (--src принимает КОРЕНЬ копии)`);
    process.exit(1);
  }
  const { violations, notes } = scanStructural(files);
  if (failures.length || violations.length) {
    console.error(`${TAG} FAIL —`);
    for (const f of failures) console.error(`  - [golden] ${f}`);
    for (const v of violations) console.error(`  - ${formatViolation(v)}`);
    process.exit(1);
  }
  for (const n of notes) console.log(`${TAG} ${n}`);
  console.log(
    `${TAG} OK — ${cases.length} кейсов фикстуры, структурные правила по ${files.size} файлам`,
  );
}

main();
