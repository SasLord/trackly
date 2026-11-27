#!/usr/bin/env node
// [check-link-section] Гейт раздела «Link» в Trackly (фаза 44, план 18; D-05, D-12).
//
// Что рушится молча (svelte-check/eslint/build зелёные): пункт «Link» показан не
// админу, маршрут /link попал в карту сотрудника, нативный <select> вместо Dropdown,
// Modal внутри ячейки таблицы (наследует white-space/text-align td), литеральные цвета.
//
// Правила (по тексту без комментариев):
//   [S1] sidebar-config: запись '/link' с roles ['admin'], стоит ПОСЛЕ '/users' и ДО
//        ближайшего разделителя;
//   [S2] '/link' есть в routes и НЕТ в employeeRoutes;
//   [S3] в ui/src/features/link нет нативного <select и импорта Select.svelte;
//        InviteModal использует Dropdown с variant="select";
//   [S4] <Modal нет в LoginsTab/InvitesTab и нет внутри незакрытых <td / <TableRow;
//        подтверждение отзыва входа и попап кода рендерятся в LinkPage;
//   [S6] UserFormModal: секция «Отозвать входы Link» + вызов users_revoke_link/userRevoke,
//        без вложенного <Modal (подтверждение только инлайн);
//   [S5] нет литеральных цветов (#hex, rgb(, hsl() в разметке и <style>.
// --selftest: мутанты РЕАЛЬНЫХ файлов обязаны краснеть, чистые — зеленеть.
//
// Usage: node scripts/check-link-section.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-link-section]';
const SIDEBAR = 'src/features/layout/sidebar-config.ts';
const ROUTES = 'src/routes.ts';
const LINK_DIR = 'src/features/link';
const LINK_FILES = [
  'LinkPage.svelte',
  'LoginsTab.svelte',
  'InvitesTab.svelte',
  'InviteModal.svelte',
  'linkApi.ts',
].map((f) => `${LINK_DIR}/${f}`);
const USER_FORM = 'src/features/users/UserFormModal.svelte';
const TAB_FILES = new Set([`${LINK_DIR}/LoginsTab.svelte`, `${LINK_DIR}/InvitesTab.svelte`]);

function balanced(src, openIdx, open, close) {
  let depth = 0;
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === open) depth++;
    else if (src[i] === close && --depth === 0) return src.slice(openIdx + 1, i);
  }
  return null;
}

function objectBody(code, name) {
  const m = new RegExp(`\\bconst\\s+${name}\\b\\s*(?::[^=]*)?=\\s*\\{`).exec(code);
  return m ? balanced(code, m.index + m[0].length - 1, '{', '}') : null;
}

