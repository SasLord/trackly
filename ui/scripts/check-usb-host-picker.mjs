#!/usr/bin/env node
// [check-usb-host-picker] ИСПОЛНЯЮЩИЙ golden-гейт чистого модуля
// `src/features/devices/usbHostPicker.ts` (фаза 41.13; PGR-03, PGR-04; D-06, D-07,
// D-08, D-15).
//
// Зачем он существует. Шапка самого модуля признавала: «JS-тест-раннера в проекте
// нет… Поведение закрепляется структурным гейтом и живой проверкой — до неё оно
// UNVERIFIED». Структурный греп (check-printer-usb-host-field.mjs) не ловит
// перестановку ветвей внутри `sectionFor`, ошибку склейки в `hostLabel`, потерю
// дедупликации в `buildHostList`, выпадение поля поиска из `filterSectionOne`,
// сломанный потолок `SUBTREE_CAP` и неверную таблицу истинности
// `isUsbHostFieldVisible`: всё это типизируется и грепается одинаково. Этот гейт
// модуль ТРАНСПИЛИРУЕТ и ИСПОЛНЯЕТ по golden-фикстуре
// `scripts/fixtures/usb-host-picker/cases.json`.
//
// РАЗДЕЛЕНИЕ С СОСЕДНИМ ГЕЙТОМ. `check-printer-usb-host-field.mjs` — СТРУКТУРНЫЙ:
// он держит КОНФИГУРАЦИЮ разметки (какой компонент, какие пропсы, какие ключи
// payload'а, какое условие вокруг поля). Этот гейт держит ПОВЕДЕНИЕ восьми чистых
// функций. Он соседа НЕ дублирует и НЕ заменяет: оба нужны, у них разные предметы.
//
// Что закрепляют кейсы (каждая строка — требование, а не реализация):
//   hostLabel              — PGR-04/D-08: подпись «Имя» либо «Имя · инв. 114», инв. №
//                            обрезается, пустой/пробельный/null/undefined инв. № даёт
//                            одно имя (а не «Имя · инв. »);
//   hostSub                — порядок склейки «инв. 114 · SN AB123», отсутствие
//                            подстроки как undefined (не как пустая строка);
//   effectiveQuery         — запрос, равный строке значения (после .trim() у ОБОИХ),
//                            считается пустым;
//   filterSectionOne       — пустой запрос возвращает всё (включая «Не указан»),
//                            непустой ищет регистронезависимой ПОДСТРОКОЙ по имени,
//                            инв. № И серийному № и выбрасывает «Не указан»;
//   buildHostList          — D-06: «Не указан» только при пустом запросе, затем
//                            секция группы, затем остальные; дедупликация считается
//                            по ПОЛНОЙ секции 1, а не по отфильтрованной; selfId
//                            выброшен;
//   sectionFor             — заголовок второй секции только при наличии первой;
//                            порядок ветвей (кейс краснеет при их перестановке);
//   isUsbHostFieldVisible  — PGR-03 и D-15: полная таблица истинности по всем 16
//                            комбинациям. Плюс ИНВАРИАНТ D-15: результат не меняется
//                            ни при каком значении «ip-подобных» ключей контекста
//                            (ipAddress / loadedPrinterIp / printerIp / ip_address /
//                            hasSavedHost) — возврат отменённой посылки R-05 «хост
//                            скрыт, если есть IP» краснеет;
//   walkGroupSubtree       — обход в ширину через подставную fetchComposition: якорь
//                            входит в результат, устройства не дублируются между
//                            узлами, excludeDeviceId исключается, ошибка ОДНОГО узла
//                            молча пропускает его, цикл не грузит узел дважды,
//                            потолок SUBTREE_CAP = 50 посещённых узлов.
//
// Отсутствие пакета `typescript` (devDependency ui/package.json) или неразбор модуля
// — КРАСНЫЙ с внятным текстом, а не молчаливый SKIP. Урезанная фикстура (меньше
// MIN_CASES кейсов, пропавшая функция, пропавшая комбинация таблицы истинности,
// пропавший разделяющий кейс) — тоже красный: гейт, которому ослабили фикстуру,
// перестаёт быть гейтом.
//
// Self-test (`--selftest`): абляция по РЕАЛЬНОМУ исходнику. Эталон обязан пройти всю
// фикстуру; затем каждый мутант обязан покраснеть. Якорь каждой мутации проверяется
// на уникальность (ровно 1 вхождение), иначе мутант снял бы гейт у соседнего
// фрагмента. Среди мутантов есть не только удаления, но и перестановки ветвей и
// возврат отменённых условий по IP.
//
// Чего гейт НЕ доказывает (остаётся за живой проверкой): рантайм рун Svelte 5,
// отрисовку заголовков секций в Dropdown, перекрытие панели, контраст в обеих темах,
// серверную часть (что сервер отдаёт именно этот состав и принимает хост).
//
// Zero-dependency, кроме `typescript` для транспиляции: node:fs/path/url/module/buffer.
//
// Usage:
//   node scripts/check-usb-host-picker.mjs               # проверить репозиторий
//   node scripts/check-usb-host-picker.mjs --impl=<path>  # проверить копию модуля
//   node scripts/check-usb-host-picker.mjs --selftest     # встроенная абляция

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { Buffer } from 'node:buffer';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-usb-host-picker]';

