#!/usr/bin/env node
// [check-path-settings-form] Постоянный структурный гейт формы экрана
// «Настройки → Организация → Формат отображения пути места»
// (Phase 39.2, находки ревью 39.1: WR-06, WR-07, IN-04, WR-08).
//
// Почему он существует: все четыре инварианта, которые он держит, уже уехали в
// сборку — и ни один из существующих гейтов не мог их увидеть ПО КОНСТРУКЦИИ.
//   `svelte-check` — сверяет типы; вложенный <label> и отсутствующий
//                    aria-describedby типов не нарушают;
//   `eslint`       — не разбирает вложенность разметки Svelte-компонентов
//                    (<Radio> для него — обычный компонент, а не <label>);
//   `pnpm build`   — доказывает, что файл компилируется, а не что кнопка
//                    гаснет и что подпись радио связана с инпутом.
// Прецедент проекта («compile gates miss Svelte runtime»): зелёная сборка не
// говорит ничего о рантайм-реактивности рун и о валидности разметки.
//
// Гейт СТРУКТУРНЫЙ: он читает исходники и проверяет наличие инвариантов, а НЕ
// правильность рендера. Реактивность рун, реальную ассоциацию подписи и
// озвучивание скринридером проверяет только человек (checkpoint 39.2-05).
// Задача гейта — громко падать, когда выстраданный фикс молча удалили при
// рефакторинге.
//
// Проверяемые инварианты:
//   INV-1 (WR-07) — РЕПОЗИТОРНЫЙ: ни один <Radio> в src/ не вызывается
//                   self-closing и не обёрнут внешним <label>. Radio.svelte
//                   САМ рендерит <label class="check-row"> и берёт подпись из
//                   children; внешний <label> даёт вложенный <label>,
//                   запрещённый спецификацией HTML, — ассоциация подписи с
//                   инпутом становится браузерозависимой.
//   INV-2 (WR-06) — у кнопки «Сохранить формат пути» есть disabled=, чьё
//                   выражение упоминает sepEndsErr и sepLastTwoErr, а тело
//                   savePathDefaults() делает ранний возврат по тем же
//                   переменным (кнопку можно обойти программно).
//   INV-3 (IN-04) — оба поля разделителей передают aria-describedby, и каждый
//                   упомянутый там id реально объявлен в этом же файле (ссылка
//                   на несуществующий узел — мёртвая связь, скринридер молчит).
//   INV-4 (WR-08) — состояние экрана НЕ владеет дефолтами формата пути:
//                   pathVariant/sepEnds/sepLastTwo инициализируются только ''
//                   или null, а рядом объявлен флаг pathLoaded. Дефолты после
//                   фазы 39.2 живут ровно в двух местах: модуль
//                   trackly_infra::repos::place_path_settings и сид V039.
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-path-settings-form.mjs               # проверить репозиторий
//   node scripts/check-path-settings-form.mjs --src=<ui-dir> # проверить копию (самотест гейта)
//
// ВАЖНО про --src: аргумент задаёт КОРЕНЬ, относительно которого резолвится
// подкаталог `src/` и путь `src/features/settings/OrgSettings.svelte`. Копия
// для самотеста готовится как `cp -R ui/src <root>/src` (с хвостовым `/src`).
// Без этого обход не найдёт ни одного файла — поэтому гейт печатает число
// просмотренных .svelte и падает кодом 1 при нуле: «зелёный» прогон по пустому
// обходу хуже красного.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-path-settings-form]';

const ORG_SETTINGS = 'src/features/settings/OrgSettings.svelte';
const SAVE_BUTTON_LABEL = 'Сохранить формат пути';
const SEP_FIELD_IDS = ['path-sep-ends', 'path-sep-last-two'];
const STATE_NAMES = ['pathVariant', 'sepEnds', 'sepLastTwo'];
const LOADED_FLAG = 'pathLoaded';
const ERR_NAMES = ['sepEndsErr', 'sepLastTwoErr'];
/** Единственные допустимые инициализаторы состояния формата пути (INV-4). */
const ALLOWED_INITIALIZERS = ["''", '""', '``', 'null'];

