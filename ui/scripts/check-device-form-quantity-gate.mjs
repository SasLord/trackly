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
// Все три правила проверяются встроенным self-testом (`--selftest`) на
// фикстурах в памяти (включая буквальный до-fae6c9d4 вид эффекта и негативы —
// текущая корректная разводка и её варианты), поэтому «гейт зелёный»
// отличимо от «гейт ничего не ищет».
//
// ВТОРОЙ инвариант (имя файла историческое; фаза 41.1, план 25, WR-01):
// эффект автостатуса «место хранения → статус «На складе»» не имеет права
// срабатывать при правке без явного действия пользователя и до загрузки
// членства в группе. До фикса правка якоря разбора («На списание») на складе
// молча получала «На складе», а чекбокс после этого скрывался — откатить
// статус было нечем. Правило (D):
//   (D0) нижняя граница: найден хотя бы один ЦЕЛЕВОЙ `$effect` — блок с
//        присваиванием `statusId =` (любой формы: String(..), шаблонная строка,
//        через переменную), который ещё и упоминает `storageId` либо
//        `на_складе`. Эффект начального статуса (D-18, `statusIdByCode(
//        presetStatusCode)`) и `onPickGroup`-присваивание вне эффекта —
//        легальны и целевыми не считаются;
//   (D1) каждый целевой эффект обязан ДО своего присваивания читать
//        `membershipLoaded`, `storageIntent` и `isEdit`, причём проверяется
//        СМЫСЛ условия, а не набор слов (WR-06, фаза 41.11, D-21): `(!isEdit ||
//        storageIntent)` целиком, конъюнктом (допустима перестановка операндов),
//        и ни одно вхождение `membershipLoaded` не стоит под отрицанием.
//        Инверсия `(isEdit || storageIntent)` / `!membershipLoaded` оставляет
//        все три имени на месте, но снимает охрану при создании и оставляет её
//        при правке — ровно наоборот;
//   (D2) `storageIntent = true` — не менее двух раз (место и чекбокс) и ни
//        разу внутри `$effect` (иначе эффект включает собственное условие);
//   (D3) `membershipLoaded = true` — не менее двух раз (then и catch запроса
//        членства) и ни разу внутри `$effect`;
//   (D4) вне `$effect` нет присваивания `statusId`, в правой части которого
//        есть `storageId` (обход целевого эффекта обработчиком).
// Счётная часть правила слепа к ПОРЯДКУ, поэтому в (D1) имена ищутся только
// в тексте блока ДО присваивания: `void membershipLoaded` после него не спасает.
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
function scanSource(relPath, rawSrc, violations, rules = ['Q', 'S']) {
  const src = stripJsComments(rawSrc);
  if (rules.includes('Q')) scanQuantityRules(relPath, src, violations);
  if (rules.includes('S')) scanStorageStatusRules(relPath, src, violations);
}

/** Собирает диапазоны всех `$effect(...)` в тексте без комментариев. */
function collectEffects(src) {
  const effects = [];
  for (let i = src.indexOf('$effect'); i !== -1; i = src.indexOf('$effect', i + 1)) {
    const block = extractBalanced(src, i, '(', ')');
    if (block) effects.push(block);
  }
  return effects;
}

