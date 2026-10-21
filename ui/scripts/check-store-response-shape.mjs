#!/usr/bin/env node
// [check-store-response-shape] Постоянный гейт «стор-справочник проверяет форму
// ответа ДО записи в $state» (фаза 41.12, ревью WR-06 + IN-01, коммит 33476797).
//
// Почему он существует: по HTTP отсутствующий `/api/`-маршрут отдаёт index.html с
// кодом 200 (spa_fallback), а ветка `text/html` в `apiCall`
// (`ui/src/lib/api/client.ts`, ~строка 60) приводит ЛЮБУЮ строку к типу ответа
// через `as R`. Корень в клиенте сознательно не исправлен — это структурное
// изменение, записанное рекомендацией в 41.12-VERIFICATION.md. Значит
// единственная действующая защита — локальная охрана в сторе: без неё стор
// получал `'<!doctype html…'`, ставил `state = 'ready'`, и `orderedTypes()`
// делал `[...'<!doctype…'].sort(a.id - b.id)` — список из символов с `undefined`
// в подписях, без состояния `error` и без тоста (запрос формально успешен).
//
// Почему гейт: удаление этой охраны не видит НИЧЕГО — ни `pnpm lint`, ни
// `svelte-check`, ни Rust-тесты (серверный тест требует массив, клиентского
// теста нет и JS-фреймворка тестов в проекте нет). Класс ровно тот, ради
// которого в ui/scripts живут структурные гейты.
//
// Реестр STORES перечисляет сторы-справочники (стор, команда, поле $state,
// обязательные поля DTO, причина существования записи не короче 20 символов).
// Правила (сообщение начинается с кода правила в квадратных скобках):
//   [S1] охраны нет вовсе: в сторе не найдено условие, ссылающееся на параметр
//        продолжения `.then((rows) => …)` (либо самого продолжения нет);
//   [S2] в охране нет проверки массивности (`Array.isArray(` / `instanceof Array`):
//        строка тоже «итерируемая», поэлементная проверка её бы пропустила;
//   [S3] в охране нет ПОЭЛЕМЕНТНОЙ проверки типов: нужен обход (`.every(`/`.some(`)
//        и `typeof <элемент>?.<поле>` по КАЖДОМУ обязательному полю из реестра;
//   [S4] охрана не бросает: в её теле нет `throw` — тихий `return` оставил бы
//        состояние `loading` навсегда и не дал бы тоста из `.catch`;
//   [S5] ПОРЯДОК: запись ответа в `$state` и перевод состояния в `'ready'` обязаны
//        стоять ПОСЛЕ охраны (гейт, слепой к порядку, принял бы охрану после
//        записи за защиту — в проекте это уже случалось со счётным гейтом);
//   [S6] антивакуум реестра: запись, под которую не нашлось файла; причина короче
//        20 символов; путь не `*.svelte.ts`; пустой список полей; и порог
//        MIN_STORES — реестр меньше порога краснит гейт (иначе защита «есть»
//        при пустом реестре);
//   [S7] антивакуум цепочки: команда справочника обязана находиться в api-модуле
//        (`<аксессор>: () => apiCall<…>('<команда>')`), а стор — вызывать этот
//        аксессор. Исчезла команда или вызов — охрана охраняет пустоту;
//   [S8] новый стор-справочник вне реестра: файл `*.svelte.ts` с `$state`, который
//        зовёт `*_list`-команду (прямо через `apiCall` или через аксессор), но в
//        реестре не числится — иначе третий такой стор родится без защиты.
//
// Инвентарь выводится ОБХОДОМ `src/**/*.ts` — список api-модулей и сторов-кандидатов
// нигде не перечислен руками. Комментарии вырезаются общим модулем `_strip.mjs`
// (WR-01: локальных копий stripComments в проекте больше не пишем).
//
// Чего гейт НЕ видит (честно):
//   - семантику охраны в рантайме: что именно бросится на `index.html`, проверяет
//     человек/интеграционный тест, а не grep;
//   - охрану, вынесенную в отдельную функцию-валидатор (`assertRows(rows)`): такая
//     форма краснит S2/S3, и это осознанно — вынос потребует правки реестра, а не
//     молчаливого ослабления правила;
//   - корень дефекта в `apiCall`: гейт держит симптом по месту, а не причину;
//   - прочие (нереестровые) вызовы, ожидающие JSON: S8 ловит только сторы.
//
// Self-test (`--selftest`): мутанты по РЕАЛЬНЫМ исходникам через mutate() (якорь
// обязан встретиться ровно один раз) плюс синтетика; ожидание сверяется ТОЧНЫМ
// множеством кодов.
//
// Zero-dependency: только node:fs/node:path/node:url + ./_strip.mjs.
//
// Usage:
//   node scripts/check-store-response-shape.mjs             # весь репозиторий
//   node scripts/check-store-response-shape.mjs --src=<dir> # копия каталога ui/
//   node scripts/check-store-response-shape.mjs --report    # инвентарь
//   node scripts/check-store-response-shape.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-store-response-shape]';
const SRC_DIR = 'src';