// ---------------------------------------------------------------------------
// Хелперы разбора
// ---------------------------------------------------------------------------

/**
 * Затирает пробелами (сохраняя длину и переводы строк) всё, что не является
 * разметкой: HTML-комментарии, блоки <script> и <style>. Без этого `<Radio`,
 * упомянутый в комментарии-объяснении или в строке внутри <script>, считался
 * бы вызовом компонента — и гейт ругался бы на объяснение самого себя.
 * Позиции символов сохраняются, поэтому индексы применимы к оригиналу.
 */
function markupOnly(src) {
  const out = src.split('');
  const blankRange = (from, to) => {
    for (let i = from; i < to && i < src.length; i++) if (src[i] !== '\n') out[i] = ' ';
  };

  for (const re of [
    /<!--[\s\S]*?-->/g,
    /<script[\s\S]*?<\/script>/gi,
    /<style[\s\S]*?<\/style>/gi,
  ]) {
    for (const m of src.matchAll(re)) blankRange(m.index, m.index + m[0].length);
  }
  return out.join('');
}

/**
 * Содержимое первого блока <script> с затёртыми комментариями, но СОХРАНЁННЫМИ
 * строковыми литералами: INV-4 ищет литералы-дефолты внутри $state(...), их
 * затирать нельзя, а комментарий, цитирующий прежний дефолт, не должен ронять
 * гейт. Возвращает { code, offset } — offset нужен для номеров строк.
 */
function scriptCode(src) {
  const m = src.match(/<script[^>]*>/i);
  if (!m) return null;
  const start = m.index + m[0].length;
  const end = src.toLowerCase().indexOf('</script>', start);
  if (end < 0) return null;
  return { code: blankCommentsKeepStrings(src.slice(start, end)), offset: start };
}

/**
 * Затирает пробелами содержимое комментариев (`//`, `/* *\/`, `<!-- -->`),
 * пропуская строковые и шаблонные литералы целиком: в этом файле легально
 * живёт литерал `' // '`, и наивный блэнкер принял бы его за начало
 * комментария, съев остаток строки.
 */
function blankCommentsKeepStrings(src) {
  const out = src.split('');
  const n = src.length;
  const blank = (idx) => {
    if (src[idx] !== '\n') out[idx] = ' ';
  };
  let i = 0;
  while (i < n) {
    const c = src[i];
    const next = src[i + 1];
    if (c === '/' && next === '/') {
      while (i < n && src[i] !== '\n') blank(i++);
    } else if (c === '/' && next === '*') {
      blank(i++);
      blank(i++);
      while (i < n && !(src[i] === '*' && src[i + 1] === '/')) blank(i++);
      if (i < n) {
        blank(i++);
        blank(i++);
      }
    } else if (src.startsWith('<!--', i)) {
      while (i < n && !src.startsWith('-->', i)) blank(i++);
      for (let k = 0; k < 3 && i < n; k++) blank(i++);
    } else if (c === "'" || c === '"' || c === '`') {
      const quote = c;
      i++;
      while (i < n && src[i] !== quote) {
        if (src[i] === '\\') {
          i += 2;
          continue;
        }
        i++;
      }
      i++;
    } else {
      i++;
    }
  }
  return out.join('');
}

/**
 * Индекс `>`, закрывающего открывающий тег, начинающийся на `startIdx`.
 * Учитывает вложенность `{}`: `disabled={a || !b}` и `onclick={() => f(x)}`
 * содержат `>`, который тегом не является.
 */