/** Правила (A)-(C): инвариант NEW-3 (количество и инвентарный номер). */
function scanQuantityRules(relPath, src, violations) {
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
  const effects = collectEffects(src);
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

// Сколько целевых эффектов нашёл последний прогон (для вывода PASS).
const STATS = { targets: 0 };
const STATUS_ASSIGN_RE = /\bstatusId\s*=(?!=)/;
const STORAGE_REQUIRED_NAMES = ['membershipLoaded', 'storageIntent', 'isEdit'];
// WR-06 (D-21): проверяется текст условия БЕЗ пробелов (squash). Охрана при правке —
// конъюнкт `(!isEdit||storageIntent)` (операнды в любом порядке), стоящий на верхнем
// уровне `&&`-цепочки: ни `||(…)`, ни `!(…)` перед скобкой не допускаются.
const EDIT_GUARD_RE =
  /(?:&&|if\()\((?:!isEdit\|\|storageIntent|storageIntent\|\|!isEdit)\)(?:&&|\))/;
// НИ ОДНО вхождение membershipLoaded не под отрицанием: «хотя бы одно без !» пропустило бы
// два вхождения, из которых инвертировано одно.
const NEGATED_MEMBERSHIP_RE = /!membershipLoaded\b|\bmembershipLoaded(?:===false|!==true)/;

/** Правило (D): эффект автостатуса склада (WR-01 фазы 41.1). */
function scanStorageStatusRules(relPath, src, violations) {
  const effects = collectEffects(src);
  const inAnyEffect = (idx) => effects.some((e) => idx > e.start && idx < e.end);

  // --- (D0)/(D1) целевые эффекты ------------------------------------------
  const targets = effects.filter(
    (e) => STATUS_ASSIGN_RE.test(e.body) && /\bstorageId\b|на_складе/.test(e.body),
  );
  STATS.targets = targets.length;
  if (targets.length === 0) {
    violations.push(
      `${relPath}: (D0) не найден ни один $effect, присваивающий \`statusId =\` и связанный со ` +
        `складом (storageId / на_складе) — гейт WR-01 нечего проверять. Если эффект переименован ` +
        `или переписан, обнови гейт вместе с ним, а не удаляй проверку.`,
    );
  }
  for (const e of targets) {
    const assign = STATUS_ASSIGN_RE.exec(e.body);
    const before = e.body.slice(0, assign.index);
    const missing = STORAGE_REQUIRED_NAMES.filter((n) => !new RegExp(`\\b${n}\\b`).test(before));
    const at = `${relPath}:${lineAt(src, e.start + 1 + assign.index)}`;
    if (missing.length > 0) {
      violations.push(
        `${at}: (D1) $effect автостатуса склада не ` +
          `читает ${missing.join(', ')} до присваивания statusId — при правке он снова молча ` +
          `перезапишет статус (WR-01: якорь разбора «На списание» получал «На складе»).`,
      );
    } else {
      // WR-06 (D-21): имена на месте — проверяем полярность условия целиком.
      const flat = squash(before);
      const problems = [];
      if (!EDIT_GUARD_RE.test(flat)) {
        problems.push(
          'условие инвертировано или ослаблено: нет конъюнкта (!isEdit || storageIntent) — ' +
            'охрана при создании снята, а при правке оставлена (или наоборот)',
        );
      }
      if (NEGATED_MEMBERSHIP_RE.test(flat)) {
        problems.push(
          'membershipLoaded под отрицанием — эффект ждёт НЕзагруженного членства и срабатывает ' +
            'раньше, чем lockedByGroup становится достоверным',
        );
      }
      if (problems.length > 0) {
        violations.push(
          `${at}: (D1) $effect автостатуса склада читает нужные имена, но смысл условия нарушен: ` +
            `${problems.join('; ')} (WR-06).`,
        );
      }
    }
  }

  // --- (D2)/(D3) записи флагов только в обработчиках ----------------------
  for (const [name, rule, where] of [
    ['storageIntent', 'D2', 'в обработчиках места и чекбокса'],
    ['membershipLoaded', 'D3', 'в ветках then и catch запроса членства'],
  ]) {
    const re = new RegExp(`\\b${name}\\s*=\\s*true\\b`, 'g');
    let outside = 0;
    let m;
    while ((m = re.exec(src)) !== null) {
      if (inAnyEffect(m.index)) {
        violations.push(
          `${relPath}:${lineAt(src, m.index)}: (${rule}) \`${name} = true\` внутри $effect — ` +
            `эффект включал бы собственное условие; запись допустима только ${where}.`,
        );
      } else {
        outside++;
      }
    }
    if (outside < 2) {
      violations.push(
        `${relPath}: (${rule}) \`${name} = true\` найдено вне $effect ${outside} раз, нужно не ` +
          `менее 2 (${where}).`,
      );
    }
  }

  // --- (D4) обход целевого эффекта обработчиком ---------------------------
  const assignRe = /\bstatusId\s*=(?!=)([^;\n]*)/g;
  let a;
  while ((a = assignRe.exec(src)) !== null) {
    if (inAnyEffect(a.index)) continue;
    if (/\bstorageId\b/.test(a[1])) {
      violations.push(
        `${relPath}:${lineAt(src, a.index)}: (D4) \`statusId = …storageId…\` вне $effect — ` +
          `автостатус склада в обход охран эффекта (membershipLoaded / storageIntent).`,
      );
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

// --- Фикстуры правила (D) -------------------------------------------------
const S_COND =
  '!readonly && membershipLoaded && (!isEdit || storageIntent) && !lockedByGroup && ' +
  'isStoragePlace && storageStatusSuggested && storageId !== null';
const sEffect = (cond, assign = 'statusId = String(storageId);') => `$effect(() => {
    const storageId = statusIdByCode('на_складе');
    if (${cond}) {
      ${assign}
    }
  });`;
// Эффект начального статуса (D-18): легален, целевым не считается.
const S_INIT_EFFECT = `$effect(() => {
    if (presetStatusApplied || target !== null || !presetStatusCode) return;
    const id = statusIdByCode(presetStatusCode);
    if (id === null) return;
    presetStatusApplied = true;
    statusId = String(id);
  });`;
// Присваивание вне эффекта по явному выбору пользователя: легально.
const S_ON_PICK = `<Dropdown onPickGroup={(o) => (statusId = String(o.id))} />`;
const S_HANDLERS = `<PlacePicker onChange={(id) => { placeId = id; storageIntent = true; }} />
  <Checkbox onchange={(c) => { storageStatusSuggested = c; storageIntent = true; }} />`;
const S_MOUNT = `onMount(() => {
    groups.forDevices([1]).then((rows) => { membership = rows[0]; membershipLoaded = true; })
      .catch(() => { membership = null; membershipLoaded = true; });
  });`;
const S_TAIL = `${S_HANDLERS}\n${S_MOUNT}\n`;
const S_CLEAN = `${sEffect(S_COND)}\n${S_TAIL}`;
const S_CLEAN_FULL = `${S_INIT_EFFECT}\n${S_CLEAN}\n${S_ON_PICK}\n`;
const dropName = (cond, re) => cond.replace(re, '');

/**
 * Встроенный self-test: фикстуры в памяти (без временных файлов). Правила
 * (A)-(C) — 9 фикстур; правило (D) — зелёные базисы, до-фиксовый эффект,
 * мутанты формы присваивания, пропажи имён, порядка и записей флагов.
 */
function runSelfTest() {
  const D = ['S'];
  const fixtures = [
    ...storageFixtures(D),
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
    scanSource('fixture.svelte', f.src, v, f.rules ?? ['Q']);
    const ok = v.length === f.expectViolations;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — ${f.name} (expected ${f.expectViolations}, got ${v.length})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  failed += runRealFileSelfTest();

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} из ${fixtures.length} фикстур не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — все ${fixtures.length} фикстур совпали с ожиданием.`);
  return true;
}

/**
 * Мутация реального исходника с проверкой уникальности якоря (D-26): якорь
 * обязан встречаться РОВНО один раз, иначе selftest завершается ошибкой —
 * замена по неуникальному якорю снимала бы охрану не там, где задумано.
 */
function mutate(source, from, to, label) {
  const n = source.split(from).length - 1;
  if (n !== 1) {
    console.error(
      `${TAG} [selftest] FAIL — якорь мутанта «${label}» (${JSON.stringify(from)}) встречается ` +
        `${n} раз вместо одного — расширь его до многострочного, а не заменяй на голый.`,
    );
    process.exit(1);
  }
  return source.replace(from, () => to);
}

/**
 * Шаг self-test по РЕАЛЬНОМУ DeviceFormBody.svelte (WR-06, D-25): контроль без
 * мутаций — 0 нарушений, каждый мутант полярности — ровно 1 нарушение (D1).
 * Возвращает число несовпавших шагов.
 */
function runRealFileSelfTest() {
  const abs = path.join(UI_ROOT, TARGET_FILE);
  if (!fs.existsSync(abs)) {
    console.error(`${TAG} [selftest] FAIL — реальный DeviceFormBody.svelte не найден: ${abs}`);
    return 1;
  }
  const real = fs.readFileSync(abs, 'utf8');
  const steps = [
    ['контроль, без мутаций', real, 0],
    [
      'мутант: (isEdit || storageIntent)',
      mutate(real, '(!isEdit || storageIntent)', '(isEdit || storageIntent)', 'isEdit'),
      1,
    ],
    [
      'мутант: !membershipLoaded',
      mutate(real, 'membershipLoaded &&', '!membershipLoaded &&', 'membershipLoaded'),
      1,
    ],
  ];
  let failed = 0;
  for (const [name, src, expected] of steps) {
    const v = [];
    scanSource(TARGET_FILE, src, v, ['S']);
    const ok = v.length === expected && v.every((x) => x.includes('(D1)') || expected === 0);
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — реальный DeviceFormBody.svelte: ${name} ` +
        `(expected ${expected}, got ${v.length})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }
  return failed;
}

function storageFixtures(D) {
  const preFix = sEffect(
    '!readonly && !lockedByGroup && isStoragePlace && storageStatusSuggested && storageId !== null',
  );
  return [
    {
      name: '(D) корректный эффект + обработчики (негатив)',
      src: S_CLEAN,
      rules: D,
      expectViolations: 0,
    },
    {
      name: '(D) корректный эффект + эффект начального статуса + onPickGroup (негатив, легальные присваивания)',
      src: S_CLEAN_FULL,
      rules: D,
      expectViolations: 0,
    },
    {
      name: '(D) весь файл: правила (A)-(C) и (D) вместе (негатив)',
      src: `${CLEAN}\n${S_CLEAN_FULL}`,
      rules: ['Q', 'S'],
      expectViolations: 0,
    },
    {
      name: '(D) шаблонная строка вместо String(), имена на месте (негатив: форма присваивания не важна)',
      src: `${sEffect(S_COND, 'statusId = `${storageId}`;')}\n${S_TAIL}`,
      rules: D,
      expectViolations: 0,
    },
    {
      name: "(D) цель найдена по 'на_складе', а не по storageId (негатив)",
      src: `$effect(() => {
        if (${S_COND.replace('storageId !== null', 'true')}) {
          statusId = String(statusIdByCode('на_складе'));
        }
      });\n${S_TAIL}`,
      rules: D,
      expectViolations: 0,
    },
    {
      name: '(D) ДО-ФИКС буквально: условие без membershipLoaded/storageIntent/isEdit',
      src: `${preFix}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D) до-фиксовое условие, шаблонная строка вместо String() — форма не даёт молчаливого пропуска',
      src: `${sEffect(
        '!readonly && !lockedByGroup && isStoragePlace && storageStatusSuggested && storageId !== null',
        'statusId = `${storageId}`;',
      )}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D) до-фиксовый эффект, но комментарий цитирует все три имени',
      src: `// membershipLoaded storageIntent isEdit — так было бы правильно\n${preFix}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D0) из эффекта убрано присваивание statusId',
      src: `${sEffect(S_COND, 'void storageId;')}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D0)+(D4) присваивание вынесено из эффекта в обработчик',
      src: `${sEffect(S_COND, 'void storageId;')}\n${S_TAIL}\n<X onclick={() => { statusId = String(storageId); }} />`,
      rules: D,
      expectViolations: 2,
    },
    {
      name: '(D4) лишнее присваивание statusId = …storageId… в обработчике при целом эффекте',
      src: `${S_CLEAN}\n<X onclick={() => { statusId = String(storageId); }} />`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) в условии нет membershipLoaded',
      src: `${sEffect(dropName(S_COND, /membershipLoaded && /))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) в условии нет storageIntent',
      src: `${sEffect(dropName(S_COND, /\(!isEdit \|\| storageIntent\) && /))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) в условии нет isEdit',
      src: `${sEffect(S_COND.replace('(!isEdit || storageIntent)', 'storageIntent'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: (isEdit || storageIntent) вместо (!isEdit || storageIntent)',
      src: `${sEffect(S_COND.replace('(!isEdit || storageIntent)', '(isEdit || storageIntent)'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: (isEdit && storageIntent)',
      src: `${sEffect(S_COND.replace('(!isEdit || storageIntent)', '(isEdit && storageIntent)'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: !membershipLoaded',
      src: `${sEffect(S_COND.replace('membershipLoaded &&', '!membershipLoaded &&'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: два вхождения membershipLoaded, одно под отрицанием',
      src: `${sEffect(S_COND.replace('membershipLoaded &&', 'membershipLoaded && !membershipLoaded &&'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: !(!isEdit || storageIntent) — отрицание всей скобки',
      src: `${sEffect(S_COND.replace('(!isEdit || storageIntent)', '!(!isEdit || storageIntent)'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 мутант: охрана правки стала дизъюнктом (… || (!isEdit || storageIntent))',
      src: `${sEffect(S_COND.replace('&& (!isEdit || storageIntent) &&', '|| (!isEdit || storageIntent) &&'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D1) WR-06 контроль: операнды переставлены, пробелы другие, перенос строки (негатив)',
      src: `${sEffect(S_COND.replace('(!isEdit || storageIntent)', '(storageIntent\n   ||   !isEdit)'))}\n${S_TAIL}`,
      rules: D,
      expectViolations: 0,
    },
    {
      name: '(D1) имена прочитаны ПОСЛЕ присваивания (охрана после действия)',
      src: `$effect(() => {
        const storageId = statusIdByCode('на_складе');
        if (!readonly && !lockedByGroup && isStoragePlace && storageStatusSuggested && storageId !== null) {
          statusId = String(storageId);
        }
        void membershipLoaded; void storageIntent; void isEdit;
      });\n${S_TAIL}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D2) storageIntent = true в $effect сверх двух обработчиков',
      src: `${S_CLEAN}\n$effect(() => { if (isStoragePlace) storageIntent = true; });`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D2) одна из записей storageIntent перенесена в $effect',
      src: `${sEffect(S_COND, 'statusId = String(storageId); storageIntent = true;')}
        <PlacePicker onChange={(id) => { placeId = id; storageIntent = true; }} />
        <Checkbox onchange={(c) => { storageStatusSuggested = c; }} />
        ${S_MOUNT}`,
      rules: D,
      expectViolations: 2,
    },
    {
      name: '(D2) единственная запись storageIntent (чекбокс не пишет)',
      src: `${sEffect(S_COND)}
        <PlacePicker onChange={(id) => { placeId = id; storageIntent = true; }} />
        <Checkbox onchange={(c) => { storageStatusSuggested = c; }} />
        ${S_MOUNT}`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D3) единственная запись membershipLoaded (catch не пишет)',
      src: `${sEffect(S_COND)}\n${S_HANDLERS}
        onMount(() => { groups.forDevices([1]).then(() => { membershipLoaded = true; }).catch(() => {}); });`,
      rules: D,
      expectViolations: 1,
    },
    {
      name: '(D3) membershipLoaded = true внутри $effect',
      src: `${S_CLEAN}\n$effect(() => { membershipLoaded = true; });`,
      rules: D,
      expectViolations: 1,
    },
  ];
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

  console.error(`${TAG} PASS — 0 нарушений (целевых эффектов автостатуса: ${STATS.targets})`);
  process.exit(0);
}

main();
