#!/usr/bin/env node
// [check-attachments-block] Статический гейт блока «Вложения» (фаза 41.3, план 13).
//
// Что держит (сообщение начинается с кода правила в квадратных скобках):
//   [G0] защита от пропуска: каждый обязательный файл присутствует в наборе;
//        отсутствующий файл — красный, а не «нечего проверять»;
//   [G1] нигде в features/attachments/*.svelte нет `{@html}` (имена файлов и людей
//        только интерполяцией; T-41.3-57);
//   [G2] AttachmentsBlock подписан через onWsEvent на 'attachments_changed' (D-15/D-25);
//   [G3] каждый `<AttachmentViewer` стоит на верхнем уровне, а не внутри тела
//        чужого `<Modal>` (просмотрщик — соседний Modal, не вложенный);
//   [G4] intakeAllowed сравнивает глубину host-Modal с самым верхним
//        (`data-modal-depth`, `hostDepth === topDepth`), а handlePaste и
//        handleDrop зовут intakeAllowed() РАНЬШЕ enqueue( — по порядку исходника;
//   [G5] в handlePaste охрана редактируемой цели (`input, textarea` и
//        `isContentEditable`) стоит РАНЬШЕ intakeAllowed() и enqueue( (T-41.3-58);
//   [G6] api.ts: uploadAttachment и fetchAttachmentBytes не идут через apiCall
//        (на HTTP-пути он портит бинарное тело);
//   [G8] слушатель paste на window подключается только при canAttach;
//   [G9] (план 14) в каждой из четырёх карточек `<AttachmentsBlock` стоит РАНЬШЕ
//        первого заголовка истории (заявка «История», принтер «История статусов»,
//        картридж «Журнал операций», просмотр места «История перемещений»);
//   [G10] (план 14) RequestDetail отбрасывает custom:attachment_* на клиенте и
//        берёт события вложений через attachments.history('request', ...);
//        CartridgeDetail тоже отбрасывает сырые custom:attachment_*.
//
// Счёт имён слеп к порядку, поэтому G4/G5 сравнивают ПОЗИЦИИ в теле функции,
// а самопроверка (--selftest) включает мутации-перестановки, не только удаление.
//
// Запуск: node scripts/check-attachments-block.mjs [--selftest]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const TAG = '[check-attachments-block]';
const HERE = path.dirname(fileURLToPath(import.meta.url));
const DIR = path.resolve(HERE, '../src/features/attachments');
const FEATURES = path.resolve(HERE, '../src/features');
// Карточки, во встраивание которых блок обязан попасть (план 14). Ключ в наборе
// файлов — `card:<имя>`, чтобы не путать с файлами каталога attachments.
const CARDS = {
  'card:RequestDetail.svelte': { rel: 'requests/RequestDetail.svelte', heading: 'История' },
  'card:PrinterDetail.svelte': {
    rel: 'printers/PrinterDetail.svelte',
    heading: 'История статусов',
  },
  'card:CartridgeDetail.svelte': {
    rel: 'cartridges/CartridgeDetail.svelte',
    heading: 'Журнал операций',
  },
  'card:PlaceEntityViewModal.svelte': {
    rel: 'places/PlaceEntityViewModal.svelte',
    heading: 'История перемещений',
  },
};
const REQUIRED = [
  'AttachmentsBlock.svelte',
  'AttachmentTile.svelte',
  'api.ts',
  ...Object.keys(CARDS),
];

/** Тело функции `name` (между фигурными скобками) либо null. */
function functionBody(text, name) {
  const m = new RegExp(`function\\s+${name}\\s*\\(`).exec(text);
  if (!m) return null;
  let i = m.index + m[0].length;
  let depth = 1;
  while (i < text.length && depth > 0) {
    if (text[i] === '(') depth++;
    else if (text[i] === ')') depth--;
    i++;
  }
  const open = text.indexOf('{', i);
  if (open === -1) return null;
  let d = 0;
  for (let j = open; j < text.length; j++) {
    if (text[j] === '{') d++;
    else if (text[j] === '}') {
      d--;
      if (d === 0) return text.slice(open + 1, j);
    }
  }
  return null;
}