function endOfOpeningTag(text, startIdx) {
  let depth = 0;
  for (let i = startIdx; i < text.length; i++) {
    const c = text[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return i;
  }
  return -1;
}

const lineOf = (src, idx) => src.slice(0, idx).split('\n').length;

/** true, если открывающий тег `attrs` (включая финальный `>`) self-closing. */
function isSelfClosing(openTag) {
  return /\/\s*>$/.test(openTag.trim());
}

/** Значение атрибута: `{...}` (с балансом скобок) либо "..."/'...'. */
function attrValue(attrs, name) {
  const re = new RegExp(`(?:^|[\\s])${name}\\s*=\\s*`);
  const m = attrs.match(re);
  if (!m) return null;
  let i = m.index + m[0].length;
  if (attrs[i] === '{') {
    let depth = 0;
    for (let j = i; j < attrs.length; j++) {
      if (attrs[j] === '{') depth++;
      else if (attrs[j] === '}') {
        depth--;
        if (depth === 0) return { kind: 'expr', raw: attrs.slice(i + 1, j) };
      }
    }
    return null;
  }
  if (attrs[i] === '"' || attrs[i] === "'") {
    const quote = attrs[i];
    const end = attrs.indexOf(quote, i + 1);
    if (end < 0) return null;
    return { kind: 'literal', raw: attrs.slice(i + 1, end) };
  }
  return null;
}

/** Список id из значения aria-describedby (литерал или выражение с литералами). */
function describedbyIds(value) {
  if (value === null) return null;
  const chunks =
    value.kind === 'literal'
      ? [value.raw]
      : [...value.raw.matchAll(/['"`]([^'"`]*)['"`]/g)].map((m) => m[1]);
  return chunks.flatMap((c) => c.split(/\s+/).filter(Boolean));
}

/** Тело функции по имени (в т.ч. `async function`). */
function functionBody(code, fnName) {
  const m = code.match(new RegExp(`function\\s+${fnName}\\s*\\(`));
  if (!m) return null;
  const open = code.indexOf('{', m.index + m[0].length);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < code.length; i++) {
    if (code[i] === '{') depth++;
    else if (code[i] === '}') {
      depth--;
      if (depth === 0) return code.slice(open + 1, i);
    }
  }
  return null;
}

/**
 * Инициализатор руны состояния: `let NAME = $state<T>(INIT)` → строка INIT.
 * Возвращает { init, index } либо null, если объявления нет.
 */
function stateInitializer(code, name) {
  const m = code.match(new RegExp(`\\blet\\s+${name}\\b`));
  if (!m) return null;
  const sIdx = code.indexOf('$state', m.index);
  if (sIdx < 0 || code.slice(m.index, sIdx).includes(';')) return null;
  let i = sIdx + '$state'.length;
  while (i < code.length && /\s/.test(code[i])) i++;
  if (code[i] === '<') {
    let depth = 0;
    for (; i < code.length; i++) {
      if (code[i] === '<') depth++;
      else if (code[i] === '>') {
        depth--;
        if (depth === 0) {
          i++;
          break;
        }
      }
    }
  }
  while (i < code.length && /\s/.test(code[i])) i++;
  if (code[i] !== '(') return null;
  let depth = 0;
  for (let j = i; j < code.length; j++) {
    if (code[j] === '(') depth++;
    else if (code[j] === ')') {
      depth--;
      if (depth === 0) return { init: code.slice(i + 1, j).trim(), index: m.index };
    }
  }
  return null;
}

/** Рекурсивный обход `.svelte`-файлов. */
function walkSvelte(dir) {
  const found = [];
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return found;
  }
  for (const e of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) found.push(...walkSvelte(full));
    else if (e.isFile() && e.name.endsWith('.svelte')) found.push(full);
  }
  return found;
}

// ---------------------------------------------------------------------------
// INV-1 (WR-07) — форма вызова <Radio> во всей кодовой базе
// ---------------------------------------------------------------------------

function checkRadioForm(src, label) {
  const violations = [];
  const fail = (message, hint) => violations.push({ inv: 'INV-1', fixId: 'WR-07', message, hint });

  const markup = markupOnly(src);
  let labelDepth = 0;

  for (const m of markup.matchAll(/<label\b|<\/label\s*>|<Radio\b/g)) {
    const token = m[0];
    if (token === '<label') {
      labelDepth++;
      continue;
    }
    if (token.startsWith('</label')) {
      labelDepth = Math.max(0, labelDepth - 1);
      continue;
    }
    const line = lineOf(src, m.index);
    const openEnd = endOfOpeningTag(markup, m.index);
    if (openEnd < 0) {
      fail(
        `${label}:${line} — открывающий тег <Radio> не закрыт`,
        'Разметка не разбирается — поправь её либо осознанно обнови гейт.',
      );
      continue;
    }
    if (isSelfClosing(markup.slice(m.index, openEnd + 1))) {
      fail(
        `${label}:${line} — <Radio ... /> вызван self-closing`,
        'Radio.svelte берёт подпись из children: требуемая форма — ' +
          '`<Radio ...>Текст</Radio>`. Self-closing вызов оставляет компонент без ' +
          'подписи, и её приходится вешать снаружи — так и появляется внешний <label>.',
      );
    }
    if (labelDepth > 0) {
      fail(
        `${label}:${line} — <Radio> находится внутри внешнего <label>`,
        'Radio.svelte САМ рендерит <label class="check-row"> — внешний даёт вложенный ' +
          '<label>, запрещённый спецификацией HTML: ассоциация подписи с инпутом ' +
          'становится браузерозависимой, скринридер читает подпись дважды или не читает ' +
          'вовсе. Требуемая форма — `<Radio ...>Текст</Radio>` без обёртки ' +
          '(см. ThresholdSettings.svelte, ActiveDirectorySettings.svelte, FieldsSection.svelte).',
      );
    }
  }

  return violations;
}

// ---------------------------------------------------------------------------
// INV-2..INV-4 — точечные проверки OrgSettings.svelte
// ---------------------------------------------------------------------------

function checkOrgSettings(src, label) {
  const violations = [];
  const fail = (inv, fixId, message, hint) => violations.push({ inv, fixId, message, hint });

  const markup = markupOnly(src);
  const script = scriptCode(src);
  const code = script ? script.code : '';
  if (!script) {
    fail(
      'INV-0',
      '39.2-05',
      `${label} — не найден блок <script>`,
      'Структура файла изменилась настолько, что гейт ничего не проверяет. Обнови его ' +
        'вместе с рефакторингом, не удаляй.',
    );
    return violations;
  }

  // --- INV-2 (WR-06): кнопка гаснет и функция делает ранний возврат ---------
  let buttonFound = false;
  for (const m of markup.matchAll(/<Button\b/g)) {
    const openEnd = endOfOpeningTag(markup, m.index);
    if (openEnd < 0) continue;
    const closeIdx = markup.indexOf('</Button', openEnd);
    if (closeIdx < 0) continue;
    const body = markup.slice(openEnd + 1, closeIdx);
    if (!body.includes(SAVE_BUTTON_LABEL)) continue;
    buttonFound = true;

    const attrs = markup.slice(m.index, openEnd + 1);
    const line = lineOf(src, m.index);
    const disabled = attrValue(attrs, 'disabled');
    if (disabled === null) {
      fail(
        'INV-2',
        'WR-06',
        `${label}:${line} — у кнопки «${SAVE_BUTTON_LABEL}» нет атрибута disabled=`,
        'Клик с заведомо пустым разделителем уходит на сервер, ловит AppError::Validation ' +
          'и превращается в красный тост, дублирующий inline-ошибку рядом с полем. ' +
          `Ожидается disabled={...} с упоминанием ${ERR_NAMES.join(' и ')}.`,
      );
    } else {
      const missing = ERR_NAMES.filter((n) => !disabled.raw.includes(n));
      if (missing.length > 0) {
        fail(
          'INV-2',
          'WR-06',
          `${label}:${line} — disabled={${disabled.raw.trim()}} не упоминает ${missing.join(', ')}`,
          'Кнопка обязана гаснуть по ОБОИМ валидаторам разделителей, иначе половина ' +
            'невалидных значений по-прежнему уезжает round-trip’ом на сервер.',
        );
      }
    }
  }
  if (!buttonFound) {
    fail(
      'INV-2',
      'WR-06',
      `${label} — не найдена кнопка с текстом «${SAVE_BUTTON_LABEL}»`,
      'Её переименовали или удалили — проверка WR-06 больше ничего не держит. ' +
        'Обнови гейт вместе с рефакторингом, осознанно.',
    );
  }

  const saveBody = functionBody(code, 'savePathDefaults');
  if (saveBody === null) {
    fail(
      'INV-2',
      'WR-06',
      `${label} — не найдена функция savePathDefaults()`,
      'Обнови гейт вместе с рефакторингом, не удаляй проверку.',
    );
  } else {
    const missing = ERR_NAMES.filter((n) => !saveBody.includes(n));
    if (missing.length > 0) {
      fail(
        'INV-2',
        'WR-06',
        `${label} — тело savePathDefaults() не упоминает ${missing.join(', ')}`,
        'disabled у кнопки — только UX: обойти его программно тривиально. Ранний возврат ' +
          'в самой функции — вторая половина фикса WR-06.',
      );
    }
  }

  // --- INV-3 (IN-04): hint/error связаны с полем ---------------------------
  const declaredIds = new Set(
    [...markup.matchAll(/\bid\s*=\s*["']([^"']+)["']/g)].map((m) => m[1]),
  );

  for (const fieldId of SEP_FIELD_IDS) {
    let inputFound = false;
    for (const m of markup.matchAll(/<Input\b/g)) {
      const openEnd = endOfOpeningTag(markup, m.index);
      if (openEnd < 0) continue;
      const attrs = markup.slice(m.index, openEnd + 1);
      const id = attrValue(attrs, 'id');
      if (id === null || id.kind !== 'literal' || id.raw !== fieldId) continue;
      inputFound = true;
      const line = lineOf(src, m.index);

      const value = attrValue(attrs, 'aria-describedby');
      if (value === null) {
        fail(
          'INV-3',
          'IN-04',
          `${label}:${line} — <Input id="${fieldId}"> не передаёт aria-describedby`,
          'Проп у Input.svelte уже есть и пробрасывается на <input>. Без него скринридер ' +
            'не прочитает ни подсказку «Значение: «…»», ни текст ошибки — они лежат в ' +
            'соседних <span> и с полем ничем не связаны.',
        );
        continue;
      }
      const ids = describedbyIds(value) ?? [];
      if (ids.length === 0) {
        fail(
          'INV-3',
          'IN-04',
          `${label}:${line} — aria-describedby у поля «${fieldId}» не содержит ни одного id`,
          'Значение собирается из строковых литералов с идентификаторами описывающих ' +
            'элементов; пустая связь эквивалентна её отсутствию.',
        );
        continue;
      }
      const dead = ids.filter((id2) => !declaredIds.has(id2));
      if (dead.length > 0) {
        fail(
          'INV-3',
          'IN-04',
          `${label}:${line} — aria-describedby у поля «${fieldId}» ссылается на ` +
            `необъявленные id: ${dead.join(', ')}`,
          'Ссылка на несуществующий узел — мёртвая связь: скринридер молча ничего не ' +
            'озвучит. У каждого упомянутого идентификатора должен быть элемент с таким id ' +
            'в этом же файле.',
        );
      }
    }
    if (!inputFound) {
      fail(
        'INV-3',
        'IN-04',
        `${label} — не найден <Input id="${fieldId}">`,
        'Поле переименовали или удалили — проверка IN-04 больше ничего не держит. ' +
          'Обнови гейт вместе с рефакторингом, осознанно.',
      );
    }
  }

  // --- INV-4 (WR-08): состояние не владеет дефолтами -----------------------
  for (const name of STATE_NAMES) {
    const decl = stateInitializer(code, name);
    if (decl === null) {
      fail(
        'INV-4',
        'WR-08',
        `${label} — не найдено объявление состояния \`let ${name} = $state(...)\``,
        'Обнови гейт вместе с рефакторингом, не удаляй проверку.',
      );
      continue;
    }
    if (!ALLOWED_INITIALIZERS.includes(decl.init)) {
      const line = lineOf(src, script.offset + decl.index);
      fail(
        'INV-4',
        'WR-08',
        `${label}:${line} — \`${name}\` инициализируется значением \`${decl.init}\``,
        'Экран снова стал владельцем дефолта формата пути (восьмое место из WR-08). ' +
          'После фазы 39.2 дефолты объявлены ровно в двух местах: модуль ' +
          'trackly_infra::repos::place_path_settings и сид migrations/V039. Состояние ' +
          `экрана обязано стартовать «незагруженным» (допустимо только ${ALLOWED_INITIALIZERS.join(
            ' / ',
          )}), а значения приходить ТОЛЬКО из settings_get_place_path_defaults — иначе ` +
          'при отказе загрузки «Сохранить» перезапишет реальные настройки организации ' +
          'умолчаниями, которых пользователь не выбирал.',
      );
    }
  }

  if (stateInitializer(code, LOADED_FLAG) === null) {
    fail(
      'INV-4',
      'WR-08',
      `${label} — не найден флаг \`let ${LOADED_FLAG} = $state(false)\``,
      'Без него «незагруженное» состояние неотличимо от загруженного: валидаторы сразу ' +
        'зажигают ошибку «не может быть пустым», а кнопка не знает, что сохранять нечего.',
    );
  }

  return violations;
}

// ---------------------------------------------------------------------------

function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--help') || argv.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-path-settings-form.mjs [--src=<ui-dir>]\n` +
        `${TAG}   --src задаёт КОРЕНЬ, содержащий подкаталог src/ ` +
        '(готовить копию как `cp -R ui/src <root>/src`).',
    );
    process.exit(0);
  }

  const argSrc = argv.find((a) => a.startsWith('--src='));
  const SRC_ROOT = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const files = walkSvelte(path.join(SRC_ROOT, 'src'));
  const checked = files.length;

  if (checked === 0) {
    console.error(
      `${TAG} FAIL — просмотрено 0 .svelte-файлов в ${path.join(SRC_ROOT, 'src')}. ` +
        'Пустой обход даёт «зелёный» прогон, который ничего не доказывает — это хуже ' +
        'красного, поэтому гейт падает. Проверь форму --src=: он указывает на КОРЕНЬ, ' +
        'СОДЕРЖАЩИЙ подкаталог src/.',
    );
    process.exit(1);
  }

  const violations = [];
  for (const full of files) {
    const rel = path.relative(SRC_ROOT, full);
    violations.push(...checkRadioForm(fs.readFileSync(full, 'utf8'), rel));
  }

  const orgPath = path.join(SRC_ROOT, ORG_SETTINGS);
  let orgSource = null;
  try {
    orgSource = fs.readFileSync(orgPath, 'utf8');
  } catch {
    console.error(
      `${TAG} FAIL — не удалось прочитать ${ORG_SETTINGS} (просмотрено ${checked} .svelte). ` +
        'Экран переехал или удалён: обнови путь в гейте осознанно, а не удаляй проверки ' +
        'WR-06/IN-04/WR-08.',
    );
    process.exit(1);
  }
  violations.push(...checkOrgSettings(orgSource, ORG_SETTINGS));

  for (const v of violations) {
    console.error(`${TAG} ${v.inv} (регресс ${v.fixId}): ${v.message}`);
    console.error(`${TAG}   ${v.hint}`);
  }

  if (violations.length > 0) {
    console.error(
      `${TAG} FAIL — ${violations.length} нарушений формы экрана «Формат пути» ` +
        `(просмотрено ${checked} .svelte-файлов). Все четыре инварианта уже уезжали в ` +
        'сборку и были найдены ревьюером, а не гейтом — не «чинить» гейт, а вернуть форму.',
    );
    process.exit(1);
  }

  console.error(`${TAG} PASS — 0 нарушений (просмотрено ${checked} .svelte-файлов)`);
  process.exit(0);
}

main();
