#!/usr/bin/env node
// [check-act-group-confirm] Структурный гейт подтверждения выноса из состава
// при сохранении акта (ActFormBody.svelte, GroupReleaseConfirmPopup.svelte,
// ActFormModal.svelte; фаза 41.2, план 12, GAM-02, D-06/D-07).
//
// Почему он существует: сервер отвечает `needs_group_release` ДО первой записи,
// и вся безопасность подтверждения держится на нескольких вещах, которых не
// видят компиляционные гейты (svelte-check, eslint, build):
//   - проверка исхода стоит ВЫШЕ `onSaved(` — иначе форма закроется «успехом»
//     на ответе, который акта не создавал;
//   - «Отмена» не трогает состав позиций и не сохраняет — иначе попап правит
//     акт за спиной формы (D-07);
//   - в попапе две кнопки, primary не destructive, начальный фокус на «Отмена»
//     (случайный Enter не выводит устройства из состава);
//   - флаг подтверждения в payload привязан к составу позиций и сбрасывается
//     после попытки — литерал `true` сделал бы подтверждение вечным;
//   - попап поднимается из обработчиков сохранения, а не из пикера (D-06).
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он не доказывает, что попап поднимается в живом
// приложении (рантайм-ошибки рун, потеря состояния при {#key}) — это
// проверяется живой проверкой фазы.
//
// Правила (над текстом без комментариев):
//   (A) submitCreate и submitEdit: `needs_group_release` проверяется и его
//       ПОЗИЦИЯ меньше позиции первого `onSaved(` (порядок, не счёт).
//   (B) openGroupReleasePopup( вызывается внутри submitCreate и submitEdit;
//       ActFormItemsTable.svelte не содержит openGroupReleasePopup/groupRelease.
//   (C) closeGroupReleasePopup не содержит acts., onChange(, `items =`,
//       onSaved(, вызовов сохранения; confirmGroupRelease повторяет
//       handleSubmit(.
//   (D) GroupReleaseConfirmPopup.svelte: ровно две <Button — «Отмена»
//       (secondary, onclick={onClose}) и «Вывести и передать» (primary,
//       onclick={onConfirm}); нет destructive, нет «добавить всю группу», нет
//       {@html; у Modal initialFocus на secondary-кнопку подвала.
//   (E) payload create и update несут `confirm_group_release` на верхнем
//       уровне (не внутри number_input) со значением groupReleaseConfirmed();
//       groupReleaseConfirmed сравнивает сигнатуру состава позиций.
//   (F) ActFormModal рисует GroupReleaseConfirmPopup в ветке 'groupRelease';
//       ActFormBody объявляет и отправляет вариант kind: 'groupRelease'.
//   (G) подтверждение не липкое: handleSubmit (finally) и закрытия попапов
//       цепочки номера (taken, mismatch, scriptWarning) обнуляют
//       confirmedItemsSignature.
//
// Отсутствующий файл — нарушение, а не SKIP (гейт, пропускающий файл молча,
// нем навсегда).
//
// Self-test (`--selftest`): чистые РЕАЛЬНЫЕ файлы проходят; мутанты по реальному
// исходнику через mutate() с проверкой ровно одного вхождения якоря. Порядок
// защищён мутантами-ПЕРЕСТАНОВКАМИ (swap): блок needs_group_release переносится
// ниже ветки с onSaved( — удаление проходит гейт, поставленный ПОСЛЕ охраняемого.
//
// Zero-dependency: node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-act-group-confirm.mjs              # проверить репозиторий
//   node scripts/check-act-group-confirm.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-act-group-confirm.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-group-confirm]';

const FILES = {
  body: 'src/features/acts/ActFormBody.svelte',
  popup: 'src/features/acts/GroupReleaseConfirmPopup.svelte',
  modal: 'src/features/acts/ActFormModal.svelte',
  table: 'src/features/acts/ActFormItemsTable.svelte',
};

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

