#!/usr/bin/env node
// [check-device-form-quantity-gate] Постоянный гейт инварианта NEW-3 (фаза
// 40.4): автоподставленный, НЕ тронутый руками инвентарный номер не блокирует
// «Количество», а массовое создание не пытается отправить этот номер на сервер.
//
// Почему он существует: CR-01 ревью 40.4 — `quantityDisabled` ослабили до
// `(inventoryNo.trim() !== '' && inventoryNoEditedByHand)`, но СОСЕДНИЙ
// `$effect`, сбрасывающий `quantity = 1`, оставили на старом условии
// `inventoryNo.trim() !== ''`. До 40.4 это было безвредно (поле «Количество»
// и так было заблокировано для любого непустого номера), после — каждая
// ПРОГРАММНАЯ запись в `inventoryNo` (автоподстановка при монтировании,
// повторный peek при смене «Устройство»↔«Принтер») молча возвращала
// «Количество» в 1, и вместо N устройств создавалось одно, без единого
// сообщения. То есть вся фича отменялась сама собой. Починено коммитом
// fae6c9d4; ничего не мешает внести это заново.
//
// Почему статический гейт, а не тест: `svelte-check` / `eslint` / `pnpm build`
// не видят рантайм-поведения рун (project memory
// `compile_gates_miss_svelte_runtime`), а синтетический Chromium-харнесс не
// считается верификацией WKWebView (`synthetic_harness_not_verification`).
// Единственное, что можно закрепить в CI навсегда — РЕАКТИВНАЯ РАЗВОДКА:
// два места не имеют права разъехаться.
//
// Три правила (все — по `ui/src/features/devices/DeviceFormBody.svelte`):
//   (A) в `const quantityDisabled = $derived(...)` условие по инвентарному
//       номеру обязано быть склеено с `inventoryNoEditedByHand` — голое
//       `inventoryNo.trim() !== ''` запрещено;
//   (B) КАЖДЫЙ `$effect`, присваивающий `quantity = 1`, обязан читать ТОТ ЖЕ
//       гейт `inventoryNoEditedByHand` — это в точности режим отказа CR-01;
//       присваивание `quantity = 1` вне `$effect` запрещено (гейт не смог бы
//       его проверить);
//   (C) ветка qty>1 в `handleSubmit` (объект `DeviceNew` перед
//       `devices.bulkCreate(`) обязана отправлять `inventory_no: null`
//       БЕЗУСЛОВНО — иначе сервер жёстко отклонит count>1 с непустым номером
//       (регресс-тест `bulk_create_rejects_when_inventory_no_set`).
//
// Все три правила проверяются встроенным self-testом (`--selftest`) на 9
// фикстурах в памяти (6 позитивных — включая буквальный до-fae6c9d4 вид
// эффекта; 3 негативных — текущая корректная разводка и её варианты), поэтому
// «гейт зелёный» отличимо от «гейт ничего не ищет».
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-device-form-quantity-gate.mjs              # проверить репозиторий
//   node scripts/check-device-form-quantity-gate.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-device-form-quantity-gate.mjs --selftest   # встроенный self-test (in-memory)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-device-form-quantity-gate]';

// Единственный проверяемый файл (POSIX-путь от корня ui/).
const TARGET_FILE = 'src/features/devices/DeviceFormBody.svelte';

// Голое условие «номер непустой» без гейта «правил руками».
const BARE_INVENTORY_COND = "inventoryNo.trim()!==''";
// Обе допустимые склейки с гейтом (порядок операндов любой).
const GATED_INVENTORY_CONDS = [
  "inventoryNo.trim()!==''&&inventoryNoEditedByHand",
  "inventoryNoEditedByHand&&inventoryNo.trim()!==''",
];

/**
 * Удаляет JS-комментарии (строчные и блочные), не трогая содержимое строковых
 * и шаблонных литералов. Нужно до нормализации пробелов: комментарии в этом
 * файле буквально цитируют запрещённые формы («до 40.4 было
 * `inventoryNo.trim() !== ''`»), и без вырезания гейт ловил бы сам себя.
 * HTML-комментарии `<!-- -->` оставляем: разметка проверяемых правил не несёт.
 */