// Сторы-справочники с охраной формы ответа. Порог: меньше двух записей — реестр
// выродился, гейт зелёный по пустоте.
const MIN_STORES = 2;
const MIN_REASON = 20;

const STORES = [
  {
    store: 'src/lib/stores/deviceTypes.svelte.ts',
    command: 'device_types_list',
    stateField: 'items',
    readyField: 'state',
    readyValue: 'ready',
    fields: ['id', 'name'],
    reason:
      'справочник типов устройств кормит фильтр отчётов и пикер групп; index.html вместо JSON дал бы массив символов и state=ready',
  },
  {
    store: 'src/lib/stores/deviceStatuses.svelte.ts',
    command: 'device_statuses_list',
    stateField: 'items',
    readyField: 'state',
    readyValue: 'ready',
    fields: ['id', 'code', 'name', 'sort_order'],
    reason:
      'справочник статусов задаёт подписи и тона бейджей по всему приложению; тот же дефект формы ответа, что и у типов',
  },
];

// Сгенерированный specta-файл: аксессоров и сторов не содержит, в git не лежит.
const GENERATED = new Map([
  [
    'src/bindings.ts',
    'сгенерирован specta (в .gitignore), перезаписывается prebuild; api-аксессоров не содержит',
  ],
]);

// `types: () => apiCall<DeviceTypeDto[]>('device_types_list')` — аксессор api-модуля.
const ACCESSOR_RE =
  /([A-Za-z_$][\w$]*)\s*:\s*(?:async\s*)?\([^()]*\)\s*=>\s*(?:apiCall|invokeCmd)\s*(?:<[^()<>]*>)?\s*\(\s*'([A-Za-z_][\w]*)'/g;
// Прямой вызов команды из файла (без аксессора).
const DIRECT_RE = /(?:apiCall|invokeCmd)\s*(?:<[^()<>]*>)?\s*\(\s*'([A-Za-z_][\w]*)'/g;
const LIST_CMD_RE = /_list(?:_|$)/;
const STORE_FILE_RE = /\.svelte\.ts$/;

// ---------------------------------------------------------------------------
// Разбор
// ---------------------------------------------------------------------------

function lineOf(text, index) {
  let n = 1;
  for (let i = 0; i < index; i++) if (text.charCodeAt(i) === 10) n++;
  return n;
}

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(full));
    else if (/\.ts$/.test(entry.name)) out.push(full);
  }
  return out;
}

/** Все .ts под <root>/src (пути от root, через «/»). */
function listSourceFiles(root) {
  return walk(path.join(root, SRC_DIR))
    .map((f) => path.relative(root, f).split(path.sep).join('/'))
    .sort();
}

function diskReader(root) {
  return (rel) => fs.readFileSync(path.join(root, rel), 'utf8');
}