/** Тело `[async] function name(...) { ... }` или null. */
function functionBody(code, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

/** Тело объектного литерала `const <name>: <Type> = { ... }` или null. */
function objectLiteralBody(code, name, type) {
  const m = new RegExp(`const\\s+${name}\\s*:\\s*${type}\\s*=\\s*\\{`).exec(code);
  if (!m) return null;
  return extractBalanced(code, m.index + m[0].length - 1, '{', '}');
}

/** Только символы нулевой вложенности фигурных скобок (ключи верхнего уровня). */
function topLevel(body) {
  let depth = 0;
  let out = '';
  for (const c of body) {
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (depth === 0) out += c;
  }
  return out;
}

export function scanSource(files, violations) {
  const body = stripComments(files.body);
  const popup = stripComments(files.popup);
  const modal = stripComments(files.modal);
  const table = stripComments(files.table);

  const create = functionBody(body, 'submitCreate');
  const edit = functionBody(body, 'submitEdit');

  // (A) порядок: проверка исхода выше onSaved( в обоих путях сохранения.
  for (const [name, fn] of [
    ['submitCreate', create],
    ['submitEdit', edit],
  ]) {
    if (fn === null) {
      violations.push(`(A) ${FILES.body}: не найдена функция ${name} — антивакуум порядка.`);
      continue;
    }
    const checkAt = fn.indexOf('needs_group_release');
    const savedAt = fn.indexOf('onSaved(');
    if (checkAt === -1) {
      violations.push(
        `(A) ${FILES.body}: ${name} не проверяет исход needs_group_release — вынос из состава пройдёт без вопроса (D-06).`,
      );
    } else if (savedAt === -1) {
      violations.push(
        `(A) ${FILES.body}: в ${name} нет onSaved( — порядок нечем проверять (антивакуум).`,
      );
    } else if (checkAt > savedAt) {
      violations.push(
        `(A) ${FILES.body}: проверка needs_group_release в ${name} стоит ПОСЛЕ первого onSaved( — форма закроется «успехом» на ответе, который акта не создавал.`,
      );
    }
  }

  // (B) попап поднимается из обработчиков сохранения, не из пикера.
  if (functionBody(body, 'openGroupReleasePopup') === null) {
    violations.push(`(B) ${FILES.body}: не найдена функция openGroupReleasePopup.`);
  }
  for (const [name, fn] of [
    ['submitCreate', create],
    ['submitEdit', edit],
  ]) {
    if (fn !== null && !fn.includes('openGroupReleasePopup(')) {
      violations.push(`(B) ${FILES.body}: ${name} не вызывает openGroupReleasePopup(.`);
    }
  }
  if (/openGroupReleasePopup|groupRelease/.test(table)) {
    violations.push(
      `(B) ${FILES.table}: пикер позиций упоминает openGroupReleasePopup/groupRelease — вопрос задаётся по ходу набора, а не при сохранении (D-06).`,
    );
  }

  // (C) «Отмена» безопасна, «Вывести и передать» повторяет сохранение.
  const close = functionBody(body, 'closeGroupReleasePopup');
  if (close === null) {
    violations.push(`(C) ${FILES.body}: не найдена функция closeGroupReleasePopup — антивакуум.`);
  } else {
    const banned = [
      [/\bacts\./, 'вызов acts.*'],
      [/\bonChange\(/, 'onChange('],
      [/(?<![\w.])items\s*=(?!=)/, 'присваивание items'],
      [/\bonSaved\(/, 'onSaved('],
      [/\b(?:handleSubmit|submitCreate|submitEdit)\(/, 'повторное сохранение'],
    ];
    for (const [re, label] of banned) {
      if (re.test(close)) {
        violations.push(
          `(C) ${FILES.body}: closeGroupReleasePopup содержит ${label} — «Отмена» обязана только закрывать попап (D-07).`,
        );
      }
    }
  }
  const confirm = functionBody(body, 'confirmGroupRelease');
  if (confirm === null) {
    violations.push(`(C) ${FILES.body}: не найдена функция confirmGroupRelease — антивакуум.`);
  } else if (!/\bhandleSubmit\(/.test(confirm)) {
    violations.push(
      `(C) ${FILES.body}: confirmGroupRelease не повторяет handleSubmit( — подтверждение ничего не сохраняет.`,
    );
  }

  // (D) попап: две кнопки, варианты, фокус, текст.
  const buttons = [...popup.matchAll(/<Button\b([^>]*)>([\s\S]*?)<\/Button>/g)].map((m) => ({
    attrs: m[1],
    label: m[2].replace(/\s+/g, ' ').trim(),
  }));
  const openTags = (popup.match(/<Button\b/g) ?? []).length;
  if (openTags !== 2 || buttons.length !== 2) {
    violations.push(
      `(D) ${FILES.popup}: найдено <Button: ${openTags} (разобрано ${buttons.length}), нужно ровно две — «Отмена» и «Вывести и передать»; третьей кнопки нет (D-07).`,
    );
  }
  const cancel = buttons.find((b) => b.label === 'Отмена');
  const go = buttons.find((b) => b.label === 'Вывести и передать');
  if (!cancel) violations.push(`(D) ${FILES.popup}: нет кнопки «Отмена».`);
  else if (!/variant="secondary"/.test(cancel.attrs) || !/onclick=\{onClose\}/.test(cancel.attrs)) {
    violations.push(`(D) ${FILES.popup}: «Отмена» обязана быть secondary с onclick={onClose}.`);
  }
  if (!go) violations.push(`(D) ${FILES.popup}: нет кнопки «Вывести и передать».`);
  else if (!/variant="primary"/.test(go.attrs) || !/onclick=\{onConfirm\}/.test(go.attrs)) {
    violations.push(
      `(D) ${FILES.popup}: «Вывести и передать» обязана быть primary (не destructive, R-06) с onclick={onConfirm}.`,
    );
  }
  if (/destructive/.test(popup)) {
    violations.push(`(D) ${FILES.popup}: упомянут destructive — красная кнопка запрещена (R-06).`);
  }
  if (/добавить всю группу/i.test(popup)) {
    violations.push(`(D) ${FILES.popup}: есть «добавить всю группу» — третьей кнопки нет (D-07).`);
  }
  if (/\{@html/.test(popup)) {
    violations.push(
      `(D) ${FILES.popup}: {@html} — имена из ответа сервера только текстом (T-41.2-49).`,
    );
  }
  const modalTag = /<Modal\b[^>]*>/.exec(popup);
  if (!modalTag || !/initialFocus="[^"]*btn-secondary[^"]*"/.test(modalTag[0])) {
    violations.push(
      `(D) ${FILES.popup}: у <Modal> нет initialFocus на secondary-кнопку подвала — Enter по инерции подтвердит вынос (T-41.2-50).`,
    );
  }

  // (E) флаг в payload.
  for (const [name, literal] of [
    ['payload', objectLiteralBody(body, 'payload', 'ActCreateDto')],
    ['updatePayload', objectLiteralBody(body, 'updatePayload', 'ActUpdateDto')],
  ]) {
    if (literal === null) {
      violations.push(`(E) ${FILES.body}: не найден литерал ${name} — антивакуум payload.`);
      continue;
    }
    const top = topLevel(literal);
    if (!/\bconfirm_group_release\s*:\s*groupReleaseConfirmed\(\)/.test(top)) {
      violations.push(
        `(E) ${FILES.body}: ${name} не несёт на верхнем уровне \`confirm_group_release: groupReleaseConfirmed()\` (литерал или вложенность в number_input не годятся).`,
      );
    }
  }
  const confirmed = functionBody(body, 'groupReleaseConfirmed');
  if (confirmed === null) {
    violations.push(`(E) ${FILES.body}: не найдена функция groupReleaseConfirmed — антивакуум.`);
  } else if (!/===\s*itemsSignature\(\)/.test(confirmed) || !/!==\s*null/.test(confirmed)) {
    violations.push(
      `(E) ${FILES.body}: groupReleaseConfirmed не сравнивает сигнатуру состава с itemsSignature() — подтверждение переживёт смену позиций (T-41.2-51).`,
    );
  }

  // (F) попап подключён.
  if (
    !/numberPopup\?\.kind\s*===\s*'groupRelease'/.test(modal) ||
    !/<GroupReleaseConfirmPopup\b/.test(modal)
  ) {
    violations.push(
      `(F) ${FILES.modal}: нет ветки numberPopup?.kind === 'groupRelease' с <GroupReleaseConfirmPopup — попап не будет нарисован.`,
    );
  }
  if ((body.match(/kind:\s*'groupRelease'/g) ?? []).length < 2) {
    violations.push(
      `(F) ${FILES.body}: вариант kind: 'groupRelease' должен быть объявлен в union и отправлен через onPopupChange (нужно не меньше двух вхождений).`,
    );
  }

  // (G) подтверждение не липкое.
  const submit = functionBody(body, 'handleSubmit');
  if (submit === null) {
    violations.push(`(G) ${FILES.body}: не найдена функция handleSubmit — антивакуум.`);
  } else if (!/confirmedItemsSignature\s*=\s*null/.test(submit)) {
    violations.push(
      `(G) ${FILES.body}: handleSubmit не обнуляет confirmedItemsSignature после попытки — подтверждение липкое.`,
    );
  }
  for (const name of ['closeTakenPopup', 'closeMismatchPopup', 'closeScriptWarningPopup']) {
    const fn = functionBody(body, name);
    if (fn === null) {
      violations.push(`(G) ${FILES.body}: не найдена функция ${name} — антивакуум.`);
    } else if (!/confirmedItemsSignature\s*=\s*null/.test(fn)) {
      violations.push(
        `(G) ${FILES.body}: ${name} не обнуляет confirmedItemsSignature — подтверждение переживёт отказ цепочки номера.`,
      );
    }
  }
}

function readSources(root, violations) {
  const out = {};
  for (const [key, rel] of Object.entries(FILES)) {
    const abs = path.join(root, rel);
    try {
      out[key] = fs.readFileSync(abs, 'utf8');
    } catch {
      violations.push(
        `(A) ${rel}: файл не читается (${abs}); отсутствие файла — нарушение, не SKIP.`,
      );
      out[key] = null;
    }
  }
  return Object.values(out).some((v) => v === null) ? null : out;
}

/** Замена с проверкой, что якорь встречается ровно один раз. */
function mutate(source, from, to, label) {
  const occurrences = source.split(from).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `якорь мутанта «${label}» встречается ${occurrences} раз, нужен ровно 1 (иначе мутация отключит соседний фрагмент)`,
    );
  }
  return source.replace(from, () => to);
}

/** Замена внутри тела одной функции (якорь должен быть уникален в её теле). */
function mutateInFn(source, fnHead, from, to, label) {
  const start = source.indexOf(fnHead);
  if (start === -1 || source.indexOf(fnHead, start + 1) !== -1) {
    throw new Error(`мутант «${label}»: заголовок «${fnHead}» не найден ровно один раз`);
  }
  const braceOpen = source.indexOf('{', source.indexOf(')', start));
  const inner = extractBalanced(source, braceOpen, '{', '}');
  const end = braceOpen + 1 + inner.length;
  const slice = source.slice(start, end + 1);
  return source.slice(0, start) + mutate(slice, from, to, label) + source.slice(end + 1);
}

const GROUP_RELEASE_BRANCH =
  "    if (outcome.outcome === 'needs_group_release') {\n      openGroupReleasePopup(outcome);\n      return;\n    }\n";

/**
 * Перестановка (swap): ветка needs_group_release переносится ЗА ветку
 * с onSaved( внутри одной функции. Удаление ветки здесь не годится — оно не
 * доказывает, что гейт видит ПОРЯДОК.
 */
function swapBranchBelowSaved(source, fnHead, savedAnchor) {
  const start = source.indexOf(fnHead);
  if (start === -1 || source.indexOf(fnHead, start + 1) !== -1) {
    throw new Error(`мутант swap: заголовок «${fnHead}» не найден ровно один раз`);
  }
  const braceOpen = source.indexOf('{', source.indexOf(')', start));
  const inner = extractBalanced(source, braceOpen, '{', '}');
  const end = braceOpen + 1 + inner.length;
  const fn = source.slice(start, end + 1);
  const without = mutate(fn, GROUP_RELEASE_BRANCH, '', `swap-cut ${fnHead}`);
  const moved = mutate(
    without,
    savedAnchor,
    savedAnchor + GROUP_RELEASE_BRANCH,
    `swap-paste ${fnHead}`,
  );
  return source.slice(0, start) + moved + source.slice(end + 1);
}

function runSelfTest() {
  const raw = {};
  for (const [key, rel] of Object.entries(FILES)) {
    try {
      raw[key] = fs.readFileSync(path.join(UI_ROOT, rel), 'utf8');
    } catch (e) {
      console.error(`${TAG} [selftest] FAIL — реальный исходник не читается: ${e.message}`);
      return false;
    }
  }
  const clean = [];
  scanSource(raw, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальные исходники обязаны проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  const withFile = (key, src) => ({ ...raw, [key]: src });
  let mutants;
  try {
    mutants = [
      {
        name: '(1) ПЕРЕСТАНОВКА (swap) в submitCreate: ветка needs_group_release ниже ветки с onSaved(',
        files: withFile(
          'body',
          swapBranchBelowSaved(
            raw.body,
            'async function submitCreate(',
            "      pushToast('success', `Создан акт №${outcome.number}`);\n      onSaved(outcome);\n      return;\n    }\n",
          ),
        ),
        rule: 'A',
      },
      {
        name: '(2) ПЕРЕСТАНОВКА (swap) в submitEdit: ветка needs_group_release ниже ветки с onSaved(',
        files: withFile(
          'body',
          swapBranchBelowSaved(
            raw.body,
            'async function submitEdit(',
            "      pushToast('success', `Акт №${outcome.number} обновлён`);\n      onSaved(outcome);\n      return;\n    }\n",
          ),
        ),
        rule: 'A',
      },
      {
        name: '(3) в closeGroupReleasePopup добавлен вызов acts.create(payload)',
        files: withFile(
          'body',
          mutateInFn(
            raw.body,
            'function closeGroupReleasePopup(',
            '    confirmedItemsSignature = null;\n',
            '    confirmedItemsSignature = null;\n    void acts.create(payload);\n',
            'cancel-saves',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(4) «Вывести и передать» стала variant="destructive"',
        files: withFile(
          'popup',
          mutate(
            raw.popup,
            '<Button variant="primary" {loading}',
            '<Button variant="destructive" {loading}',
            'go-destructive',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(5) в попап добавлена третья кнопка',
        files: withFile(
          'popup',
          mutate(
            raw.popup,
            '    <Button variant="secondary" onclick={onClose}>Отмена</Button>\n',
            '    <Button variant="secondary" onclick={onClose}>Отмена</Button>\n    <Button variant="secondary" onclick={onClose}>Добавить всю группу</Button>\n',
            'third-button',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(6) из updatePayload убран confirm_group_release',
        files: withFile(
          'body',
          mutate(
            raw.body,
            '      items: updateItems,\n      confirm_group_release: groupReleaseConfirmed(),\n',
            '      items: updateItems,\n',
            'update-no-flag',
          ),
        ),
        rule: 'E',
      },
      {
        name: '(7) в пикер позиций добавлен вызов openGroupReleasePopup(',
        files: withFile(
          'table',
          raw.table +
            '\n<script lang="ts">\n  function ask() {\n    openGroupReleasePopup(null);\n  }\n</script>\n',
        ),
        rule: 'B',
      },
      {
        name: '(8) confirm_group_release в create-payload заменён литералом true',
        files: withFile(
          'body',
          mutate(
            raw.body,
            '      items: payloadItems,\n      confirm_group_release: groupReleaseConfirmed(),\n',
            '      items: payloadItems,\n      confirm_group_release: true,\n',
            'create-literal-true',
          ),
        ),
        rule: 'E',
      },
      {
        name: '(9) groupReleaseConfirmed больше не сравнивает сигнатуру состава',
        files: withFile(
          'body',
          mutate(
            raw.body,
            'return confirmedItemsSignature !== null && confirmedItemsSignature === itemsSignature();',
            'return confirmedItemsSignature !== null;',
            'confirmed-no-compare',
          ),
        ),
        rule: 'E',
      },
      {
        name: '(10) у Modal попапа снят initialFocus',
        files: withFile(
          'popup',
          mutate(
            raw.popup,
            '  initialFocus=".modal-footer .btn-secondary"\n',
            '',
            'no-initial-focus',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(11) ActFormModal не рисует попап (ветка groupRelease удалена)',
        files: withFile(
          'modal',
          mutate(
            raw.modal,
            "{:else if numberPopup?.kind === 'groupRelease'}",
            '{:else if false}',
            'no-branch',
          ),
        ),
        rule: 'F',
      },
      {
        name: '(12) handleSubmit не обнуляет confirmedItemsSignature (подтверждение липкое)',
        files: withFile(
          'body',
          mutateInFn(
            raw.body,
            'async function handleSubmit(',
            '      if (activePopup === null) confirmedItemsSignature = null;\n',
            '',
            'sticky',
          ),
        ),
        rule: 'G',
      },
      {
        name: '(13) closeTakenPopup не обнуляет confirmedItemsSignature',
        files: withFile(
          'body',
          mutateInFn(
            raw.body,
            'function closeTakenPopup(',
            '    confirmedItemsSignature = null;\n',
            '',
            'taken-sticky',
          ),
        ),
        rule: 'G',
      },
      {
        name: '(14) confirmGroupRelease не повторяет сохранение',
        files: withFile(
          'body',
          mutateInFn(
            raw.body,
            'function confirmGroupRelease(',
            '    void handleSubmit();\n',
            '',
            'no-resave',
          ),
        ),
        rule: 'C',
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  let caught = 0;
  for (const m of mutants) {
    const v = [];
    scanSource(m.files, v);
    const hit = v.some((line) => line.startsWith(`(${m.rule})`));
    console.error(
      `${TAG} [selftest] ${hit ? 'PASS' : 'FAIL'} — ${m.name} (ожидалось ${m.rule}, получено ${v.length})`,
    );
    if (hit) caught++;
    else {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  // Отсутствие файла — красное, не SKIP.
  {
    const tmp = fs.mkdtempSync(
      path.join(fs.realpathSync(process.env.TMPDIR ?? '/tmp'), 'actconfirm-'),
    );
    try {
      for (const rel of [FILES.body, FILES.popup, FILES.modal]) {
        fs.mkdirSync(path.dirname(path.join(tmp, rel)), { recursive: true });
        fs.writeFileSync(
          path.join(tmp, rel),
          raw[Object.keys(FILES).find((k) => FILES[k] === rel)],
        );
      }
      const v = [];
      const got = readSources(tmp, v);
      const ok =
        got === null && v.some((line) => line.startsWith('(A)') && line.includes('не SKIP'));
      console.error(
        `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — отсутствующий ActFormItemsTable.svelte даёт нарушение, а не SKIP`,
      );
      if (ok) caught++;
      else failed++;
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  }

  // Зелёный контроль: запретные слова в КОММЕНТАРИИ не краснят.
  try {
    const commented = mutate(
      raw.body,
      '  // «Отмена», Esc, крестик, оверлей: back to the form',
      '  // раньше: acts.create( и onSaved( и items = [] в closeGroupReleasePopup\n  // «Отмена», Esc, крестик, оверлей: back to the form',
      'comment-ok',
    );
    const v = [];
    scanSource(withFile('body', commented), v);
    const ok = v.length === 0;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — запретные слова в комментарии не краснят (ожидалось 0, получено ${v.length})`,
    );
    if (!ok) failed++;
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(`${TAG} [selftest] PASS — реальные исходники чисты, мутантов поймано: ${caught}.`);
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-act-group-confirm.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  const files = readSources(root, violations);
  if (files !== null) scanSource(files, violations);
  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