function stripJsComments(src) {
  let out = '';
  let i = 0;
  // 'code' | 'line' | 'block' | "'" | '"' | '`'
  let mode = 'code';
  while (i < src.length) {
    const c = src[i];
    const next = src[i + 1];
    if (mode === 'code') {
      if (c === '/' && next === '/') {
        mode = 'line';
        i += 2;
        continue;
      }
      if (c === '/' && next === '*') {
        mode = 'block';
        i += 2;
        continue;
      }
      if (c === "'" || c === '"' || c === '`') {
        mode = c;
        out += c;
        i += 1;
        continue;
      }
      out += c;
      i += 1;
      continue;
    }
    if (mode === 'line') {
      if (c === '\n') {
        mode = 'code';
        out += c;
      }
      i += 1;
      continue;
    }
    if (mode === 'block') {
      if (c === '*' && next === '/') {
        mode = 'code';
        i += 2;
        continue;
      }
      if (c === '\n') out += c; // сохраняем нумерацию строк
      i += 1;
      continue;
    }
    // внутри строкового/шаблонного литерала
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

/** Убирает все пробельные символы — нормализация перед поиском форм. */
function squash(s) {
  return s.replace(/\s+/g, '');
}

/**
 * Возвращает `{ body, end }` для сбалансированного блока, открывающегося
 * символом `open` на позиции `from` (или после неё), либо `null`.
 */
function extractBalanced(src, from, open, close) {
  const start = src.indexOf(open, from);
  if (start === -1) return null;
  let depth = 0;
  for (let i = start; i < src.length; i++) {
    const c = src[i];
    if (c === open) depth++;
    else if (c === close) {
      depth--;
      if (depth === 0) return { body: src.slice(start + 1, i), start, end: i };
    }
  }
  return null;
}

/** Номер строки (1-based) по индексу символа в тексте. */
function lineAt(src, index) {
  let line = 1;
  for (let i = 0; i < index; i++) {
    if (src.charCodeAt(i) === 10) line++;
  }
  return line;
}

/** Условие по номеру склеено с гейтом «правил руками»? */
function hasGatedInventoryCond(squashedBody) {
  return GATED_INVENTORY_CONDS.some((form) => squashedBody.includes(form));
}

/** Осталось ли в блоке ГОЛОЕ условие по номеру после вычёркивания склеенных? */
function hasBareInventoryCond(squashedBody) {
  let rest = squashedBody;
  for (const form of GATED_INVENTORY_CONDS) {
    rest = rest.split(form).join('«gated»');
  }
  return rest.includes(BARE_INVENTORY_COND);
}

/**
 * Чистая функция сканирования: принимает уже прочитанный исходник, не трогает
 * fs. Переиспользуется и реальным сканированием файла, и self-testом.
 */
function scanSource(relPath, rawSrc, violations) {
  const src = stripJsComments(rawSrc);

  // --- (A) quantityDisabled ------------------------------------------------
  const derivedIdx = src.indexOf('quantityDisabled = $derived');
  if (derivedIdx === -1) {
    violations.push(
      `${relPath}: не найден \`quantityDisabled = $derived(...)\` — гейт NEW-3 не может ` +
        `проверить блокировку «Количество». Если поле переименовано, обнови гейт вместе с ним.`,
    );
  } else {
    const block = extractBalanced(src, derivedIdx, '(', ')');
    if (!block) {
      violations.push(`${relPath}:${lineAt(src, derivedIdx)}: не разобрать тело $derived(...).`);
    } else {
      const squashed = squash(block.body);
      if (!hasGatedInventoryCond(squashed)) {
        violations.push(
          `${relPath}:${lineAt(src, derivedIdx)}: в quantityDisabled условие по инвентарному ` +
            `номеру не склеено с inventoryNoEditedByHand — автоподставленный номер снова ` +
            `блокирует «Количество» (NEW-3).`,
        );
      }
      if (hasBareInventoryCond(squashed)) {
        violations.push(
          `${relPath}:${lineAt(src, derivedIdx)}: в quantityDisabled осталось ГОЛОЕ ` +
            `${BARE_INVENTORY_COND} без гейта inventoryNoEditedByHand (NEW-3).`,
        );
      }
    }
  }

  // --- (B) сброс quantity = 1 ---------------------------------------------
  // Собираем тела всех $effect(...) с их диапазонами.
  const effects = [];
  for (let i = src.indexOf('$effect'); i !== -1; i = src.indexOf('$effect', i + 1)) {
    const block = extractBalanced(src, i, '(', ')');
    if (block) effects.push(block);
  }
  const resetRe = /\bquantity\s*=\s*1\b/g;
  let m;
  while ((m = resetRe.exec(src)) !== null) {
    const owner = effects.find((e) => m.index > e.start && m.index < e.end);
    if (!owner) {
      violations.push(
        `${relPath}:${lineAt(src, m.index)}: сброс \`quantity = 1\` вне \`$effect(...)\` — ` +
          `гейт не может проверить, что он закрыт тем же inventoryNoEditedByHand (CR-01).`,
      );
      continue;
    }
    const squashed = squash(owner.body);
    if (!hasGatedInventoryCond(squashed)) {
      violations.push(
        `${relPath}:${lineAt(src, m.index)}: $effect, сбрасывающий \`quantity = 1\`, не читает ` +
          `inventoryNoEditedByHand — это в точности CR-01: программная автоподстановка номера ` +
          `молча возвращает «Количество» в 1 и отменяет NEW-3.`,
      );
    }
    if (hasBareInventoryCond(squashed)) {
      violations.push(
        `${relPath}:${lineAt(src, m.index)}: $effect, сбрасывающий \`quantity = 1\`, содержит ` +
          `ГОЛОЕ ${BARE_INVENTORY_COND} — условие обязано совпадать с quantityDisabled (CR-01).`,
      );
    }
  }

  // --- (C) ветка qty>1: inventory_no всегда null --------------------------
  const bulkIdx = src.indexOf('devices.bulkCreate(');
  if (bulkIdx === -1) {
    violations.push(
      `${relPath}: не найден вызов \`devices.bulkCreate(\` — гейт NEW-3 не может проверить ` +
        `payload массового создания.`,
    );
  } else {
    const litIdx = src.lastIndexOf('DeviceNew = {', bulkIdx);
    if (litIdx === -1) {
      violations.push(
        `${relPath}:${lineAt(src, bulkIdx)}: перед devices.bulkCreate( не найден литерал ` +
          `\`DeviceNew = { ... }\` — гейт не может проверить inventory_no.`,
      );
    } else {
      const block = extractBalanced(src, litIdx, '{', '}');
      const squashed = block ? squash(block.body) : '';
      const values = [...squashed.matchAll(/inventory_no:([^,}]*)/g)].map((x) => x[1]);
      if (values.length === 0) {
        violations.push(
          `${relPath}:${lineAt(src, litIdx)}: в payload массового создания нет ключа ` +
            `inventory_no — он обязан присутствовать и быть null (NEW-3).`,
        );
      }
      for (const v of values) {
        if (v !== 'null') {
          violations.push(
            `${relPath}:${lineAt(src, litIdx)}: в ветке qty>1 inventory_no = ${v} вместо ` +
              `безусловного null — сервер жёстко отклоняет count>1 с непустым номером ` +
              `(bulk_create_rejects_when_inventory_no_set).`,
          );
        }
      }
    }
  }
}

/** Тонкая обёртка: читает файл через fs и вызывает scanSource. */
function checkFile(root, relPath, violations) {
  const abs = path.join(root, relPath);
  if (!fs.existsSync(abs)) {
    violations.push(`${relPath}: файл не найден — гейт NEW-3 нечего проверять.`);
    return;
  }
  scanSource(relPath, fs.readFileSync(abs, 'utf8'), violations);
}

// Корректная разводка — базис негативных фикстур и «здоровая» часть позитивных.
const CLEAN_DERIVED = `const quantityDisabled = $derived(
    isEdit ||
      (inventoryNo.trim() !== '' && inventoryNoEditedByHand) ||
      serialNo.trim() !== '',
  );`;
const CLEAN_EFFECT = `$effect(() => {
    if ((inventoryNo.trim() !== '' && inventoryNoEditedByHand) || serialNo.trim() !== '') {
      quantity = 1;
    }
  });`;
const CLEAN_BULK = `const newDevice: DeviceNew = {
            name: name.trim(),
            inventory_no: null,
            serial_no: serialNo.trim() || null,
          };
          await devices.bulkCreate(newDevice, qty);`;
const CLEAN = `${CLEAN_DERIVED}\n${CLEAN_EFFECT}\n${CLEAN_BULK}\n`;

/**
 * Встроенный self-test: 9 фикстур в памяти (без временных файлов) — 6
 * позитивных (все известные формы возврата CR-01/NEW-3) + 3 негативных
 * (корректная разводка и её допустимые варианты).
 */
function runSelfTest() {
  const fixtures = [
    {
      name: "CR-01 буквально: $effect на голом inventoryNo.trim() !== ''",
      src:
        CLEAN_DERIVED +
        `\n$effect(() => {
    if (inventoryNo.trim() !== '' || serialNo.trim() !== '') {
      quantity = 1;
    }
  });\n` +
        CLEAN_BULK,
      expectViolations: 2, // нет склейки + осталось голое условие
    },
    {
      name: 'quantityDisabled вернули к голому условию',
      src:
        `const quantityDisabled = $derived(
    isEdit || inventoryNo.trim() !== '' || serialNo.trim() !== '',
  );\n` +
        CLEAN_EFFECT +
        CLEAN_BULK,
      expectViolations: 2,
    },
    {
      name: 'сброс quantity = 1 вынесен из $effect в функцию',
      src:
        CLEAN_DERIVED +
        `\nfunction resetQty() {
    if (inventoryNo.trim() !== '') {
      quantity = 1;
    }
  }\n` +
        CLEAN_BULK,
      expectViolations: 1,
    },
    {
      name: 'ветка qty>1 снова шлёт номер условно',
      src:
        CLEAN_DERIVED +
        CLEAN_EFFECT +
        `const newDevice: DeviceNew = {
            name: name.trim(),
            inventory_no: inventoryNo.trim() || null,
          };
          await devices.bulkCreate(newDevice, qty);`,
      expectViolations: 1,
    },
    {
      name: 'ключ inventory_no в payload массового создания удалён',
      src:
        CLEAN_DERIVED +
        CLEAN_EFFECT +
        `const newDevice: DeviceNew = {
            name: name.trim(),
            serial_no: serialNo.trim() || null,
          };
          await devices.bulkCreate(newDevice, qty);`,
      expectViolations: 1,
    },
    {
      name: 'quantityDisabled переименован/удалён',
      src: CLEAN_EFFECT + CLEAN_BULK,
      // Ровно одно нарушение: правила (B) и (C) в этой фикстуре корректны,
      // отсутствует только сам `quantityDisabled = $derived(...)`.
      expectViolations: 1,
    },
    {
      name: 'корректная разводка (негатив)',
      src: CLEAN,
      expectViolations: 0,
    },
    {
      name: 'корректная разводка, операнды склейки в обратном порядке (негатив)',
      src: CLEAN.split("inventoryNo.trim() !== '' && inventoryNoEditedByHand").join(
        "inventoryNoEditedByHand && inventoryNo.trim() !== ''",
      ),
      expectViolations: 0,
    },
    {
      name: 'корректная разводка с комментарием, цитирующим запрещённую форму (негатив)',
      src: `// до 40.4 здесь было \`inventoryNo.trim() !== ''\` без гейта — см. CR-01\n${CLEAN}`,
      expectViolations: 0,
    },
  ];

  let failed = 0;
  for (const f of fixtures) {
    const v = [];
    scanSource('fixture.svelte', f.src, v);
    const ok = v.length === f.expectViolations;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (expected ${f.expectViolations}, got ${v.length})`,
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
      `${TAG} Usage: node scripts/check-device-form-quantity-gate.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }

  if (args.includes('--selftest')) {
    const ok = runSelfTest();
    process.exit(ok ? 0 : 1);
  }

  const argSrc = args.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  checkFile(SRC_ROOT, TARGET_FILE, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