/** @param {Map<string,string>} files путь (от ui/) -> исходник */
export function scan(files) {
  const v = [];
  const get = (p) => files.get(p);

  // S1
  const sb = get(SIDEBAR);
  if (sb === undefined) v.push(`[S1] нет ${SIDEBAR}`);
  else {
    const code = stripComments(sb);
    const lines = code.split('\n').filter((l) => /\bkind:\s*'(item|divider)'/.test(l));
    const iUsers = lines.findIndex((l) => /route:\s*'\/users'/.test(l));
    const iLink = lines.findIndex((l) => /route:\s*'\/link'/.test(l));
    if (iLink < 0) v.push("[S1] нет пункта сайдбара '/link'");
    else {
      if (!/roles:\s*\['admin'\]/.test(lines[iLink]))
        v.push("[S1] пункт '/link' виден не только администратору");
      if (iUsers < 0 || iLink !== iUsers + 1)
        v.push("[S1] пункт '/link' должен стоять сразу после '/users'");
    }
  }

  // S2
  const rt = get(ROUTES);
  if (rt === undefined) v.push(`[S2] нет ${ROUTES}`);
  else {
    const code = stripComments(rt);
    const admin = objectBody(code, 'routes');
    const emp = objectBody(code, 'employeeRoutes');
    if (admin === null || emp === null) v.push('[S2] не найден routes/employeeRoutes');
    else {
      if (!/['"]\/link['"]\s*:\s*LinkPage\b/.test(admin))
        v.push("[S2] '/link' → LinkPage нет в routes");
      if (/\/link\b/.test(emp)) v.push("[S2] '/link' попал в employeeRoutes");
    }
  }

  // S3-S5 по файлам раздела
  let seenFiles = 0;
  for (const p of LINK_FILES) {
    const raw = get(p);
    if (raw === undefined) {
      v.push(`[S3] нет файла ${p}`);
      continue;
    }
    seenFiles++;
    const code = stripComments(raw);
    if (/<select\b/i.test(code) || /Select\.svelte/.test(code))
      v.push(`[S3] ${p}: нативный <select> или Select.svelte`);
    if (/#[0-9a-fA-F]{3,8}\b(?![\w-])/.test(code.replace(/&#\d+;/g, '')) && /<style/.test(code))
      v.push(`[S5] ${p}: hex-цвет`);
    if (/\b(?:rgba?|hsla?)\s*\(/.test(code)) v.push(`[S5] ${p}: rgb/hsl-литерал`);
    if (p.endsWith('.svelte')) {
      const modals = [...code.matchAll(/<Modal\b/g)];
      if (modals.length > 0 && TAB_FILES.has(p))
        v.push(`[S4] ${p}: <Modal во вкладке — рендерить в LinkPage`);
      for (const m of modals) {
        const before = code.slice(0, m.index);
        const open = (before.match(/<(?:td|TableRow)\b/g) ?? []).length;
        const close = (before.match(/<\/(?:td|TableRow)>/g) ?? []).length;
        if (open > close) v.push(`[S4] ${p}: <Modal внутри <td/<TableRow`);
      }
    }
  }
  if (seenFiles === 0) v.push('[S3] ни одного файла раздела — вакуумный проход запрещён');
  const im = get(`${LINK_DIR}/InviteModal.svelte`);
  if (im !== undefined) {
    const code = stripComments(im);
    if (!/<Dropdown\b[^>]*variant="select"/.test(code))
      v.push('[S3] InviteModal: срок действия не на Dropdown variant="select"');
  }
  const lp = get(`${LINK_DIR}/LinkPage.svelte`);
  if (lp !== undefined && !/<Modal\b|<InviteModal\b/.test(stripComments(lp)))
    v.push('[S4] LinkPage: нет Modal/InviteModal на уровне страницы');

  // S6
  const uf = get(USER_FORM);
  if (uf === undefined) v.push(`[S6] нет ${USER_FORM}`);
  else {
    const code = stripComments(uf);
    if (!/Отозвать входы Link/.test(code))
      v.push('[S6] UserFormModal: нет кнопки «Отозвать входы Link»');
    if (!/users_revoke_link|userRevoke\b/.test(code))
      v.push('[S6] UserFormModal: нет вызова users_revoke_link');
    if ((code.match(/<Modal\b/g) ?? []).length > 1)
      v.push('[S6] UserFormModal: вложенный <Modal (подтверждение только инлайн)');
  }
  return v;
}

function loadReal() {
  const m = new Map();
  for (const p of [SIDEBAR, ROUTES, USER_FORM, ...LINK_FILES]) {
    const abs = path.join(UI_ROOT, p);
    if (fs.existsSync(abs)) m.set(p, fs.readFileSync(abs, 'utf8'));
  }
  return m;
}

function mutate(files, p, from, to, label) {
  const src = files.get(p);
  const n = src === undefined ? 0 : src.split(from).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL - якорь «${label}» встречается ${n} раз, нужен ровно 1`);
    process.exit(1);
  }
  const out = new Map(files);
  out.set(
    p,
    src.replace(from, () => to),
  );
  return out;
}

function selftest() {
  const real = loadReal();
  const clean = scan(real);
  if (clean.length) {
    console.error(`${TAG} SELFTEST FAIL - чистое состояние красное:\n${clean.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} чистое состояние: PASS`);
  const LINK_ITEM = "{ kind: 'item', route: '/link', label: 'Link', phase: 44, roles: ['admin'] }";
  const mutants = [
    [
      'пункт Link виден всем',
      mutate(
        real,
        SIDEBAR,
        LINK_ITEM,
        "{ kind: 'item', route: '/link', label: 'Link' }",
        'sb-roles',
      ),
      /\[S1\].*не только администратору/,
    ],
    [
      'пункт Link удалён',
      mutate(real, SIDEBAR, `  ${LINK_ITEM},\n`, '', 'sb-rm'),
      /\[S1\] нет пункта/,
    ],
    [
      'пункт Link в строчном комментарии',
      mutate(real, SIDEBAR, `  ${LINK_ITEM},`, `  // ${LINK_ITEM},`, 'sb-comment'),
      /\[S1\] нет пункта/,
    ],
    [
      '/link в employeeRoutes',
      mutate(
        real,
        ROUTES,
        "  '/devices': EmployeeDevicesPage,\n",
        "  '/devices': EmployeeDevicesPage,\n  '/link': LinkPage,\n",
        'emp-link',
      ),
      /\[S2\].*employeeRoutes/,
    ],
    [
      '/link убран из routes',
      mutate(real, ROUTES, "  '/link': LinkPage,\n", '', 'routes-rm'),
      /\[S2\].*нет в routes/,
    ],
    [
      'нативный select в InviteModal',
      mutate(
        real,
        `${LINK_DIR}/InviteModal.svelte`,
        '<Dropdown',
        '<select></select><Dropdown',
        'native-select',
      ),
      /\[S3\].*нативный/,
    ],
    [
      'Modal во вкладке «Входы»',
      mutate(
        real,
        `${LINK_DIR}/LoginsTab.svelte`,
        '<script lang="ts">',
        '<Modal open={false} title="x" onClose={() => {}} />\n<script lang="ts">',
        'modal-tab',
      ),
      /\[S4\].*во вкладке/,
    ],
    [
      'hex-цвет в стиле',
      mutate(
        real,
        `${LINK_DIR}/LinkPage.svelte`,
        '<style lang="scss">',
        '<style lang="scss">\n  .zz { color: #ff0000; }',
        'hex',
      ),
      /\[S5\].*hex/,
    ],
    [
      'UserFormModal без отзыва входов',
      mutate(real, USER_FORM, 'userRevoke', 'userRevokeX', 'uf-revoke'),
      /\[S6\].*users_revoke_link/,
    ],
    [
      'вложенный Modal в UserFormModal',
      mutate(
        real,
        USER_FORM,
        '{#snippet footer()}',
        '<Modal open={false} title="x" onClose={() => {}} />\n  {#snippet footer()}',
        'uf-modal',
      ),
      /\[S6\].*вложенный/,
    ],
  ];
  let bad = 0;
  for (const [label, files, re] of mutants) {
    const out = scan(files);
    if (out.some((l) => re.test(l))) console.log(`${TAG} мутант «${label}»: краснеет (OK)`);
    else {
      console.error(
        `${TAG} SELFTEST FAIL - мутант «${label}» не пойман; вывод: ${out.join(' | ') || 'пусто'}`,
      );
      bad++;
    }
  }
  if (bad) process.exit(1);
  console.log(`${TAG} SELFTEST PASS (${mutants.length} мутантов)`);
}

if (process.argv.includes('--selftest')) selftest();
else {
  const v = scan(loadReal());
  if (v.length) {
    console.error(`${TAG} FAIL\n${v.join('\n')}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}