const MODULE_FILE = 'src/features/devices/usbHostPicker.ts';
const FIXTURE = path.join(__dirname, 'fixtures/usb-host-picker/cases.json');
// Фикстура сейчас 67 кейсов. Порог ниже — чтобы её нельзя было урезать молча.
const MIN_CASES = 60;

const UNDEF = '__undefined__';

const FUNCTIONS = [
  'hostLabel',
  'hostSub',
  'effectiveQuery',
  'filterSectionOne',
  'buildHostList',
  'sectionFor',
  'isUsbHostFieldVisible',
  'walkGroupSubtree',
];
const CONSTANTS = ['SUBTREE_CAP', 'SECTION_ALL'];

const VISIBILITY_KEYS = ['typeIsPrinter', 'isEdit', 'readonly', 'printerLoaded'];

// D-15 (пользователь 2026-11-01): «Принтер может быть связан по USB и в это же время
// быть сетевым». Видимость поля хоста не зависит от IP ни в одной ветке. Инвариант
// проверяется подмешиванием ip-подобных ключей: если функция снова начнёт их читать
// (отменённая посылка R-05), результаты по пробам разойдутся.
const IP_PROBES = [
  { label: 'без ip-ключей', extra: {} },
  {
    label: 'ip заполнен',
    extra: {
      ipAddress: '10.0.0.5',
      loadedPrinterIp: '10.0.0.5',
      printerIp: '10.0.0.5',
      ip_address: '10.0.0.5',
      hasSavedHost: true,
    },
  },
  {
    label: 'ip пуст',
    extra: {
      ipAddress: '',
      loadedPrinterIp: null,
      printerIp: '',
      ip_address: null,
      hasSavedHost: false,
    },
  },
];

// ---------------------------------------------------------------------------
// Загрузка модуля
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
  let js;
  try {
    js = ts.transpileModule(tsSource, {
      compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
      fileName: label,
      reportDiagnostics: false,
    }).outputText;
  } catch (e) {
    console.error(`${TAG} FAIL — ${label} не транспилируется: ${e.message}`);
    process.exit(1);
  }
  try {
    // data:-URL: модуль самодостаточен (ни одного import, кроме типов).
    return await import(
      `data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`
    );
  } catch (e) {
    console.error(`${TAG} FAIL — ${label} не загружается как модуль: ${e.message}`);
    process.exit(1);
  }
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

// ---------------------------------------------------------------------------
// Сравнение
// ---------------------------------------------------------------------------

/** JSON не умеет undefined: в фикстуре он записан строкой «__undefined__». */
function revive(v) {
  if (v === UNDEF) return undefined;
  if (Array.isArray(v)) return v.map(revive);
  if (v && typeof v === 'object') {
    const out = {};
    for (const k of Object.keys(v)) out[k] = revive(v[k]);
    return out;
  }
  return v;
}