/** Индекс закрывающей скобки к открывающей на openIdx. -1, если не нашлась. */
function matchBracket(text, openIdx) {
  const open = text[openIdx];
  const close = open === '(' ? ')' : '}';
  let depth = 0;
  for (let i = openIdx; i < text.length; i++) {
    if (text[i] === open) depth++;
    else if (text[i] === close) {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

const identRe = (id) => new RegExp(`(?<![\\w$.])${id}(?![\\w$])`);

/** Параметры продолжений `.then((rows) => …)`. */
function thenParams(text) {
  const out = [];
  for (const m of text.matchAll(/\.then\(\s*(?:async\s*)?\(?\s*([A-Za-z_$][\w$]*)\s*\)?\s*=>/g))
    out.push(m[1]);
  return [...new Set(out)];
}

/** Условия `if (…)`, ссылающиеся на один из идентификаторов, с телом и границами. */
function findGuards(text, idents) {
  const guards = [];
  for (const m of text.matchAll(/\bif\s*\(/g)) {
    const openIdx = m.index + m[0].length - 1;
    const closeIdx = matchBracket(text, openIdx);
    if (closeIdx < 0) continue;
    const cond = text.slice(openIdx + 1, closeIdx);
    if (!idents.some((id) => identRe(id).test(cond))) continue;
    let j = closeIdx + 1;
    while (j < text.length && /\s/.test(text[j])) j++;
    let end;
    if (text[j] === '{') {
      const b = matchBracket(text, j);
      end = b < 0 ? text.length : b + 1;
    } else {
      const semi = text.indexOf(';', j);
      end = semi < 0 ? text.length : semi + 1;
    }
    guards.push({ start: m.index, cond, body: text.slice(j, end), end });
  }
  return guards;
}

// ---------------------------------------------------------------------------
// Проверка
// ---------------------------------------------------------------------------

/**
 * @param read  (rel) => текст файла
 * @param files просматриваемые .ts-файлы (пути от root)
 * @param opts  { stores, minStores, full }
 * @returns { violations, scanned, registry, candidates, skipped, commands }
 */
function collectViolations(
  read,
  files,
  { stores = STORES, minStores = MIN_STORES, full = true } = {},
) {
  const violations = [];
  const push = (code, file, line, msg) => violations.push({ code, file, line, msg });
  const skipped = [];
  const texts = new Map();
  const textOf = (rel) => {
    if (!texts.has(rel)) texts.set(rel, stripComments(read(rel)));
    return texts.get(rel);
  };

  const scanFiles = files.filter((rel) => {
    if (GENERATED.has(rel)) {
      skipped.push({ file: rel, reason: GENERATED.get(rel) });
      return false;
    }
    return true;
  });

  // ---- [S6] реестр ----
  if (full && stores.length < minStores)
    push(
      'S6',
      '(реестр)',
      null,
      `в реестре ${stores.length} сторов, нужно не меньше ${minStores}: гейт зелёный по пустоте`,
    );
  const present = [];
  for (const e of stores) {
    // Запись реестра, не прошедшая собственную проверку, уже красная: правила
    // формы по ней не гоняем, иначе к S6 примешивается шум S1/S3/S5/S7.
    let bad = false;
    if (!e.reason || e.reason.length < MIN_REASON) {
      push(
        'S6',
        e.store,
        null,
        `запись реестра без причины (нужно не короче ${MIN_REASON} символов)`,
      );
      bad = true;
    }
    if (!STORE_FILE_RE.test(e.store)) {
      push('S6', e.store, null, 'путь записи реестра: ожидается стор на рунах (*.svelte.ts)');
      bad = true;
    }
    if (!e.fields || e.fields.length === 0) {
      push('S6', e.store, null, 'в записи реестра не перечислены обязательные поля DTO');
      bad = true;
    }
    if (!scanFiles.includes(e.store)) {
      push('S6', e.store, null, 'устаревшая запись реестра: такого файла в дереве нет');
      bad = true;
    }
    if (!bad) present.push(e);
  }

  // ---- карта «команда -> аксессоры api-модулей» (обходом, без ручного списка) ----
  const accessors = new Map();
  for (const rel of scanFiles)
    for (const m of textOf(rel).matchAll(ACCESSOR_RE)) {
      if (!accessors.has(m[2])) accessors.set(m[2], new Set());
      accessors.get(m[2]).add(m[1]);
    }

  // ---- правила по каждому стору реестра ----
  for (const e of present) {
    const text = textOf(e.store);
    const at = (idx) => lineOf(text, idx);

    // [S7] цепочка стор -> аксессор -> команда
    const names = accessors.get(e.command);
    if (!names || names.size === 0) {
      push(
        'S7',
        e.store,
        null,
        `команда «${e.command}» не найдена ни в одном api-модуле (\`<аксессор>: () => apiCall('${e.command}')\`): охрана охраняет пустоту`,
      );
    } else {
      const called = [...names].filter(
        (n) =>
          new RegExp(`\\.\\s*${n}\\s*\\(`).test(text) ||
          new RegExp(`(?<![\\w$.])${n}\\s*\\(`).test(text),
      );
      const direct = new RegExp(`'${e.command}'`).test(text);
      if (called.length === 0 && !direct)
        push(
          'S7',
          e.store,
          null,
          `стор не вызывает аксессор команды «${e.command}» (${[...names].map((n) => n + '()').join(', ')}): справочник грузится мимо охраны`,
        );
    }

    const idents = thenParams(text);
    if (idents.length === 0) {
      push(
        'S1',
        e.store,
        null,
        'не найдено продолжение `.then((rows) => …)`: охрану формы ответа не к чему привязать',
      );
      continue;
    }

    const guards = findGuards(text, idents);
    if (guards.length === 0) {
      push(
        'S1',
        e.store,
        null,
        `нет охраны формы ответа: ни одно условие не ссылается на ответ (${idents.join(', ')}) — index.html вместо JSON даст state='${e.readyValue}' и мусор в ${e.stateField}`,
      );
      continue;
    }
    const guard = guards[0];
    const gline = at(guard.start);

    // [S2] массивность
    if (!/Array\.isArray\s*\(/.test(guard.cond) && !/instanceof\s+Array/.test(guard.cond))
      push(
        'S2',
        e.store,
        gline,
        'в охране нет `Array.isArray(`: строка тоже «итерируемая», поэлементная проверка её пропустит',
      );

    // [S3] поэлементная проверка типов
    if (!/\.\s*(?:every|some)\s*\(/.test(guard.cond)) {
      push(
        'S3',
        e.store,
        gline,
        'в охране нет поэлементного обхода (`.every(`): проверена только оболочка ответа, не элементы',
      );
    } else {
      for (const field of e.fields ?? []) {
        const re = new RegExp(`typeof\\s+[A-Za-z_$][\\w$]*\\s*\\??\\.\\s*${field}(?![\\w$])`);
        if (!re.test(guard.cond))
          push(
            'S3',
            e.store,
            gline,
            `в охране нет проверки типа обязательного поля «${field}» (\`typeof r?.${field} === …\`)`,
          );
      }
    }

    // [S4] throw
    if (!/\bthrow\b/.test(guard.body))
      push(
        'S4',
        e.store,
        gline,
        'охрана не бросает: без `throw` ответ не попадёт в `.catch`, состояние и тост останутся неверными',
      );

    // [S5] порядок: записи только ПОСЛЕ охраны
    const writeRe = new RegExp(
      `([A-Za-z_$][\\w$]*)\\s*\\.\\s*${e.stateField}\\s*=\\s*([A-Za-z_$][\\w$]*)`,
      'g',
    );
    const writes = [...text.matchAll(writeRe)].filter((m) => idents.includes(m[2]));
    if (writes.length === 0) {
      push(
        'S5',
        e.store,
        null,
        `не найдена запись ответа в $state (\`<стор>.${e.stateField} = ${idents[0]}\`): форма записи изменилась, охрану больше нечем привязать к записи`,
      );
    } else {
      for (const m of writes)
        if (m.index < guard.end)
          push(
            'S5',
            e.store,
            at(m.index),
            `запись ответа в $state стоит ДО охраны (охрана на строке ${gline}): порядок обязателен, иначе мусор уже в сторе`,
          );
    }
    const readyRe = new RegExp(`\\.\\s*${e.readyField}\\s*=\\s*'${e.readyValue}'`, 'g');
    const readies = [...text.matchAll(readyRe)];
    if (readies.length === 0) {
      push(
        'S5',
        e.store,
        null,
        `не найден перевод состояния в '${e.readyValue}': форма записи изменилась, охрану больше нечем привязать к записи`,
      );
    } else {
      for (const m of readies)
        if (m.index < guard.end)
          push(
            'S5',
            e.store,
            at(m.index),
            `перевод состояния в '${e.readyValue}' стоит ДО охраны (охрана на строке ${gline}): порядок обязателен`,
          );
    }
  }

  // ---- [S8] сторы-справочники вне реестра ----
  const listAccessors = new Set();
  for (const [cmd, names] of accessors)
    if (LIST_CMD_RE.test(cmd)) for (const n of names) listAccessors.add(n);
  const registered = new Set(stores.map((e) => e.store));
  const candidates = [];
  for (const rel of scanFiles) {
    if (!STORE_FILE_RE.test(rel)) continue;
    const text = textOf(rel);
    if (!/\$state\b/.test(text)) continue;
    const direct = [...text.matchAll(DIRECT_RE)]
      .map((m) => m[1])
      .filter((c) => LIST_CMD_RE.test(c));
    const via = [...listAccessors].filter((n) => new RegExp(`\\.\\s*${n}\\s*\\(`).test(text));
    if (direct.length === 0 && via.length === 0) continue;
    candidates.push({ file: rel, via: [...direct, ...via.map((n) => n + '()')] });
    if (!registered.has(rel))
      push(
        'S8',
        rel,
        null,
        `стор на рунах зовёт справочник (${[...direct, ...via.map((n) => n + '()')].join(', ')}) и пишет в $state, но в реестре STORES его нет: родится без охраны формы ответа`,
      );
  }

  return {
    violations,
    scanned: scanFiles.length,
    registry: present.map((e) => e.store),
    candidates,
    skipped,
    commands: accessors.size,
  };
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL — якорь «${label}» встречается ${n} раз (нужно ровно 1)`);
    process.exit(1);
  }
  return source.replace(from, () => to);
}

const TYPES_STORE = 'src/lib/stores/deviceTypes.svelte.ts';
const STATUSES_STORE = 'src/lib/stores/deviceStatuses.svelte.ts';
const DEVICES_API = 'src/lib/api/devices.ts';

/** Текст охранного блока `if (…) { throw … }` из реального стора. */
function guardBlockOf(text, store) {
  const idents = thenParams(text);
  const guards = findGuards(text, idents);
  const g = guards.find((x) => /Array\.isArray/.test(x.cond) && /throw/.test(x.body));
  if (!g) {
    console.error(
      `${TAG} SELFTEST FAIL — в ${store} не найден охранный блок с Array.isArray и throw`,
    );
    process.exit(1);
  }
  return { block: text.slice(g.start, g.end), cond: g.cond };
}

function selftest(root) {
  const files = listSourceFiles(root);
  const disk = diskReader(root);
  const cases = [];

  const run = ({
    over = new Map(),
    extra = [],
    list = files,
    stores = STORES,
    minStores = MIN_STORES,
    full = true,
  } = {}) => {
    const synth = new Map(extra);
    const read = (rel) =>
      over.has(rel) ? over.get(rel) : synth.has(rel) ? synth.get(rel) : disk(rel);
    return collectViolations(read, [...list, ...synth.keys()], { stores, minStores, full });
  };
  const codes = (r) => [...new Set(r.violations.map((v) => v.code))].sort().join(',');
  // msg — подстрока, обязательная среди сообщений: разводит кейсы с одним кодом.
  const add = (name, expected, result, msg) =>
    cases.push({ name, expected, got: codes(result), result, msg });

  // --- контроль: настоящее дерево чисто ---
  add('контроль: реальное дерево чисто', '', run());

  // --- мутанты по РЕАЛЬНОМУ deviceTypes.svelte.ts ---
  const typesReal = disk(TYPES_STORE);
  const typesGuard = guardBlockOf(stripComments(typesReal), TYPES_STORE);
  // Блок берётся из текста со снятыми комментариями; в исходнике он тот же —
  // проверяем это явно, иначе mutate() ниже молча не найдёт якорь.
  if (typesReal.split(typesGuard.block).length - 1 !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL — охранный блок deviceTypes не найден в исходнике дословно`,
    );
    process.exit(1);
  }
  const typesOver = (txt, label = 'охрана deviceTypes') =>
    new Map([[TYPES_STORE, mutate(typesReal, typesGuard.block, txt, label)]]);

  // 1. охрана удалена целиком
  add(
    'M1 deviceTypes: охрана удалена целиком',
    'S1',
    run({ over: typesOver('') }),
    'нет охраны формы ответа',
  );
  // 2. снят Array.isArray, поэлементная проверка оставлена
  add(
    'M2 deviceTypes: снят Array.isArray (строка пройдёт как «итерируемая»)',
    'S2',
    run({
      over: typesOver(
        "if (!rows.every((r) => typeof r?.id === 'number' && typeof r?.name === 'string')) {\n        throw new Error('device_types_list: неожиданная форма ответа');\n      }",
      ),
    }),
  );
  // 3. снята поэлементная проверка, Array.isArray оставлен
  add(
    'M3 deviceTypes: снята поэлементная проверка',
    'S3',
    run({
      over: typesOver(
        "if (!Array.isArray(rows)) {\n        throw new Error('device_types_list: неожиданная форма ответа');\n      }",
      ),
    }),
    'поэлементного обхода',
  );
  // 3б. обход есть, но пропало поле name
  add(
    'M3б deviceTypes: в поэлементной проверке пропало поле name',
    'S3',
    run({
      over: typesOver(
        "if (\n        !Array.isArray(rows) ||\n        !rows.every((r) => typeof r?.id === 'number')\n      ) {\n        throw new Error('device_types_list: неожиданная форма ответа');\n      }",
      ),
    }),
    'обязательного поля «name»',
  );
  // 4. throw заменён на тихий return
  add(
    'M4 deviceTypes: throw заменён на тихий return',
    'S4',
    run({
      over: typesOver(
        "if (\n        !Array.isArray(rows) ||\n        !rows.every((r) => typeof r?.id === 'number' && typeof r?.name === 'string')\n      ) {\n        return;\n      }",
      ),
    }),
  );
  // 5. ПЕРЕСТАНОВКА: запись в $state поднята выше охраны
  {
    const anchor = `${typesGuard.block}\n      deviceTypesStore.items = rows;\n      deviceTypesStore.state = 'ready';`;
    const swapped = `deviceTypesStore.items = rows;\n      deviceTypesStore.state = 'ready';\n      ${typesGuard.block}`;
    add(
      'M5 deviceTypes: ПЕРЕСТАНОВКА — запись в $state поднята выше охраны',
      'S5',
      run({
        over: new Map([
          [TYPES_STORE, mutate(typesReal, anchor, swapped, 'охрана+записи deviceTypes')],
        ]),
      }),
      'стоит ДО охраны',
    );
  }
  // 5б. запись ответа переименована (охрану больше не к чему привязать)
  add(
    'M5б deviceTypes: запись ответа подменена на пустой массив',
    'S5',
    run({
      over: new Map([
        [
          TYPES_STORE,
          mutate(
            typesReal,
            'deviceTypesStore.items = rows;',
            'deviceTypesStore.items = [];',
            'запись items',
          ),
        ],
      ]),
    }),
    'не найдена запись ответа',
  );

  // --- 6. второй стор реестра: охрана удалена в deviceStatuses ---
  {
    const real = disk(STATUSES_STORE);
    const g = guardBlockOf(stripComments(real), STATUSES_STORE);
    if (real.split(g.block).length - 1 !== 1) {
      console.error(
        `${TAG} SELFTEST FAIL — охранный блок deviceStatuses не найден в исходнике дословно`,
      );
      process.exit(1);
    }
    add(
      'M6 deviceStatuses: охрана удалена целиком (правило держит не только первый стор)',
      'S1',
      run({
        over: new Map([[STATUSES_STORE, mutate(real, g.block, '', 'охрана deviceStatuses')]]),
      }),
      'src/lib/stores/deviceStatuses.svelte.ts',
    );
    add(
      'M6б deviceStatuses: пропало поле sort_order',
      'S3',
      run({
        over: new Map([
          [
            STATUSES_STORE,
            mutate(real, "typeof r?.sort_order === 'number',", 'true,', 'проверка sort_order'),
          ],
        ]),
      }),
      'обязательного поля «sort_order»',
    );
  }

  // --- 7. антивакуум реестра (а): запись на несуществующий файл ---
  const third = (over) => ({
    store: 'src/lib/stores/deviceTypes.svelte.ts',
    command: 'device_types_list',
    stateField: 'items',
    readyField: 'state',
    readyValue: 'ready',
    fields: ['id'],
    reason: 'третья запись реестра для проверки движка [S6] — причина достаточной длины',
    ...over,
  });
  add(
    'M7 реестр: запись на несуществующий файл',
    'S6',
    run({ stores: [...STORES, third({ store: 'src/lib/stores/nope.svelte.ts' })] }),
    'такого файла в дереве нет',
  );
  add(
    'реестр: причина короче 20 символов',
    'S6',
    run({ stores: [...STORES, third({ reason: 'потому что' })] }),
    'без причины',
  );
  add(
    'реестр: путь не *.svelte.ts',
    'S6',
    run({ stores: [...STORES, third({ store: 'src/lib/api/devices.ts' })] }),
    'ожидается стор на рунах',
  );
  add(
    'реестр: пустой список обязательных полей',
    'S6',
    run({ stores: [...STORES, third({ fields: [] })] }),
    'не перечислены обязательные поля',
  );
  // антивакуум (в): реестр меньше порога (на живом реестре из двух сторов порог 3
  // обязан краснеть — иначе константа MIN_STORES мертва)
  add('реестр: порог 3 на двух сторах', 'S6', run({ minStores: 3 }), 'гейт зелёный по пустоте');
  add('реестр: порог не срабатывает при full=false', '', run({ minStores: 3, full: false }));

  // --- антивакуум (б): цепочка стор -> аксессор -> команда ---
  {
    const api = disk(DEVICES_API);
    add(
      'M8 цепочка: команда device_types_list исчезла из api-модуля',
      'S7',
      run({
        over: new Map([
          [
            DEVICES_API,
            mutate(api, "'device_types_list'", "'device_types_listX'", 'строка команды типов'),
          ],
        ]),
      }),
      'не найдена ни в одном api-модуле',
    );
    add(
      'M9 цепочка: стор перестал вызывать свой аксессор',
      'S7',
      run({
        over: new Map([
          [TYPES_STORE, mutate(typesReal, '.types()', '.somethingElse()', 'вызов аксессора типов')],
        ]),
      }),
      'не вызывает аксессор команды',
    );
  }

  // --- S8: новый стор-справочник вне реестра ---
  add(
    'M10 S8: новый стор зовёт справочник через аксессор, но в реестре его нет',
    'S8',
    run({
      extra: [
        [
          'src/lib/stores/zzGroupTypes.svelte.ts',
          "import { groupTypes } from '$lib/api/groups';\nconst s = $state({ items: [] });\nexport function ensureLoaded() {\n  return groupTypes.list(true).then((rows) => {\n    s.items = rows;\n  });\n}\n",
        ],
      ],
    }),
    'в реестре STORES его нет',
  );
  add(
    'M10б S8: новый стор зовёт команду напрямую через apiCall',
    'S8',
    run({
      extra: [
        [
          'src/lib/stores/zzDirect.svelte.ts',
          "import { apiCall } from '$lib/api/client';\nconst s = $state({ items: [] });\nexport const load = () => apiCall('cartridge_models_list').then((rows) => { s.items = rows; });\n",
        ],
      ],
    }),
  );
  add(
    'S8: обычный модуль (не *.svelte.ts) с вызовом справочника — чисто',
    '',
    run({
      extra: [
        [
          'src/features/zz/page-data.ts',
          "import { apiCall } from '$lib/api/client';\nexport const load = () => apiCall('cartridge_models_list');\n",
        ],
      ],
    }),
  );
  add(
    'S8: стор без $state — чисто',
    '',
    run({
      extra: [
        [
          'src/lib/stores/zzNoState.svelte.ts',
          "import { apiCall } from '$lib/api/client';\nexport const load = () => apiCall('cartridge_models_list');\n",
        ],
      ],
    }),
  );

  // --- обход: сгенерированный bindings.ts пропускается ---
  add(
    'обход: src/bindings.ts пропущен',
    '',
    run({
      extra: [
        ['src/bindings.ts', "export const x = { list: () => apiCall<number[]>('zz_list') };\n"],
      ],
    }),
  );

  // --- WR-01: `/*` в строчном комментарии не прячет код ниже ---
  add(
    'WR-01: `/*` в строчном комментарии не прячет стор от S8',
    'S8',
    run({
      extra: [
        [
          'src/lib/stores/zzTrap.svelte.ts',
          [
            '// шаблоны лежат в templates/*.html',
            "import { apiCall } from '$lib/api/client';",
            'const s = $state({ items: [] });',
            "export const load = () => apiCall('cartridge_models_list').then((rows) => { s.items = rows; });",
            '/* настоящий блочный комментарий */',
            '',
          ].join('\n'),
        ],
      ],
    }),
  );
  // Охрана, закомментированная целиком, защитой не является.
  add(
    'WR-01: закомментированная охрана не считается охраной',
    'S1',
    run({
      over: new Map([
        [
          TYPES_STORE,
          mutate(
            typesReal,
            typesGuard.block,
            `/* ${typesGuard.block.replace(/\/\*|\*\//g, '')} */`,
            'охрана deviceTypes под комментарием',
          ),
        ],
      ]),
    }),
    'нет охраны формы ответа',
  );

  let failed = 0;
  for (const c of cases) {
    if (
      c.msg &&
      !c.result.violations.some((v) => v.msg.includes(c.msg) || v.file.includes(c.msg))
    ) {
      failed++;
      console.error(`${TAG} SELFTEST FAIL — ${c.name}: нет нарушения с текстом «${c.msg}»`);
      continue;
    }
    if (c.got !== c.expected) {
      failed++;
      console.error(
        `${TAG} SELFTEST FAIL — ${c.name}: ожидалось {${c.expected}}, получено {${c.got}}`,
      );
      for (const v of c.result.violations.slice(0, 5))
        console.error(`  [${v.code}] ${v.file}${v.line ? ':' + v.line : ''} — ${v.msg}`);
    }
  }
  if (failed) {
    console.error(`${TAG} SELFTEST FAIL — красных кейсов: ${failed} из ${cases.length}`);
    process.exit(1);
  }
  console.log(`${TAG} SELFTEST PASS — ${cases.length} кейсов`);
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const srcArg = args.find((a) => a.startsWith('--src='));
const ROOT = srcArg ? path.resolve(srcArg.slice('--src='.length)) : path.resolve(__dirname, '..');

if (args.includes('--selftest')) {
  selftest(ROOT);
} else {
  const files = listSourceFiles(ROOT);
  const res = collectViolations(diskReader(ROOT), files);
  if (args.includes('--report')) {
    console.log(`${TAG} просмотрено: ${res.scanned} .ts-файлов из ${files.length}`);
    console.log(`${TAG} команд с аксессорами: ${res.commands}`);
    console.log(`${TAG} реестр сторов (${res.registry.length}, порог ${MIN_STORES}):`);
    for (const f of res.registry) console.log(`  ${f}`);
    console.log(
      `${TAG} сторы-кандидаты (зовут справочник и пишут в $state): ${res.candidates.length}`,
    );
    for (const c of res.candidates) console.log(`  ${c.file} — ${c.via.join(', ')}`);
    console.log(`${TAG} пропущено (${res.skipped.length}):`);
    for (const s of res.skipped) console.log(`  ${s.file} — ${s.reason}`);
  }
  if (res.violations.length) {
    for (const v of res.violations)
      console.error(`${TAG} [${v.code}] ${v.file}${v.line ? ':' + v.line : ''} — ${v.msg}`);
    console.error(`${TAG} FAIL — ${res.violations.length} нарушений`);
    process.exit(1);
  }
  console.log(
    `${TAG} PASS — 0 нарушений (${res.scanned} файлов, ${res.registry.length} сторов реестра, ${res.candidates.length} кандидатов)`,
  );
}
