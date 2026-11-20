#!/usr/bin/env node
// [check-employee-routes] Гейт таблицы маршрутов сотрудника (фаза 41.6, EMP-01).
//
// Почему он существует: раздел «Устройства» сотрудника — это ОДНА строка в
// `employeeRoutes` (ui/src/routes.ts), а «всё прочее → Доступ запрещён» — это
// одновременно запись '*' и ОТСУТСТВИЕ любых других ключей. Добавить туда
// маршрут персонала ('/groups', '/devices' → DevicesPage) или потерять
// '/devices' можно, не сломав ни svelte-check, ни eslint, ни pnpm build.
//
// Правила (по тексту routes.ts без комментариев):
//   [R1] тело `employeeRoutes` разбирается на записи вида 'ключ': Идентификатор;
//        любая иная запись (spread, вычисляемый ключ, вызов) — нарушение;
//   [R2] ключи — только из allowlist, без повторов;
//   [R3] точное соответствие: '/' и '/requests' → RequestsPage,
//        '/devices' → EmployeeDevicesPage, '/access-denied' и '*' → AccessDenied;
//   [R4] '*' — последняя запись (svelte-spa-router сопоставляет по порядку:
//        записи после '*' недостижимы);
//   [R5] импорт EmployeeDevicesPage / RequestsPage / AccessDenied указывает на
//        ожидаемый файл, и этот файл существует.
// --selftest: мутанты РЕАЛЬНОГО routes.ts обязаны краснеть, чистый — зеленеть.
//
// Usage: node scripts/check-employee-routes.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-employee-routes]';
const ROUTES_FILE = path.join(UI_ROOT, 'src/routes.ts');

const EXPECTED = [
  ['/', 'RequestsPage'],
  ['/requests', 'RequestsPage'],
  ['/devices', 'EmployeeDevicesPage'],
  ['/access-denied', 'AccessDenied'],
  ['*', 'AccessDenied'],
];
const ALLOWLIST = new Set(EXPECTED.map(([k]) => k));
const IMPORTS = {
  EmployeeDevicesPage: './features/employee/EmployeeDevicesPage.svelte',
  RequestsPage: './pages/RequestsPage.svelte',
  AccessDenied: './pages/AccessDenied.svelte',
};

function extractBalanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === open) depth++;
    else if (src[i] === close) {
      depth--;
      if (depth === 0) return src.slice(openIdx + 1, i);
    }
  }
  return null;
}

/** Тело `const <name> = { ... }` или null. Имя ищется целым словом. */
function objectLiteralBody(code, name) {
  const m = new RegExp(`\\bconst\\s+${name}\\b\\s*(?::[^=]*)?=\\s*\\{`).exec(code);
  if (!m) return null;
  return extractBalanced(code, m.index + m[0].length - 1, '{', '}');
}

/** Разбивка тела объекта по запятым верхнего уровня. */
function topLevelSegments(body) {
  const out = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if ('{[('.includes(c)) depth++;
    else if ('}])'.includes(c)) depth--;
    else if (c === ',' && depth === 0) {
      out.push(body.slice(start, i));
      start = i + 1;
    }
  }
  out.push(body.slice(start));
  return out.map((s) => s.trim()).filter((s) => s.length > 0);
}