/** Глубина вложенности `<Modal` ... `</Modal>` в позиции pos. */
function modalDepthAt(text, pos) {
  const re = /<Modal\b|<\/Modal>/g;
  let depth = 0;
  let m;
  while ((m = re.exec(text)) && m.index < pos) {
    if (m[0] === '</Modal>') depth--;
    else {
      // самозакрывающийся <Modal ... /> тела не имеет
      const end = text.indexOf('>', m.index);
      if (text[end - 1] !== '/') depth++;
    }
  }
  return depth;
}

/**
 * @param {Record<string,string>} files имя файла -> исходник
 * @returns {{code:string,msg:string}[]}
 */
export function check(files) {
  const v = [];
  for (const name of REQUIRED) {
    if (typeof files[name] !== 'string') {
      v.push({ code: 'G0', msg: `нет файла ${name} в наборе: гейту нечего проверять` });
    }
  }
  if (v.length > 0) return v;

  const src = {};
  for (const [name, text] of Object.entries(files)) src[name] = stripComments(text);

  for (const [name, text] of Object.entries(src)) {
    if (name.endsWith('.svelte') && !name.startsWith('card:') && /\{@html\b/.test(text)) {
      v.push({ code: 'G1', msg: `${name}: найден {@html}` });
    }
  }

  const block = src['AttachmentsBlock.svelte'];

  if (!/onWsEvent\s*\(/.test(block) || !/['"]attachments_changed['"]/.test(block)) {
    v.push({
      code: 'G2',
      msg: 'AttachmentsBlock не подписан на attachments_changed через onWsEvent',
    });
  }

  const viewerRe = /<AttachmentViewer\b/g;
  let vm;
  let viewers = 0;
  while ((vm = viewerRe.exec(block))) {
    viewers++;
    if (modalDepthAt(block, vm.index) !== 0) {
      v.push({ code: 'G3', msg: '<AttachmentViewer внутри тела другого <Modal>' });
    }
  }
  if (viewers === 0) v.push({ code: 'G3', msg: 'в AttachmentsBlock нет <AttachmentViewer' });

  const intake = functionBody(block, 'intakeAllowed');
  if (
    intake === null ||
    !intake.includes('data-modal-depth') ||
    !/hostDepth\s*===\s*topDepth/.test(intake)
  ) {
    v.push({
      code: 'G4',
      msg: 'intakeAllowed не сравнивает глубину host-Modal с верхним (data-modal-depth, hostDepth === topDepth)',
    });
  }
  for (const fn of ['handlePaste', 'handleDrop']) {
    const body = functionBody(block, fn);
    if (body === null) {
      v.push({ code: 'G4', msg: `нет функции ${fn}` });
      continue;
    }
    const guard = body.indexOf('intakeAllowed()');
    const enq = body.indexOf('enqueue(');
    if (guard === -1 || enq === -1 || guard > enq) {
      v.push({ code: 'G4', msg: `${fn}: intakeAllowed() должен стоять раньше enqueue(` });
    }
  }

  const paste = functionBody(block, 'handlePaste');
  if (paste !== null) {
    const a = paste.indexOf('input, textarea');
    const b = paste.indexOf('isContentEditable');
    const guard = paste.indexOf('intakeAllowed()');
    const enq = paste.indexOf('enqueue(');
    if (a === -1 || b === -1 || enq === -1 || Math.max(a, b) > guard || Math.max(a, b) > enq) {
      v.push({
        code: 'G5',
        msg: 'handlePaste: охрана редактируемой цели (input, textarea / isContentEditable) должна стоять раньше intakeAllowed() и enqueue(',
      });
    }
  }

  const api = src['api.ts'];
  for (const fn of ['uploadAttachment', 'fetchAttachmentBytes']) {
    const body = functionBody(api, fn);
    if (body === null) v.push({ code: 'G6', msg: `api.ts: нет функции ${fn}` });
    else if (/\bapiCall\b/.test(body))
      v.push({ code: 'G6', msg: `api.ts: ${fn} идёт через apiCall` });
  }

  if (!/onpaste=\{\s*canAttach\s*\?\s*handlePaste\s*:\s*undefined\s*\}/.test(block)) {
    v.push({ code: 'G8', msg: 'слушатель paste подключён не только при canAttach' });
  }

  for (const [key, card] of Object.entries(CARDS)) {
    const text = src[key];
    const at = text.indexOf('<AttachmentsBlock');
    const head = text.indexOf(`heading="${card.heading}"`);
    if (at === -1) {
      v.push({ code: 'G9', msg: `${card.rel}: нет <AttachmentsBlock` });
    } else if (head === -1 || at > head) {
      v.push({
        code: 'G9',
        msg: `${card.rel}: <AttachmentsBlock должен стоять раньше заголовка «${card.heading}»`,
      });
    }
  }

  const req = src['card:RequestDetail.svelte'];
  if (!/startsWith\(\s*['"]custom:attachment_['"]\s*\)/.test(req)) {
    v.push({ code: 'G10', msg: 'RequestDetail не отбрасывает custom:attachment_* на клиенте' });
  }
  if (!req.includes("attachments.history('request'")) {
    v.push({ code: 'G10', msg: "RequestDetail не зовёт attachments.history('request', ...)" });
  }
  if (!/startsWith\(\s*['"]custom:attachment_['"]\s*\)/.test(src['card:CartridgeDetail.svelte'])) {
    v.push({ code: 'G10', msg: 'CartridgeDetail не отбрасывает custom:attachment_* на клиенте' });
  }
  return v;
}

function readFiles(dir) {
  const out = {};
  for (const n of fs.readdirSync(dir)) {
    if (n.endsWith('.svelte') || n === 'api.ts')
      out[n] = fs.readFileSync(path.join(dir, n), 'utf8');
  }
  for (const [key, card] of Object.entries(CARDS)) {
    const file = path.join(FEATURES, card.rel);
    if (fs.existsSync(file)) out[key] = fs.readFileSync(file, 'utf8');
  }
  return out;
}

function once(text, anchor, name) {
  const n = text.split(anchor).length - 1;
  if (n !== 1) {
    console.error(`${TAG} SELFTEST FAIL — якорь «${name}» встречается ${n} раз (нужен ровно 1)`);
    process.exit(1);
  }
}

function mutate(files, file, anchor, replacement, name) {
  once(files[file], anchor, name);
  return { ...files, [file]: files[file].replace(anchor, () => replacement) };
}

function selftest(files) {
  const base = check(files);
  if (base.length > 0) {
    console.error(
      `${TAG} SELFTEST FAIL — реальные файлы красные: ${base.map((x) => `[${x.code}] ${x.msg}`).join('; ')}`,
    );
    process.exit(1);
  }
  const B = 'AttachmentsBlock.svelte';
  const mutants = [
    {
      name: 'удалена охрана редактируемой цели в paste',
      expect: 'G5',
      make: () =>
        mutate(
          files,
          B,
          "target.closest('input, textarea, [contenteditable]') || target.isContentEditable",
          'false',
          'paste guard',
        ),
    },
    {
      name: 'перестановка: enqueue раньше охраны в paste',
      expect: 'G5',
      make: () =>
        mutate(
          files,
          B,
          'function handlePaste(e: globalThis.ClipboardEvent) {',
          'function handlePaste(e: globalThis.ClipboardEvent) {\n    enqueue([]);',
          'paste reorder',
        ),
    },
    {
      name: 'перестановка: enqueue раньше intakeAllowed в drop',
      expect: 'G4',
      make: () =>
        mutate(
          files,
          B,
          'function handleDrop(e: globalThis.DragEvent) {',
          'function handleDrop(e: globalThis.DragEvent) {\n    enqueue([]);',
          'drop reorder',
        ),
    },
    {
      name: 'intakeAllowed больше не сравнивает глубину',
      expect: 'G4',
      make: () => mutate(files, B, 'return hostDepth === topDepth;', 'return true;', 'host depth'),
    },
    {
      name: 'добавлен {@html} в плитку',
      expect: 'G1',
      make: () =>
        mutate(
          files,
          'AttachmentTile.svelte',
          '<li class="tile" class:tile-missing={missing}>',
          '{@html item.originalName}<li class="tile" class:tile-missing={missing}>',
          'tile li',
        ),
    },
    {
      name: 'просмотрщик перенесён внутрь Modal',
      expect: 'G3',
      make: () =>
        mutate(
          files,
          B,
          '{#if confirmPurge}',
          '<Modal open={true} title="x" onClose={() => {}}><AttachmentViewer items={[]} startId={null} open={false} onClose={() => {}} /></Modal>{#if confirmPurge}',
          'viewer in modal',
        ),
    },
    {
      name: 'снята подписка на attachments_changed',
      expect: 'G2',
      make: () =>
        mutate(files, B, "ev.type === 'attachments_changed'", "ev.type === 'other'", 'ws type'),
    },
    {
      name: 'загрузка пошла через apiCall',
      expect: 'G6',
      make: () =>
        mutate(
          files,
          'api.ts',
          'const form = new FormData();',
          "void apiCall('attachments_upload', {});\n    const form = new FormData();",
          'api upload',
        ),
    },
    {
      name: 'paste подключён всегда',
      expect: 'G8',
      make: () =>
        mutate(
          files,
          B,
          'onpaste={canAttach ? handlePaste : undefined}',
          'onpaste={handlePaste}',
          'paste attach',
        ),
    },
    ...Object.entries(CARDS).map(([key, card]) => ({
      name: `блок переставлен после заголовка «${card.heading}» (${card.rel})`,
      expect: 'G9',
      make: () =>
        mutate(
          files,
          key,
          '<AttachmentsBlock',
          `<DetailSection heading="${card.heading}"></DetailSection><AttachmentsBlock`,
          `order ${key}`,
        ),
    })),
    {
      name: 'RequestDetail не фильтрует сырые custom:attachment_',
      expect: 'G10',
      make: () =>
        mutate(
          files,
          'card:RequestDetail.svelte',
          "!e.action.startsWith('custom:attachment_')",
          'true',
          'request filter',
        ),
    },
    {
      name: 'RequestDetail не берёт события через attachments.history',
      expect: 'G10',
      make: () =>
        mutate(
          files,
          'card:RequestDetail.svelte',
          "attachments.history('request', id)",
          "attachments.history('device', id)",
          'request history call',
        ),
    },
    {
      name: 'CartridgeDetail не фильтрует сырые custom:attachment_',
      expect: 'G10',
      make: () =>
        mutate(
          files,
          'card:CartridgeDetail.svelte',
          "!e.action.startsWith('custom:attachment_')",
          'true',
          'cartridge filter',
        ),
    },
    {
      name: 'пропал файл карточки (не «пропуск», а красный)',
      expect: 'G0',
      make: () => {
        const copy = { ...files };
        delete copy['card:PrinterDetail.svelte'];
        return copy;
      },
    },
    {
      name: 'пропал файл блока (не «пропуск», а красный)',
      expect: 'G0',
      make: () => {
        const copy = { ...files };
        delete copy[B];
        return copy;
      },
    },
  ];
  let bad = 0;
  for (const m of mutants) {
    const codes = check(m.make()).map((x) => x.code);
    const ok = codes.includes(m.expect);
    console.log(`${TAG} мутант «${m.name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${m.expect}]`);
    if (!ok) bad++;
  }
  if (bad > 0) process.exit(1);
  console.log(`${TAG} SELFTEST PASS — реальные файлы зелёные, ${mutants.length} мутантов красные`);
}

function main() {
  let files;
  try {
    files = readFiles(DIR);
  } catch {
    console.error(`${TAG} FAIL — [G0] каталог ${DIR} не читается`);
    process.exit(1);
  }
  if (process.argv.includes('--selftest')) return selftest(files);
  const violations = check(files);
  if (violations.length > 0) {
    for (const x of violations) console.error(`${TAG} FAIL — [${x.code}] ${x.msg}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS — 0 нарушений`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