function canonical(v) {
  if (v === undefined) return '«undefined»';
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`;
  if (v && typeof v === 'object') {
    return `{${Object.keys(v)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v);
}

// ---------------------------------------------------------------------------
// walkGroupSubtree: подставной состав
// ---------------------------------------------------------------------------

function buildGraph(input) {
  const graph = new Map();
  if (input.fan) {
    const kids = [];
    for (let i = 0; i < input.fan.children; i += 1) kids.push(input.root + 1 + i);
    graph.set(input.root, {
      devices: [
        {
          device_id: 1000,
          name: 'Узел-якорь',
          inventory_number: null,
          serial_number: null,
        },
      ],
      child_groups: kids,
    });
    for (const g of kids) {
      graph.set(g, {
        devices: [
          {
            device_id: 1000 + g,
            name: `Устройство ${g}`,
            inventory_number: null,
            serial_number: null,
          },
        ],
        child_groups: [],
      });
    }
    return graph;
  }
  for (const key of Object.keys(input.graph)) graph.set(Number(key), input.graph[key]);
  return graph;
}

async function runWalkCase(mod, input) {
  const graph = buildGraph(input);
  const reject = new Set(input.reject ?? []);
  const fetched = [];
  const fetchComposition = (gid) => {
    fetched.push(gid);
    if (reject.has(gid)) return Promise.reject(new Error(`узел ${gid} недоступен`));
    const node = graph.get(gid);
    if (!node) return Promise.reject(new Error(`узел ${gid} не найден`));
    return Promise.resolve({
      devices: node.devices.map((d) => ({ ...d })),
      child_groups: node.child_groups.map((id) => ({ id })),
    });
  };
  const items = await mod.walkGroupSubtree(fetchComposition, input.root, {
    excludeDeviceId: input.excludeDeviceId ?? null,
  });
  return { items, fetched };
}

// ---------------------------------------------------------------------------
// Антиослабление: чего фикстура обязана содержать
// ---------------------------------------------------------------------------

function fixtureGuards(cases) {
  const failures = [];
  const byFn = (fn) => cases.filter((c) => c.fn === fn);

  for (const fn of FUNCTIONS) {
    if (byFn(fn).length === 0) failures.push(`в фикстуре нет ни одного кейса для ${fn}`);
  }
  for (const name of CONSTANTS) {
    if (!cases.some((c) => c.fn === 'const' && c.input.name === name)) {
      failures.push(`в фикстуре нет кейса для экспортируемой константы ${name}`);
    }
  }

  // hostLabel: пробельный инв. № и инв. № с пробелами по краям.
  const labels = byFn('hostLabel');
  if (
    !labels.some(
      (c) =>
        typeof c.input.inventoryNo === 'string' &&
        c.input.inventoryNo.trim() === '' &&
        c.input.inventoryNo !== '',
    )
  ) {
    failures.push(
      'в фикстуре нет кейса hostLabel с инв. № из одних пробелов (без него .trim() можно снять)',
    );
  }
  if (
    !labels.some(
      (c) =>
        typeof c.input.inventoryNo === 'string' &&
        c.input.inventoryNo.trim() !== '' &&
        c.input.inventoryNo !== c.input.inventoryNo.trim(),
    )
  ) {
    failures.push('в фикстуре нет кейса hostLabel с инв. № в пробелах по краям');
  }
  if (
    !labels.some((c) => c.input.inventoryNo === null) ||
    !labels.some((c) => c.input.inventoryNo === UNDEF)
  ) {
    failures.push('в фикстуре нет кейсов hostLabel с инв. № null И undefined');
  }

  // hostSub: отсутствие подстроки записано как undefined, а не как пустая строка.
  if (!byFn('hostSub').some((c) => c.expected === UNDEF)) {
    failures.push('в фикстуре нет кейса hostSub, где подстроки нет (ожидается undefined)');
  }

  // effectiveQuery: равенство строке значения считается после .trim() у ОБОИХ.
  if (
    !byFn('effectiveQuery').some(
      (c) =>
        c.expected === '' &&
        c.input.raw.trim() !== '' &&
        c.input.raw.trim() === c.input.currentLabel.trim() &&
        c.input.raw !== c.input.currentLabel,
    )
  ) {
    failures.push(
      'в фикстуре нет кейса effectiveQuery, где запрос равен строке значения только после .trim() у обоих',
    );
  }

  // filterSectionOne: поиск обязан находить ТОЛЬКО по каждому из трёх полей.
  const filters = byFn('filterSectionOne');
  const onlyBy = (field) =>
    filters.some((c) => {
      const q = String(c.input.query).trim().toLowerCase();
      if (q === '' || !Array.isArray(c.expected) || c.expected.length === 0) return false;
      return c.expected.every((d) => {
        const hit = (v) => typeof v === 'string' && v.toLowerCase().includes(q);
        const fields = { name: d.name, inventoryNo: d.inventoryNo, serialNo: d.serialNo };
        if (!hit(fields[field])) return false;
        return Object.keys(fields).every((k) => k === field || !hit(fields[k]));
      });
    });
  for (const field of ['name', 'inventoryNo', 'serialNo']) {
    if (!onlyBy(field)) {
      failures.push(
        `в фикстуре нет кейса filterSectionOne, где устройство находится ТОЛЬКО по полю ${field} (без него поле можно выкинуть из поиска)`,
      );
    }
  }
  if (
    !filters.some(
      (c) =>
        String(c.input.query).trim() !== '' &&
        c.input.items.some((it) => it.kind === 'clear') &&
        Array.isArray(c.expected) &&
        !c.expected.some((it) => it.kind === 'clear'),
    )
  ) {
    failures.push(
      'в фикстуре нет кейса filterSectionOne с непустым запросом, где пункт «Не указан» выброшен',
    );
  }
  if (
    !filters.some(
      (c) =>
        String(c.input.query).trim() === '' &&
        c.input.query !== '' &&
        canonical(revive(c.expected)) === canonical(revive(c.input.items)),
    )
  ) {
    failures.push(
      'в фикстуре нет кейса filterSectionOne с запросом из пробелов, возвращающего весь список',
    );
  }
  if (
    !filters.some(
      (c) =>
        String(c.input.query).trim() !== '' &&
        c.input.items.some(
          (it) => it.kind === 'device' && (it.inventoryNo === null || it.serialNo === null),
        ),
    )
  ) {
    failures.push('в фикстуре нет кейса filterSectionOne с null-полями при непустом запросе');
  }

  // buildHostList: дедупликация по ПОЛНОЙ секции 1, а не по отфильтрованной.
  if (
    !byFn('buildHostList').some((c) => {
      const q = c.input.query.trim();
      if (q === '' || q === c.input.currentLabel.trim()) return false;
      const shownIds = new Set(c.expected.filter((it) => it.kind === 'device').map((it) => it.id));
      return c.input.sectionOne.some(
        (it) =>
          it.kind === 'device' &&
          !shownIds.has(it.id) &&
          c.input.searchResults.some((s) => s.kind === 'device' && s.id === it.id),
      );
    })
  ) {
    failures.push(
      'в фикстуре нет кейса buildHostList, различающего дедупликацию по полной секции 1 и по отфильтрованной (устройство отфильтровано запросом, есть в ответе поиска и не должно всплыть в секции 2)',
    );
  }
  if (
    !byFn('buildHostList').some(
      (c) =>
        c.input.selfId !== null &&
        c.input.searchResults.some((s) => s.kind === 'device' && s.id === c.input.selfId),
    )
  ) {
    failures.push('в фикстуре нет кейса buildHostList, где selfId есть в ответе поиска');
  }
  if (
    !byFn('buildHostList').some(
      (c) =>
        c.input.query.trim() !== '' &&
        c.input.query.trim() !== c.input.currentLabel.trim() &&
        !c.expected.some((it) => it.kind === 'clear'),
    )
  ) {
    failures.push('в фикстуре нет кейса buildHostList с непустым запросом без пункта «Не указан»');
  }
  if (
    !byFn('buildHostList').some(
      (c) =>
        c.input.query.trim() !== '' &&
        c.input.query.trim() === c.input.currentLabel.trim() &&
        c.expected.some((it) => it.kind === 'clear'),
    )
  ) {
    failures.push(
      'в фикстуре нет кейса buildHostList, где запрос равен строке значения и список ведёт себя как при пустом запросе',
    );
  }

  // sectionFor: обе ветви при обоих значениях hasSectionOne.
  const sections = byFn('sectionFor');
  const hasSectionCase = (section, hasSectionOne) =>
    sections.some(
      (c) =>
        c.input.item.kind === 'device' &&
        c.input.item.section === section &&
        c.input.ctx.hasSectionOne === hasSectionOne,
    );
  for (const section of ['group', 'all']) {
    for (const flag of [true, false]) {
      if (!hasSectionCase(section, flag)) {
        failures.push(`в фикстуре нет кейса sectionFor(section=${section}, hasSectionOne=${flag})`);
      }
    }
  }
  if (!sections.some((c) => c.input.item.kind === 'clear' && c.input.ctx.hasSectionOne === true)) {
    failures.push('в фикстуре нет кейса sectionFor для «Не указан» при наличии секции 1');
  }

  // isUsbHostFieldVisible: полная таблица истинности 16 комбинаций.
  const vis = byFn('isUsbHostFieldVisible');
  for (const c of vis) {
    const keys = Object.keys(c.input).sort();
    if (canonical(keys) !== canonical([...VISIBILITY_KEYS].sort())) {
      failures.push(
        `кейс «${c.name}»: в контексте isUsbHostFieldVisible лишние или недостающие ключи (${keys.join(',')}); ip-подобные ключи подмешивает гейт, в фикстуре их быть не должно`,
      );
    }
  }
  for (const typeIsPrinter of [true, false]) {
    for (const isEdit of [true, false]) {
      for (const readonly of [true, false]) {
        for (const printerLoaded of [true, false]) {
          const has = vis.some(
            (c) =>
              c.input.typeIsPrinter === typeIsPrinter &&
              c.input.isEdit === isEdit &&
              c.input.readonly === readonly &&
              c.input.printerLoaded === printerLoaded,
          );
          if (!has) {
            failures.push(
              `в фикстуре нет комбинации isUsbHostFieldVisible(typeIsPrinter=${typeIsPrinter}, isEdit=${isEdit}, readonly=${readonly}, printerLoaded=${printerLoaded})`,
            );
          }
        }
      }
    }
  }

  // walkGroupSubtree: обязательные ситуации.
  const walks = byFn('walkGroupSubtree');
  if (!walks.some((c) => (c.input.reject ?? []).length > 0)) {
    failures.push('в фикстуре нет кейса walkGroupSubtree с недоступным узлом');
  }
  if (!walks.some((c) => c.input.excludeDeviceId != null)) {
    failures.push('в фикстуре нет кейса walkGroupSubtree с excludeDeviceId');
  }
  if (!walks.some((c) => c.input.fan && c.input.fan.children > 50)) {
    failures.push(
      'в фикстуре нет кейса walkGroupSubtree, где узлов больше SUBTREE_CAP (потолок не проверяется)',
    );
  }
  if (
    !walks.some((c) => {
      if (!c.input.graph) return false;
      const counts = new Map();
      for (const node of Object.values(c.input.graph)) {
        for (const d of node.devices) counts.set(d.device_id, (counts.get(d.device_id) ?? 0) + 1);
      }
      return [...counts.values()].some((n) => n > 1);
    })
  ) {
    failures.push(
      'в фикстуре нет кейса walkGroupSubtree, где одно устройство встречается в двух узлах (дедупликация не проверяется)',
    );
  }
  if (
    !walks.some(
      (c) =>
        c.input.graph &&
        Object.values(c.input.graph).some((node) => node.child_groups.includes(c.input.root)),
    )
  ) {
    failures.push('в фикстуре нет кейса walkGroupSubtree с циклом (ребро назад в якорь)');
  }
  if (!walks.some((c) => Array.isArray(c.expected.items))) {
    failures.push(
      'в фикстуре нет кейса walkGroupSubtree, сверяющего пункты целиком (отображение полей и section не проверяется)',
    );
  }
  if (!walks.some((c) => Array.isArray(c.expected.fetched))) {
    failures.push(
      'в фикстуре нет кейса walkGroupSubtree, сверяющего порядок загруженных узлов (обход в ширину не проверяется)',
    );
  }

  return failures;
}

// ---------------------------------------------------------------------------
// Прогон кейсов
// ---------------------------------------------------------------------------

/** Возвращает список текстов провалов (пустой — всё в порядке). */
async function runCases(mod, cases) {
  const failures = [];
  for (const fn of FUNCTIONS) {
    if (typeof mod[fn] !== 'function') failures.push(`модуль не экспортирует функцию ${fn}`);
  }
  if (failures.length) return failures;

  for (const c of cases) {
    const input = revive(c.input);
    const expected = revive(c.expected);
    try {
      let got;
      switch (c.fn) {
        case 'const':
          if (!(input.name in mod)) {
            failures.push(`${c.name}: модуль не экспортирует константу ${input.name}`);
            continue;
          }
          got = mod[input.name];
          break;
        case 'hostLabel':
          got = mod.hostLabel(input.name, input.inventoryNo);
          break;
        case 'hostSub':
          got = mod.hostSub(input);
          break;
        case 'effectiveQuery':
          got = mod.effectiveQuery(input.raw, input.currentLabel);
          break;
        case 'filterSectionOne':
          got = mod.filterSectionOne(input.items, input.query);
          break;
        case 'buildHostList':
          got = mod.buildHostList(input);
          break;
        case 'sectionFor':
          got = mod.sectionFor(input.item, input.ctx);
          break;
        case 'isUsbHostFieldVisible': {
          got = mod.isUsbHostFieldVisible(input);
          // Инвариант D-15: ни одна ip-подобная добавка не меняет результат.
          for (const probe of IP_PROBES) {
            if (Object.keys(probe.extra).length === 0) continue;
            const res = mod.isUsbHostFieldVisible({ ...input, ...probe.extra });
            if (canonical(res) !== canonical(got)) {
              failures.push(
                `${c.name} [проба «${probe.label}»]: с ip-подобными ключами получено ${canonical(res)}, без них ${canonical(got)} — видимость поля хоста не должна зависеть от IP ни в одной ветке (D-15, отменённая посылка R-05)`,
              );
            }
          }
          break;
        }
        case 'walkGroupSubtree': {
          const { items, fetched } = await runWalkCase(mod, input);
          const checks = [];
          if (Array.isArray(expected.items)) checks.push(['пункты', items, expected.items]);
          if (Array.isArray(expected.ids)) {
            checks.push(['id пунктов', items.map((it) => it.id), expected.ids]);
          }
          if (typeof expected.count === 'number') {
            checks.push(['число пунктов', items.length, expected.count]);
          }
          if (Array.isArray(expected.fetched)) {
            checks.push(['порядок загруженных узлов', fetched, expected.fetched]);
          }
          if (typeof expected.fetchedCount === 'number') {
            checks.push(['число загруженных узлов', fetched.length, expected.fetchedCount]);
          }
          if (checks.length === 0) {
            failures.push(`${c.name}: кейс walkGroupSubtree ничего не утверждает`);
            continue;
          }
          for (const [what, actual, want] of checks) {
            if (canonical(actual) !== canonical(want)) {
              failures.push(
                `${c.name} (${what}): получено ${canonical(actual)}, ожидалось ${canonical(want)}`,
              );
            }
          }
          continue;
        }
        default:
          failures.push(`${c.name}: неизвестная функция «${c.fn}» в фикстуре`);
          continue;
      }
      if (canonical(got) !== canonical(expected)) {
        failures.push(`${c.name}: получено ${canonical(got)}, ожидалось ${canonical(expected)}`);
      }
    } catch (e) {
      failures.push(`${c.name}: исключение ${e.message}`);
    }
  }
  return [...failures, ...fixtureGuards(cases)];
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
  // hostLabel
  {
    name: 'hostLabel: инв. № не обрезается',
    from: "const inv = (inventoryNo ?? '').trim();",
    to: "const inv = inventoryNo ?? '';",
  },
  {
    name: 'hostLabel: имя и инв. № переставлены',
    from: "return inv === '' ? name : `${name} · инв. ${inv}`;",
    to: "return inv === '' ? name : `инв. ${inv} · ${name}`;",
  },
  {
    name: 'hostLabel: иной разделитель',
    from: "return inv === '' ? name : `${name} · инв. ${inv}`;",
    to: "return inv === '' ? name : `${name}, инв. ${inv}`;",
  },
  {
    name: 'hostLabel: пустой инв. № даёт «Имя · инв. »',
    from: "return inv === '' ? name : `${name} · инв. ${inv}`;",
    to: 'return inv === undefined ? name : `${name} · инв. ${inv}`;',
  },
  // hostSub
  {
    name: 'hostSub: инв. и SN переставлены',
    from: '    item.inventoryNo ? `инв. ${item.inventoryNo}` : null,\n    item.serialNo ? `SN ${item.serialNo}` : null,',
    to: '    item.serialNo ? `SN ${item.serialNo}` : null,\n    item.inventoryNo ? `инв. ${item.inventoryNo}` : null,',
  },
  {
    name: 'hostSub: иной разделитель склейки',
    from: ".join(' · ')",
    to: ".join(', ')",
  },
  {
    name: 'hostSub: в части «инв.» печатается серийный №',
    from: 'item.inventoryNo ? `инв. ${item.inventoryNo}` : null,',
    to: 'item.inventoryNo ? `инв. ${item.serialNo}` : null,',
  },
  {
    name: 'hostSub: отсутствие подстроки как пустая строка, а не undefined',
    from: "return parts.length > 0 ? parts.join(' · ') : undefined;",
    to: "return parts.join(' · ');",
  },
  {
    name: 'hostSub: иная подпись серийного №',
    from: 'item.serialNo ? `SN ${item.serialNo}` : null,',
    to: 'item.serialNo ? `S/N ${item.serialNo}` : null,',
  },
  // walkGroupSubtree
  {
    name: 'walkGroupSubtree: потеряна дедупликация устройств между узлами',
    from: 'if (seenDevices.has(d.device_id)) continue;',
    to: '',
  },
  {
    name: 'walkGroupSubtree: excludeDeviceId не исключается',
    from: 'if (options.excludeDeviceId != null && d.device_id === options.excludeDeviceId) continue;',
    to: '',
  },
  {
    name: 'walkGroupSubtree: ошибка узла не глотается (падает весь обход)',
    from: '.catch((): CompositionLike | null => null)',
    to: '',
  },
  {
    name: 'walkGroupSubtree: пустой ответ узла не пропускается',
    from: 'if (comp === null) continue;',
    to: '',
  },
  {
    name: 'walkGroupSubtree: потолок >= заменён на > (на один узел больше)',
    from: 'if (visited.size >= SUBTREE_CAP) continue;',
    to: 'if (visited.size > SUBTREE_CAP) continue;',
  },
  {
    name: 'walkGroupSubtree: потолок снят',
    from: 'if (visited.size >= SUBTREE_CAP) continue;',
    to: '',
  },
  {
    name: 'walkGroupSubtree: якорь не помечен посещённым заранее (цикл грузит его дважды)',
    from: 'const visited = new Set<number>([rootGroupId]);',
    to: 'const visited = new Set<number>();',
  },
  {
    name: 'walkGroupSubtree: в инв. № попадает серийный',
    from: 'inventoryNo: d.inventory_number,',
    to: 'inventoryNo: d.serial_number,',
  },
  {
    name: 'walkGroupSubtree: пункты поддерева помечены секцией «все»',
    from: "section: 'group',",
    to: "section: 'all',",
  },
  // filterSectionOne
  {
    name: 'filterSectionOne: запрос не приводится к нижнему регистру',
    from: 'const q = query.trim().toLowerCase();',
    to: 'const q = query.trim();',
  },
  {
    name: 'filterSectionOne: запрос не обрезается',
    from: 'const q = query.trim().toLowerCase();',
    to: 'const q = query.toLowerCase();',
  },
  {
    name: 'filterSectionOne: значение поля не приводится к нижнему регистру',
    from: '(v) => v != null && v.toLowerCase().includes(q),',
    to: '(v) => v != null && v.includes(q),',
  },
  {
    name: 'filterSectionOne: null-поле не защищено (падает)',
    from: '(v) => v != null && v.toLowerCase().includes(q),',
    to: '(v) => v.toLowerCase().includes(q),',
  },
  {
    name: 'filterSectionOne: поиск по подстроке заменён на поиск по началу',
    from: '(v) => v != null && v.toLowerCase().includes(q),',
    to: '(v) => v != null && v.toLowerCase().startsWith(q),',
  },
  {
    name: 'filterSectionOne: имя выпало из полей поиска',
    from: '[it.name, it.inventoryNo, it.serialNo]',
    to: '[it.inventoryNo, it.serialNo]',
  },
  {
    name: 'filterSectionOne: инв. № выпал из полей поиска',
    from: '[it.name, it.inventoryNo, it.serialNo]',
    to: '[it.name, it.serialNo]',
  },
  {
    name: 'filterSectionOne: серийный № выпал из полей поиска',
    from: '[it.name, it.inventoryNo, it.serialNo]',
    to: '[it.name, it.inventoryNo]',
  },
  {
    name: 'filterSectionOne: пустой запрос возвращает пустой список',
    from: "if (q === '') return items;",
    to: "if (q === '') return [];",
  },
  {
    name: 'filterSectionOne: «Не указан» остаётся при непустом запросе',
    from: "if (it.kind !== 'device') return false;",
    to: "if (it.kind !== 'device') return true;",
  },
  // effectiveQuery
  {
    name: 'effectiveQuery: запрос не обрезается',
    from: 'const q = raw.trim();',
    to: 'const q = raw;',
  },
  {
    name: 'effectiveQuery: строка значения не обрезается',
    from: "return q === currentLabel.trim() ? '' : q;",
    to: "return q === currentLabel ? '' : q;",
  },
  {
    name: 'effectiveQuery: ветви переставлены',
    from: "return q === currentLabel.trim() ? '' : q;",
    to: "return q === currentLabel.trim() ? q : '';",
  },
  // buildHostList
  {
    name: 'buildHostList: секция 1 не фильтруется запросом',
    from: 'const first = filterSectionOne(args.sectionOne, q);',
    to: 'const first = args.sectionOne;',
  },
  {
    name: 'buildHostList: дедупликация считается по отфильтрованной секции 1',
    from: "for (const it of args.sectionOne) if (it.kind === 'device') taken.add(it.id);",
    to: "for (const it of first) if (it.kind === 'device') taken.add(it.id);",
  },
  {
    name: 'buildHostList: дедупликация снята',
    from: '!taken.has(it.id) &&',
    to: '',
  },
  {
    name: 'buildHostList: selfId не исключается',
    from: '&& it.id !== args.selfId',
    to: '',
  },
  {
    name: 'buildHostList: секции переставлены (секция «все» перед секцией группы, D-06)',
    from: "return [...(q === '' ? [{ kind: 'clear' } as HostItem] : []), ...first, ...second];",
    to: "return [...(q === '' ? [{ kind: 'clear' } as HostItem] : []), ...second, ...first];",
  },
  {
    name: 'buildHostList: «Не указан» добавляется всегда',
    from: "...(q === '' ? [{ kind: 'clear' } as HostItem] : [])",
    to: "...[{ kind: 'clear' } as HostItem]",
  },
  {
    name: 'buildHostList: «Не указан» не добавляется никогда',
    from: "...(q === '' ? [{ kind: 'clear' } as HostItem] : [])",
    to: '...[]',
  },
  {
    name: 'buildHostList: запрос берётся сырым, без effectiveQuery',
    from: 'const q = effectiveQuery(args.query, args.currentLabel);',
    to: 'const q = args.query.trim();',
  },
  // sectionFor
  {
    name: 'sectionFor: заголовок второй секции без условия «есть первая»',
    from: 'return ctx.hasSectionOne ? SECTION_ALL : undefined;',
    to: 'return SECTION_ALL;',
  },
  {
    name: 'sectionFor: вторая секция подписана заголовком первой',
    from: 'return ctx.hasSectionOne ? SECTION_ALL : undefined;',
    to: 'return ctx.hasSectionOne ? ctx.sectionOneTitle : undefined;',
  },
  {
    name: 'sectionFor: ветвь группы получила лишнее условие hasSectionOne (перестановка условий)',
    from: "if (item.section === 'group') return ctx.sectionOneTitle;",
    to: "if (item.section === 'group') return ctx.hasSectionOne ? ctx.sectionOneTitle : undefined;",
  },
  {
    name: 'sectionFor: ветвь «Не указан» снята (пункт получает заголовок)',
    from: "if (item.kind === 'clear') return undefined;",
    to: '',
  },
  {
    name: 'SECTION_ALL: иной текст заголовка',
    from: "export const SECTION_ALL = 'Все устройства';",
    to: "export const SECTION_ALL = 'Остальные устройства';",
  },
  {
    name: 'SUBTREE_CAP: иной потолок',
    from: 'export const SUBTREE_CAP = 50;',
    to: 'export const SUBTREE_CAP = 20;',
  },
  // isUsbHostFieldVisible
  {
    name: 'isUsbHostFieldVisible: проверка типа «принтер» снята',
    from: 'if (!ctx.typeIsPrinter) return false;',
    to: '',
  },
  {
    name: 'isUsbHostFieldVisible: просмотр показывает поле без ответа сервера',
    from: 'if (ctx.readonly) return ctx.printerLoaded;',
    to: 'if (ctx.readonly) return true;',
  },
  {
    name: 'isUsbHostFieldVisible: создание ждёт ответа сервера (PGR-03 сломан)',
    from: 'if (!ctx.isEdit) return true;',
    to: 'if (!ctx.isEdit) return ctx.printerLoaded;',
  },
  {
    name: 'isUsbHostFieldVisible: правка показывает поле без ответа сервера',
    from: 'if (!ctx.isEdit) return true;\n  return ctx.printerLoaded;',
    to: 'if (!ctx.isEdit) return true;\n  return true;',
  },
  {
    name: 'isUsbHostFieldVisible: ветви просмотра и создания переставлены',
    from: '  if (ctx.readonly) return ctx.printerLoaded;\n  if (!ctx.isEdit) return true;',
    to: '  if (!ctx.isEdit) return true;\n  if (ctx.readonly) return ctx.printerLoaded;',
  },
  {
    name: 'isUsbHostFieldVisible: возврат отменённой посылки R-05 — поле скрыто при непустом IP (D-15)',
    from: 'if (!ctx.typeIsPrinter) return false;',
    to: 'if (!ctx.typeIsPrinter) return false;\n  if (ctx.ipAddress) return false;',
  },
  {
    name: 'isUsbHostFieldVisible: возврат условия по загруженному IP в ветви просмотра (D-15)',
    from: 'if (ctx.readonly) return ctx.printerLoaded;',
    to: 'if (ctx.readonly) return ctx.printerLoaded && !ctx.loadedPrinterIp;',
  },
  {
    name: 'isUsbHostFieldVisible: видимость при создании поставлена в зависимость от сохранённого хоста',
    from: 'if (!ctx.isEdit) return true;',
    to: 'if (!ctx.isEdit) return ctx.hasSavedHost === true;',
  },
];

/** Фикстура с удалённым кейсом по предикату — для проверки антиослабления. */
function withoutCase(cases, pred) {
  const idx = cases.findIndex(pred);
  if (idx < 0) return null;
  return cases.filter((_, i) => i !== idx);
}

const GUARD_PROBES = [
  {
    name: 'убран кейс с инв. № из одних пробелов',
    pred: (c) =>
      c.fn === 'hostLabel' &&
      typeof c.input.inventoryNo === 'string' &&
      c.input.inventoryNo !== '' &&
      c.input.inventoryNo.trim() === '',
  },
  {
    name: 'убрана комбинация таблицы истинности видимости',
    pred: (c) => c.fn === 'isUsbHostFieldVisible',
  },
  {
    name: 'убран кейс поиска только по серийному №',
    pred: (c) => c.fn === 'filterSectionOne' && String(c.input.query) === 'b12',
  },
  {
    name: 'убран кейс потолка поддерева',
    pred: (c) => c.fn === 'walkGroupSubtree' && Boolean(c.input.fan),
  },
  {
    name: 'убран кейс различения дедупликации buildHostList',
    pred: (c) => c.fn === 'buildHostList' && String(c.input.query) === 'рабочая',
  },
  {
    name: 'убраны все кейсы sectionFor',
    pred: (c) => c.fn === 'sectionFor',
  },
];

async function selftest(cases) {
  let tsSource;
  try {
    tsSource = fs.readFileSync(path.join(UI_ROOT, MODULE_FILE), 'utf8');
  } catch {
    console.error(`${TAG} SELFTEST FAIL — не удалось прочитать ${MODULE_FILE}`);
    process.exit(1);
  }

  // 1. Эталон проходит фикстуру целиком.
  const goodFailures = await runCases(await loadModuleFromTs(tsSource, 'good.ts'), cases);
  if (goodFailures.length) {
    console.error(
      `${TAG} SELFTEST FAIL — реальный исходник не прошёл фикстуру:\n  - ${goodFailures.join('\n  - ')}`,
    );
    process.exit(1);
  }

  // 2. Каждый мутант РЕАЛЬНОГО исходника обязан покраснеть.
  for (const m of SOURCE_MUTANTS) {
    const mutated = mutate(tsSource, m.from, m.to, m.name);
    if (mutated === tsSource) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» не изменил исходник`);
      process.exit(1);
    }
    const failures = await runCases(await loadModuleFromTs(mutated, 'mutant.ts'), cases);
    if (!failures.length) {
      console.error(`${TAG} SELFTEST FAIL — мутант «${m.name}» прошёл фикстуру: гейт его не ловит`);
      process.exit(1);
    }
  }

  // 3. Антиослабление: урезанная фикстура обязана краснеть на эталонном исходнике.
  const good = await loadModuleFromTs(tsSource, 'good.ts');
  for (const probe of GUARD_PROBES) {
    const trimmed = withoutCase(cases, probe.pred);
    if (trimmed === null) {
      console.error(
        `${TAG} SELFTEST FAIL — проба антиослабления «${probe.name}» не нашла кейса в фикстуре`,
      );
      process.exit(1);
    }
    const failures = await runCases(good, trimmed);
    if (!failures.length) {
      console.error(
        `${TAG} SELFTEST FAIL — «${probe.name}»: урезанная фикстура прошла, гейт ослабляется молча`,
      );
      process.exit(1);
    }
  }

  console.log(
    `${TAG} selftest OK — эталон проходит ${cases.length} кейсов; ${SOURCE_MUTANTS.length} мутантов исходника и ${GUARD_PROBES.length} проб урезания фикстуры ловятся`,
  );
}

// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { impl: path.join(UI_ROOT, MODULE_FILE), selftest: false };
  for (const a of argv) {
    if (a === '--selftest') args.selftest = true;
    else if (a.startsWith('--impl=')) args.impl = path.resolve(process.cwd(), a.slice(7));
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
  const mod = await loadModuleFromTs(tsSource, path.basename(args.impl));
  const failures = await runCases(mod, cases);
  if (failures.length) {
    console.error(`${TAG} FAIL —`);
    for (const f of failures) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log(
    `${TAG} OK — ${cases.length} кейсов фикстуры на ${FUNCTIONS.length} функциях и ${CONSTANTS.length} константах ${path.relative(UI_ROOT, args.impl)}`,
  );
}

main();