/** Чистая функция: исходник routes.ts + проверка существования файла → нарушения. */
function scan(routesRaw, fileExists) {
  const v = [];
  const code = stripComments(routesRaw);
  const body = objectLiteralBody(code, 'employeeRoutes');
  if (body === null) {
    v.push('[R1] не найден объект `employeeRoutes` — гейт не может проверить маршруты');
    return v;
  }
  const entries = [];
  for (const seg of topLevelSegments(body)) {
    const m = /^(['"])([^'"]*)\1\s*:\s*([A-Za-z_$][\w$]*)$/.exec(seg);
    if (!m) {
      v.push(`[R1] запись не вида 'ключ': Компонент — «${seg.replace(/\s+/g, ' ')}»`);
      continue;
    }
    entries.push({ key: m[2], comp: m[3] });
  }

  const seen = new Map();
  for (const e of entries) {
    if (!ALLOWLIST.has(e.key))
      v.push(`[R2] лишний ключ '${e.key}' в employeeRoutes — сотрудник получил бы ${e.comp}`);
    if (seen.has(e.key)) v.push(`[R2] ключ '${e.key}' повторяется`);
    seen.set(e.key, e.comp);
  }

  for (const [key, comp] of EXPECTED) {
    if (!seen.has(key)) v.push(`[R3] нет записи '${key}' → ${comp}`);
    else if (seen.get(key) !== comp) v.push(`[R3] '${key}' → ${seen.get(key)}, ожидается ${comp}`);
  }

  if (entries.length > 0 && entries[entries.length - 1].key !== '*')
    v.push("[R4] '*' не последняя запись — маршруты после неё недостижимы или '*' нет");

  for (const [comp, rel] of Object.entries(IMPORTS)) {
    const im = new RegExp(`\\bimport\\s+${comp}\\s+from\\s+(['"])([^'"]+)\\1`).exec(code);
    if (!im) {
      v.push(`[R5] нет импорта ${comp}`);
      continue;
    }
    if (im[2] !== rel) v.push(`[R5] ${comp} импортируется из '${im[2]}', ожидается '${rel}'`);
    if (!fileExists(im[2])) v.push(`[R5] файл импорта ${comp} не существует: ${im[2]}`);
  }
  return v;
}

const realExists = (rel) => fs.existsSync(path.resolve(path.dirname(ROUTES_FILE), rel));

function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(
      `${TAG} SELFTEST FAIL - якорь мутанта «${label}» встречается ${n} раз, нужен ровно 1`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

function selftest() {
  const src = fs.readFileSync(ROUTES_FILE, 'utf8');
  const clean = scan(src, realExists);
  if (clean.length) {
    console.error(`${TAG} SELFTEST FAIL - чистый routes.ts красный:\n${clean.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} чистый routes.ts: PASS`);

  const DEV = "  '/devices': EmployeeDevicesPage,\n";
  const STAR = "  '*': AccessDenied,\n} as const;";
  const IMPORT_DEV =
    "import EmployeeDevicesPage from './features/employee/EmployeeDevicesPage.svelte';";
  const mutants = [
    ["удалён '/devices'", mutate(src, DEV, '', 'rm-devices'), /\[R3\] нет записи '\/devices'/],
    [
      "'/devices' → DevicesPage (страница персонала)",
      mutate(src, DEV, "  '/devices': DevicesPage,\n", 'devices-staff'),
      /\[R3\] '\/devices' → DevicesPage/,
    ],
    [
      "добавлен '/groups' (маршрут персонала)",
      mutate(src, DEV, `${DEV}  '/groups': GroupsPage,\n`, 'add-groups'),
      /\[R2\] лишний ключ '\/groups'/,
    ],
    [
      "добавлен '/devices-admin'",
      mutate(src, DEV, `${DEV}  '/devices-admin': DevicesPage,\n`, 'add-devices-admin'),
      /\[R2\] лишний ключ '\/devices-admin'/,
    ],
    [
      "'*' → NotFound вместо AccessDenied",
      mutate(src, STAR, "  '*': NotFound,\n} as const;", 'star-notfound'),
      /\[R3\] '\*' → NotFound/,
    ],
    [
      "'*' → Dashboard",
      mutate(src, STAR, "  '*': Dashboard,\n} as const;", 'star-dashboard'),
      /\[R3\] '\*' → Dashboard/,
    ],
    ["удалён '*'", mutate(src, STAR, '} as const;', 'rm-star'), /\[R3\] нет записи '\*'/],
    [
      "'/devices' спрятан в строчный комментарий",
      mutate(src, DEV, "  // '/devices': EmployeeDevicesPage,\n", 'comment-line'),
      /\[R3\] нет записи '\/devices'/,
    ],
    [
      "'/devices' спрятан в блочный комментарий",
      mutate(src, DEV, "  /* '/devices': EmployeeDevicesPage, */\n", 'comment-block'),
      /\[R3\] нет записи '\/devices'/,
    ],
    [
      'spread маршрутов персонала в employeeRoutes',
      mutate(src, DEV, `${DEV}  ...routes,\n`, 'spread'),
      /\[R1\]/,
    ],
    [
      "'*' не последняя",
      mutate(
        mutate(src, STAR, '} as const;', 'star-move-1'),
        "  '/': RequestsPage,\n  '/requests'",
        "  '*': AccessDenied,\n  '/': RequestsPage,\n  '/requests'",
        'star-move-2',
      ),
      /\[R4\]/,
    ],
    [
      'EmployeeDevicesPage импортируется из несуществующего файла',
      mutate(
        src,
        IMPORT_DEV,
        "import EmployeeDevicesPage from './features/employee/NoSuchPage.svelte';",
        'import-missing',
      ),
      /\[R5\] файл импорта EmployeeDevicesPage не существует/,
    ],
    [
      'EmployeeDevicesPage — это страница персонала',
      mutate(
        src,
        IMPORT_DEV,
        "import EmployeeDevicesPage from './features/devices/DevicesPage.svelte';",
        'import-staff',
      ),
      /\[R5\] EmployeeDevicesPage импортируется из/,
    ],
    [
      'импорт EmployeeDevicesPage закомментирован',
      mutate(src, IMPORT_DEV, `// ${IMPORT_DEV}`, 'import-commented'),
      /\[R5\] нет импорта EmployeeDevicesPage/,
    ],
  ];

  let bad = 0;
  for (const [name, mutated, expect] of mutants) {
    const v = scan(mutated, realExists);
    if (v.some((line) => expect.test(line))) console.log(`${TAG} ПОЙМАН: ${name}`);
    else {
      console.error(
        `${TAG} SELFTEST FAIL - мутант «${name}» не пойман правилом ${expect} (нарушения: ${v.length})`,
      );
      for (const line of v) console.error(`${TAG}   ${line}`);
      bad++;
    }
  }
  if (bad) process.exit(1);
  console.log(`${TAG} SELFTEST PASS - ${mutants.length} мутантов пойманы`);
}

function main() {
  if (process.argv.includes('--selftest')) return selftest();
  const v = scan(fs.readFileSync(ROUTES_FILE, 'utf8'), realExists);
  if (v.length) {
    console.error(`${TAG} FAIL\n${v.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}

main();
