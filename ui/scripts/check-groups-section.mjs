#!/usr/bin/env node
// [check-groups-section] Постоянный гейт раздела «Группы» (фаза 41, план 41-23,
// GRP-04 / GRP-09): запись сайдбара, маршрут и пометка PINNED не разъезжаются.
//
// Почему он существует: SPEC 6 — раздел «Группы» виден только admin и manager,
// а employee по прямому адресу `#/groups` получает «Доступ запрещён». Скрытие
// пункта (`roles` в sidebar-config.ts) и ОТСУТСТВИЕ маршрута в `employeeRoutes`
// (routes.ts) — две независимые правки в двух файлах; убрать любую можно,
// не сломав ни `svelte-check`, ни `eslint`, ни `pnpm build` (компиляционные
// гейты слепы к такому: оба файла остаются валидными). Кроме того, пометка
// `PINNED: N items + M dividers = K entries` в sidebar-config.ts уже один раз
// устарела молча — 12+4 осталось в комментарии, когда записей стало больше.
// JS-тест-раннера в проекте нет, поэтому закрепляем структурным гейтом (как
// check-device-form-quantity-gate.mjs) со встроенным self-testом.
//
// Правила:
//   (A) в SIDEBAR_ITEMS ровно одна запись route '/groups', она стоит между
//       записями '/devices' и '/acts';
//   (B) её roles — ровно ['admin', 'manager'] (без 'employee', без пропуска);
//   (C) числа в комментарии PINNED («N items + M dividers = K entries»)
//       совпадают с фактическим числом записей kind:'item' / kind:'divider'
//       и сумма N+M равна K (комментарий читается из СЫРОГО текста, остальное —
//       из текста без комментариев);
//   (D) в routes.ts '/groups' есть в `routes` (значение GroupsPage) и
//       отсутствует в `employeeRoutes`.
//
// Self-test (`--selftest`): 6 фикстур в памяти — 1 корректная (должна пройти)
// и 5 негативных, каждая получена из корректной мутацией с ПРОВЕРКОЙ, что
// якорь мутации встречается ровно один раз (иначе мутант мог бы снять гейт у
// соседней записи и тест «прошёл бы» зря). Негативная фикстура обязана
// провалить гейт именно ТЕМ правилом, ради которого написана.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-groups-section.mjs              # проверить репозиторий
//   node scripts/check-groups-section.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-groups-section.mjs --selftest   # встроенный self-test (in-memory)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-groups-section]';

// Проверяемые файлы (POSIX-пути от корня ui/).
const SIDEBAR_FILE = 'src/features/layout/sidebar-config.ts';
const ROUTES_FILE = 'src/routes.ts';

const GROUPS_ROUTE = '/groups';

/**
 * Затирает JS-комментарии (строчные и блочные), не трогая строковые литералы;
 * длина и переводы строк сохраняются. Нужно: комментарии в обоих файлах
 * буквально цитируют проверяемые формы (`'/groups'`, «13 items»), и без
 * вычистки гейт «проходил» бы по комментарию при удалённом коде.
 */
function stripComments(src) {
  let out = '';
  let i = 0;
  let mode = 'code'; // 'code' | 'line' | 'block' | "'" | '"' | '`'
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
    // строковый литерал
    if (c === '\\') {
      out += c + (next ?? '');
      i += 2;
      continue;
    }
    if (c === mode) mode = 'code';
    out += c;
    i += 1;
  }
  return out;
}

/** Содержимое парных скобок, начиная с индекса открывающей. */
function extractBalanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return src.slice(openIdx + 1, i);
    }
  }
  return null;
}

/** Верхнеуровневые `{ ... }` внутри тела массива. */
function topLevelObjects(body) {
  const out = [];
  let depth = 0;
  let start = -1;
  for (let i = 0; i < body.length; i++) {
    if (body[i] === '{') {
      if (depth === 0) start = i;
      depth++;
    } else if (body[i] === '}') {
      depth--;
      if (depth === 0 && start !== -1) {
        out.push(body.slice(start, i + 1));
        start = -1;
      }
    }
  }
  return out;
}

/** Записи SIDEBAR_ITEMS: [{kind, route, roles}] или null, если массив не разобрать. */
function parseSidebarEntries(code) {
  const nameIdx = code.indexOf('SIDEBAR_ITEMS');
  if (nameIdx === -1) return null;
  const eqIdx = code.indexOf('=', nameIdx);
  if (eqIdx === -1) return null;
  const openIdx = code.indexOf('[', eqIdx);
  if (openIdx === -1) return null;
  const body = extractBalanced(code, openIdx, '[', ']');
  if (body === null) return null;
  return topLevelObjects(body).map((obj) => {
    const kind = /kind\s*:\s*'(item|divider)'/.exec(obj)?.[1] ?? null;
    const route = /route\s*:\s*'([^']*)'/.exec(obj)?.[1] ?? null;
    const rolesRaw = /roles\s*:\s*\[([^\]]*)\]/.exec(obj)?.[1];
    const roles =
      rolesRaw === undefined ? null : [...rolesRaw.matchAll(/'([^']*)'/g)].map((m) => m[1]);
    return { kind, route, roles };
  });
}

/** Тело `export const <name> = { ... }` или null. */
function objectLiteralBody(code, name) {
  const idx = code.indexOf(`const ${name}`);
  if (idx === -1) return null;
  const openIdx = code.indexOf('{', code.indexOf('=', idx));
  if (openIdx === -1) return null;
  return extractBalanced(code, openIdx, '{', '}');
}

/**
 * Чистая функция сканирования: принимает уже прочитанные исходники, не трогает
 * fs. Переиспользуется и реальным сканированием, и self-testом.
 */
function scanSources(sidebarRaw, routesRaw, violations) {
  const sidebarCode = stripComments(sidebarRaw);
  const routesCode = stripComments(routesRaw);

  // --- (A)/(B) запись '/groups' --------------------------------------------
  const entries = parseSidebarEntries(sidebarCode);
  if (entries === null || entries.length === 0) {
    violations.push(
      `${SIDEBAR_FILE}: не разобрать массив SIDEBAR_ITEMS — гейт не может проверить раздел. ` +
        `Если массив переименован, обнови гейт вместе с ним.`,
    );
  } else {
    const idxOf = (route) =>
      entries
        .map((e, i) => (e.kind === 'item' && e.route === route ? i : -1))
        .filter((i) => i >= 0);
    const groupsIdx = idxOf(GROUPS_ROUTE);
    const devicesIdx = idxOf('/devices');
    const actsIdx = idxOf('/acts');

    if (groupsIdx.length === 0) {
      violations.push(
        `${SIDEBAR_FILE}: запись route '${GROUPS_ROUTE}' отсутствует в SIDEBAR_ITEMS — ` +
          `раздел «Группы» пропал из сайдбара (GRP-04).`,
      );
    } else if (groupsIdx.length > 1) {
      violations.push(
        `${SIDEBAR_FILE}: запись route '${GROUPS_ROUTE}' встречается ${groupsIdx.length} раз — ожидается одна.`,
      );
    } else if (devicesIdx.length !== 1 || actsIdx.length !== 1) {
      violations.push(
        `${SIDEBAR_FILE}: не найдены ровно по одной записи '/devices' и '/acts' — ` +
          `гейт не может проверить позицию раздела «Группы».`,
      );
    } else if (!(devicesIdx[0] < groupsIdx[0] && groupsIdx[0] < actsIdx[0])) {
      violations.push(
        `${SIDEBAR_FILE}: запись '${GROUPS_ROUTE}' (позиция ${groupsIdx[0] + 1}) должна стоять ` +
          `МЕЖДУ '/devices' (${devicesIdx[0] + 1}) и '/acts' (${actsIdx[0] + 1}) (SPEC 6).`,
      );
    }

    if (groupsIdx.length === 1) {
      const roles = entries[groupsIdx[0]].roles;
      const sorted = roles === null ? null : [...roles].sort().join(',');
      if (roles === null) {
        violations.push(
          `${SIDEBAR_FILE}: у записи '${GROUPS_ROUTE}' нет roles — раздел виден ВСЕМ, включая employee (GRP-09).`,
        );
      } else if (roles.includes('employee')) {
        violations.push(
          `${SIDEBAR_FILE}: roles записи '${GROUPS_ROUTE}' содержит 'employee' — сотрудник не должен видеть раздел (GRP-09).`,
        );
      } else if (sorted !== 'admin,manager') {
        violations.push(
          `${SIDEBAR_FILE}: roles записи '${GROUPS_ROUTE}' = [${roles.join(', ')}], ожидается ровно ['admin', 'manager'].`,
        );
      }
    }

    // --- (C) пометка PINNED ---------------------------------------------------
    const pinned = /PINNED:\s*(\d+)\s+items\s*\+\s*(\d+)\s+dividers\s*=\s*(\d+)\s+entries/.exec(
      sidebarRaw,
    );
    const actualItems = entries.filter((e) => e.kind === 'item').length;
    const actualDividers = entries.filter((e) => e.kind === 'divider').length;
    if (!pinned) {
      violations.push(
        `${SIDEBAR_FILE}: не найден комментарий «PINNED: N items + M dividers = K entries».`,
      );
    } else {
      const [n, m, k] = [Number(pinned[1]), Number(pinned[2]), Number(pinned[3])];
      if (n + m !== k) {
        violations.push(`${SIDEBAR_FILE}: PINNED противоречит сам себе: ${n} + ${m} ≠ ${k}.`);
      }
      if (n !== actualItems || m !== actualDividers || k !== entries.length) {
        violations.push(
          `${SIDEBAR_FILE}: PINNED устарел — в комментарии ${n} items + ${m} dividers = ${k} entries, ` +
            `фактически ${actualItems} + ${actualDividers} = ${entries.length}.`,
        );
      }
    }
  }

  // --- (D) маршруты ---------------------------------------------------------
  const routesBody = objectLiteralBody(routesCode, 'routes');
  const employeeBody = objectLiteralBody(routesCode, 'employeeRoutes');
  if (routesBody === null || employeeBody === null) {
    violations.push(
      `${ROUTES_FILE}: не разобрать \`routes\` / \`employeeRoutes\` — гейт не может проверить доступ.`,
    );
  } else {
    if (!/['"]\/groups['"]\s*:\s*GroupsPage\b/.test(routesBody)) {
      violations.push(
        `${ROUTES_FILE}: в \`routes\` нет записи '${GROUPS_ROUTE}': GroupsPage — раздел недоступен (GRP-04).`,
      );
    }
    if (/['"]\/groups['"]/.test(employeeBody)) {
      violations.push(
        `${ROUTES_FILE}: '${GROUPS_ROUTE}' добавлен в \`employeeRoutes\` — employee получил бы раздел вместо «Доступ запрещён» (GRP-09).`,
      );
    }
  }
}

function checkRepo(srcRoot, violations) {
  const read = (rel) => {
    const abs = path.join(srcRoot, rel);
    if (!fs.existsSync(abs)) {
      violations.push(`${rel}: файл не найден (${abs}).`);
      return null;
    }
    return fs.readFileSync(abs, 'utf8');
  };
  const sidebar = read(SIDEBAR_FILE);
  const routes = read(ROUTES_FILE);
  if (sidebar === null || routes === null) return;
  scanSources(sidebar, routes, violations);
}

// ---------------------------------------------------------------------------
// Self-test
// ---------------------------------------------------------------------------

const CLEAN_SIDEBAR = `
// PINNED: 13 items + 4 dividers = 17 entries — source of truth.
// Старая запись: { kind: 'item', route: '/groups' } — комментарий, не код.
export const SIDEBAR_ITEMS: SidebarEntry[] = [
  { kind: 'item', route: '/', label: 'Дашборд' },
  { kind: 'item', route: '/map', label: 'Карта' },
  { kind: 'item', route: '/places', label: 'Места', roles: ['admin', 'manager'] },
  { kind: 'divider' },
  { kind: 'item', route: '/devices', label: 'Устройства' },
  { kind: 'item', route: '/groups', label: 'Группы', roles: ['admin', 'manager'] },
  { kind: 'item', route: '/acts', label: 'Акты' },
  { kind: 'divider' },
  { kind: 'item', route: '/printers', label: 'Принтеры' },
  { kind: 'item', route: '/cartridges', label: 'Картриджи' },
  { kind: 'item', route: '/requests', label: 'Заявки' },
  { kind: 'divider' },
  { kind: 'item', route: '/reports', label: 'Отчёты' },
  { kind: 'item', route: '/users', label: 'Пользователи', roles: ['admin'] },
  { kind: 'divider' },
  { kind: 'item', route: '/settings', label: 'Настройки', roles: ['admin'] },
  { kind: 'item', route: '/showcase', label: 'Витрина', roles: ['admin'] },
];
`;

const CLEAN_ROUTES = `
export const routes = {
  '/': Dashboard,
  '/devices': DevicesPage,
  '/groups': GroupsPage,
  '/acts': ActsPage,
  '*': NotFound,
} as const;

// В employeeRoutes '/groups' не добавляется — комментарий не должен ломать гейт.
export const employeeRoutes = {
  '/': RequestsPage,
  '/requests': RequestsPage,
  '*': AccessDenied,
} as const;
`;

const GROUPS_ENTRY_RE_SRC =
  "  { kind: 'item', route: '/groups', label: 'Группы', roles: ['admin', 'manager'] },\n";

/** Мутация с проверкой уникальности якоря: иначе мутант мог бы задеть соседа. */
function mutate(src, from, to) {
  const count = src.split(from).length - 1;
  if (count !== 1) {
    throw new Error(`якорь мутации встречается ${count} раз(а), ожидалось ровно 1: ${from.trim()}`);
  }
  return src.replace(from, to);
}

function runSelfTest() {
  const fixtures = [];
  try {
    fixtures.push({
      name: 'корректные sidebar-config и routes (позитив)',
      sidebar: CLEAN_SIDEBAR,
      routes: CLEAN_ROUTES,
      expectViolations: 0,
    });
    fixtures.push({
      name: '(а) записи /groups нет в сайдбаре',
      sidebar: mutate(CLEAN_SIDEBAR, GROUPS_ENTRY_RE_SRC, ''),
      routes: CLEAN_ROUTES,
      mention: /отсутствует в SIDEBAR_ITEMS/,
    });
    fixtures.push({
      name: '(б) запись /groups после /acts',
      sidebar: mutate(
        mutate(CLEAN_SIDEBAR, GROUPS_ENTRY_RE_SRC, ''),
        "  { kind: 'item', route: '/acts', label: 'Акты' },\n",
        "  { kind: 'item', route: '/acts', label: 'Акты' },\n" + GROUPS_ENTRY_RE_SRC,
      ),
      routes: CLEAN_ROUTES,
      mention: /МЕЖДУ/,
    });
    fixtures.push({
      name: "(в) roles записи /groups содержит 'employee'",
      sidebar: mutate(
        CLEAN_SIDEBAR,
        "label: 'Группы', roles: ['admin', 'manager']",
        "label: 'Группы', roles: ['admin', 'manager', 'employee']",
      ),
      routes: CLEAN_ROUTES,
      mention: /содержит 'employee'/,
    });
    fixtures.push({
      name: '(г) счётчик PINNED не совпадает с массивом',
      sidebar: mutate(
        CLEAN_SIDEBAR,
        'PINNED: 13 items + 4 dividers = 17 entries',
        'PINNED: 12 items + 4 dividers = 16 entries',
      ),
      routes: CLEAN_ROUTES,
      mention: /PINNED устарел/,
    });
    fixtures.push({
      name: "(д) '/groups' внутри employeeRoutes",
      sidebar: CLEAN_SIDEBAR,
      routes: mutate(
        CLEAN_ROUTES,
        "  '/requests': RequestsPage,\n",
        "  '/requests': RequestsPage,\n  '/groups': GroupsPage,\n",
      ),
      mention: /employeeRoutes/,
    });
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSources(f.sidebar, f.routes, v);
    let ok;
    let expected;
    if (f.expectViolations === 0) {
      ok = v.length === 0;
      expected = '0';
    } else {
      ok = v.length > 0 && v.some((line) => f.mention.test(line));
      expected = `≥1, с ${f.mention}`;
    }
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (expected ${expected}, got ${v.length})`,
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
      `${TAG} Usage: node scripts/check-groups-section.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  checkRepo(SRC_ROOT, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
