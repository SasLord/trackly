#!/usr/bin/env node
// [check-printer-usb-host-field] Структурный гейт поля «Устройство-хост (USB)» принтера
// (фаза 41.13, PGR-03/PGR-04): конфигурация Dropdown, видимость, payload'ы создания и
// правки, блокировка «Количества», заголовок второй секции и деталь принтера.
//
// Почему он существует. svelte-check, eslint и pnpm build слепы к тому, что поле
// собрано не на том компоненте, что хост потерялся в payload'е или что в детали
// принтера снова печатается сырой «#id»: всё это типизируется одинаково. JS-тест-
// раннера в проекте нет, поэтому конфигурация закрепляется грепающим гейтом.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он НЕ доказывает, что заголовки секций рисуются,
// что панель не перекрывает соседние поля и что руны Svelte 5 работают в рантайме:
// это проверяется только в живом приложении (план 08 фазы 41.13).
//
// Правила (над текстом без комментариев):
//   (A) Dropdown с id="f-printer-usb-host": именно <Dropdown>, variant="combobox",
//       flat={true}, getGroupSection; variant="select" в теге нет;
//   (B) submitEdit: usb_host_device_id попадает в patch ТОЛЬКО внутри блока, чьё
//       условие содержит usbHostTouched, PRINTER_TYPE_ID и showUsbHost (иначе правка
//       без касания поля затирает хост);
//   (C) submitSingleCreate (D-15, IP и хост независимы): usbHostDeviceId лежит в том же
//       литерале блока принтера, что и ipAddress:, условие блока упоминает И ipAddress,
//       И usbHost, а выражение после usbHostDeviceId: НЕ упоминает ipAddress;
//   (D) quantityDisabled содержит `usbHost !== null` (хост блокирует «Количество»);
//   (E) поле хоста не обёрнуто условием, принудительно требующим !isEdit (оно
//       доступно при правке), и обёрнуто условием с showUsbHost;
//   (F) DeviceFormBody не содержит member_of_group_id;
//   (G) PrinterDetail: нет «устройство #», usbHostDeviceId не печатается в тексте
//       разметки, есть usbHostName;
//   (H) usbHostPicker.ts: SECTION_ALL = 'Все устройства', sectionFor возвращает его
//       ТОЛЬКО при hasSectionOne (порядок ветвей важен), isUsbHostFieldVisible
//       экспортирован; DeviceFormBody вызывает isUsbHostFieldVisible( в showUsbHost.
// Поправка D-13/D-15 (2026-11-01: «принтер может быть связан по USB и в это же время быть
// сетевым»):
//   (B2) submitEdit: охрана присваивания usb_host_device_id не упоминает ipAddress и
//       loadedPrinterIp (ключ хоста не зависит от IP; иначе хост молча перестал бы уходить
//       у принтера с IP);
//   (E+) ни одно условие вокруг поля хоста не упоминает ipAddress / loadedPrinterIp;
//   (I) usbHostPicker.ts: в сигнатуре и теле isUsbHostFieldVisible нет ipAddress,
//       loadedPrinterIp, hasSavedHost; вызов в форме не передаёт ipAddress и loadedPrinterIp;
//   (J) поля f-printer-ip и f-printer-community не под условием, требующим !isEdit, и
//       блок ограничен типом «Принтер» (DOM отсутствует вне него);
//   (K) onMount: в обработчике getByDeviceId.then есть ipAddress = ...p.ipAddress
//       (предзаполнение IP при правке);
//   (L) submitEdit: printer_ip_address присваивается внутри блока с PRINTER_TYPE_ID и
//       printerLoaded и внутри блока со сравнением с loadedPrinterIp, значением с null
//       (стёртый IP уходит как null);
//   (M) community инициализируется не голым 'public' (зависит от target) и
//       printer_community уходит только в блоке с непустой community и непустым IP;
//   (N) в сыром тексте формы есть D-13 и D-15 и нет «create+Принтер ONLY» (отмена D-03
//       фазы 40.3 записана у кода);
//   (O) в разметке нет подсказки «Оставьте пустым для USB» (ложная посылка);
//   (P) PrinterDetail: строка хоста — самостоятельный {#if printer.usbHostDeviceId}, а не
//       {:else if} после строки IP.
// Файл или функцию не разобрать — нарушение, не молчаливый PASS; отсутствие файла —
// красный, не SKIP.
//
// Self-test (`--selftest`): чистый РЕАЛЬНЫЙ исходник обязан пройти; затем мутанты через
// mutate() с проверкой ровно одного вхождения якоря (иначе мутант мог бы снять гейт у
// соседнего фрагмента), в том числе перестановки ветвей, а не только удаления.
//
// Zero-dependency: node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-printer-usb-host-field.mjs              # проверить репозиторий
//   node scripts/check-printer-usb-host-field.mjs --src=<dir>  # проверить копию каталога ui/
//   node scripts/check-printer-usb-host-field.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-printer-usb-host-field]';

const FORM = 'src/features/devices/DeviceFormBody.svelte';
const DETAIL = 'src/features/printers/PrinterDetail.svelte';
const PICKER = 'src/features/devices/usbHostPicker.ts';
const FIELD_ID = 'f-printer-usb-host';

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

/** Тексты всех блоков <script> (module и instance) из уже очищенного от комментариев текста. */
function scriptBlocks(code) {
  const parts = [...code.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1]);
  return parts.length > 0 ? parts.join('\n') : null;
}

/** Тело `function name(...) { ... }` (с началом и концом в исходнике) или null. */
function functionBody(code, name) {
  const m = new RegExp(`\\bfunction\\s+${name}\\s*\\(`).exec(code);
  if (!m) return null;
  const parenOpen = code.indexOf('(', m.index);
  const params = extractBalanced(code, parenOpen, '(', ')');
  if (params === null) return null;
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  const body = extractBalanced(code, braceOpen, '{', '}');
  return body === null ? null : { body, start: braceOpen + 1 };
}

/** Заголовки всех блоков `{`, внутри которых лежит позиция pos (от внутреннего к внешнему). */
function enclosingHeaders(code, pos) {
  const stack = [];
  for (let i = 0; i < pos; i++) {
    const c = code[i];
    if (c === '{') stack.push(i);
    else if (c === '}') stack.pop();
  }
  return stack.reverse().map((open) => {
    let j = open - 1;
    while (j >= 0 && !';{}'.includes(code[j])) j--;
    return code.slice(j + 1, open);
  });
}

/** Тело самого внутреннего блока `{ ... }`, содержащего позицию pos. */
function innermostBlock(code, pos) {
  const stack = [];
  for (let i = 0; i < pos; i++) {
    const c = code[i];
    if (c === '{') stack.push(i);
    else if (c === '}') stack.pop();
  }
  if (stack.length === 0) return null;
  return extractBalanced(code, stack[stack.length - 1], '{', '}');
}

/** Тег <Name ...> до закрывающего `>` на нулевой глубине фигурных скобок. */
function openingTagAt(code, idx) {
  let last = null;
  const re = /<([A-Za-z][\w.]*)\b/g;
  for (let m = re.exec(code.slice(0, idx)); m; m = re.exec(code.slice(0, idx))) last = m;
  if (!last) return null;
  const start = last.index;
  let depth = 0;
  for (let i = start; i < code.length; i++) {
    const c = code[i];
    if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0)
      return { name: last[1], text: code.slice(start, i + 1), start };
  }
  return null;
}

/** Условия всех {#if}/{:else if}, внутри которых лежит позиция pos разметки. */
function enclosingIfConditions(code, pos) {
  const stack = [];
  const re =
    /\{(#if|:else if|:else|\/if|#each|\/each|#key|\/key|#await|\/await|#snippet|\/snippet)\b/g;
  for (let m = re.exec(code); m && m.index < pos; m = re.exec(code)) {
    const open = m.index;
    const inner = extractBalanced(code, open, '{', '}') ?? '';
    const kind = m[1];
    if (kind === '#if') stack.push({ kind: 'if', cond: inner.slice(3).trim() });
    else if (kind === ':else if') {
      const top = stack[stack.length - 1];
      if (top) top.cond = inner.slice(8).trim();
    } else if (kind === ':else') {
      const top = stack[stack.length - 1];
      if (top) top.cond = '';
    } else if (kind.startsWith('#')) stack.push({ kind: 'other', cond: '' });
    else stack.pop();
  }
  return stack.filter((s) => s.kind === 'if').map((s) => s.cond);
}

/** Условие принудительно требует !isEdit: верхнеуровневый && -конъюнкт `!isEdit`. */
function forcesNotEdit(cond) {
  const c = cond.trim();
  const topLevel = (s, op) => {
    const parts = [];
    let depth = 0;
    let from = 0;
    for (let i = 0; i < s.length; i++) {
      if (s[i] === '(') depth++;
      else if (s[i] === ')') depth--;
      else if (depth === 0 && s.startsWith(op, i)) {
        parts.push(s.slice(from, i));
        from = i + op.length;
        i += op.length - 1;
      }
    }
    parts.push(s.slice(from));
    return parts;
  };
  if (topLevel(c, '||').length > 1) return false;
  return topLevel(c, '&&').some((part) => {
    const p = part.trim();
    if (p === '!isEdit' || p === 'isEdit === false') return true;
    if (p.startsWith('(') && p.endsWith(')')) return forcesNotEdit(p.slice(1, -1));
    return false;
  });
}

function scanForm(raw, violations) {
  const code = stripComments(raw);

  // (F)
  if (/\bmember_of_group_id\b/.test(code)) {
    violations.push(
      `(F) ${FORM}: найден member_of_group_id. Пикер хоста видит все устройства, без привязки к группе.`,
    );
  }

  // (A), (E): тег поля.
  const ids = [...code.matchAll(new RegExp(`\\bid="${FIELD_ID}"`, 'g'))];
  if (ids.length !== 1) {
    violations.push(
      `(A) ${FORM}: id="${FIELD_ID}" встречается ${ids.length} раз, нужен ровно один.`,
    );
  } else {
    const idx = ids[0].index;
    const tag = openingTagAt(code, idx);
    if (!tag) {
      violations.push(`(A) ${FORM}: не удалось разобрать тег поля ${FIELD_ID}.`);
    } else {
      if (tag.name !== 'Dropdown') {
        violations.push(
          `(A) ${FORM}: поле ${FIELD_ID} собрано на <${tag.name}>, нужен собственный <Dropdown> (не нативный select и не обёртка Select).`,
        );
      }
      if (!/\bvariant="combobox"/.test(tag.text)) {
        violations.push(`(A) ${FORM}: у поля ${FIELD_ID} нет variant="combobox".`);
      }
      if (/\bvariant="select"/.test(tag.text)) {
        violations.push(`(A) ${FORM}: у поля ${FIELD_ID} variant="select".`);
      }
      if (!/\bflat=\{true\}/.test(tag.text)) {
        violations.push(
          `(A) ${FORM}: у поля ${FIELD_ID} нет flat={true} (без него схлопывание AUTO-05 прячет серверные результаты).`,
        );
      }
      if (!/\bgetGroupSection=/.test(tag.text)) {
        violations.push(`(A) ${FORM}: у поля ${FIELD_ID} нет getGroupSection (заголовки секций).`);
      }

      const conds = enclosingIfConditions(code, tag.start);
      for (const cond of conds) {
        if (forcesNotEdit(cond)) {
          violations.push(
            `(E) ${FORM}: поле хоста обёрнуто условием «${cond}», которое требует !isEdit. Поле обязано быть доступно при правке.`,
          );
        }
      }
      for (const cond of conds) {
        if (/\b(ipAddress|loadedPrinterIp|hasSavedHost)\b/.test(cond)) {
          violations.push(
            `(E) ${FORM}: поле хоста обёрнуто условием «${cond}», которое смотрит на IP. IP и хост независимы (D-15): поле хоста не прячется из-за IP.`,
          );
        }
      }
      if (conds.length === 0 || !/\bshowUsbHost\b/.test(conds[conds.length - 1])) {
        violations.push(
          `(E) ${FORM}: ближайшее условие вокруг поля хоста не содержит showUsbHost: видимость решает isUsbHostFieldVisible.`,
        );
      }
    }
  }

  const sc = scriptBlocks(code);
  if (sc === null) {
    violations.push(`(A) ${FORM}: нет блока <script>.`);
    return;
  }

  // (B)
  const edit = functionBody(sc, 'submitEdit');
  if (!edit) {
    violations.push(
      `(B) ${FORM}: не найдена функция submitEdit. Если она переименована, обнови гейт.`,
    );
  } else {
    const hits = [...edit.body.matchAll(/\busb_host_device_id\b/g)];
    if (hits.length === 0) {
      violations.push(
        `(B) ${FORM}: в submitEdit нет usb_host_device_id: правка не сможет менять хост.`,
      );
    }
    for (const h of hits) {
      const headers = enclosingHeaders(edit.body, h.index);
      const guarded = headers.some(
        (hd) =>
          /\bif\b/.test(hd) &&
          /\busbHostTouched\b/.test(hd) &&
          /\bPRINTER_TYPE_ID\b/.test(hd) &&
          /\bshowUsbHost\b/.test(hd),
      );
      const ipGuarded = headers.some(
        (hd) => /\bif\b/.test(hd) && /\b(ipAddress|loadedPrinterIp|hasSavedHost)\b/.test(hd),
      );
      if (ipGuarded) {
        violations.push(
          `(B2) ${FORM}: охрана присваивания usb_host_device_id в submitEdit смотрит на IP. Ключ хоста не зависит от IP (D-15): иначе хост молча перестанет уходить у принтера с IP.`,
        );
      }
      if (!guarded) {
        violations.push(
          `(B) ${FORM}: usb_host_device_id в submitEdit вне блока с условием usbHostTouched && PRINTER_TYPE_ID && showUsbHost: правка без касания поля затрёт хост.`,
        );
      }
    }
  }

  // (C)
  const create = functionBody(sc, 'submitSingleCreate');
  if (!create) {
    violations.push(
      `(C) ${FORM}: не найдена функция submitSingleCreate. Если она переименована, обнови гейт.`,
    );
  } else {
    const hits = [...create.body.matchAll(/\busbHostDeviceId\b/g)];
    if (hits.length === 0) {
      violations.push(
        `(C) ${FORM}: в submitSingleCreate нет usbHostDeviceId: блок принтера не несёт хост.`,
      );
    }
    for (const h of hits) {
      const headers = enclosingHeaders(create.body, h.index);
      if (
        !headers.some(
          (hd) => /\bif\b/.test(hd) && /\busbHost\b/.test(hd) && /\bipAddress\b/.test(hd),
        )
      ) {
        violations.push(
          `(C) ${FORM}: условие блока принтера в submitSingleCreate обязано упоминать и ipAddress, и usbHost: блок строится, если задан хотя бы один из двух (D-15).`,
        );
      }
      const literal = innermostBlock(create.body, h.index) ?? '';
      if (!/\bipAddress\s*:/.test(literal)) {
        violations.push(
          `(C) ${FORM}: usbHostDeviceId в submitSingleCreate не в одном литерале с ipAddress: при создании блок должен нести оба значения (D-15).`,
        );
      }
      const val = /\busbHostDeviceId\s*:\s*([^,}]*)/.exec(literal);
      if (val && /\bipAddress\b/.test(val[1])) {
        violations.push(
          `(C) ${FORM}: значение usbHostDeviceId в submitSingleCreate зависит от ipAddress. IP и хост независимы (D-15): хост не пропадает из-за IP.`,
        );
      }
    }
  }

  // (D)
  const qd = /\bconst\s+quantityDisabled\s*=\s*\$derived\(/.exec(sc);
  if (!qd) {
    violations.push(`(D) ${FORM}: не найдено объявление quantityDisabled.`);
  } else {
    const expr =
      extractBalanced(
        sc,
        sc.indexOf('(', qd.index + 'const quantityDisabled = $derived'.length - 1),
        '(',
        ')',
      ) ?? '';
    if (!/\busbHost\s*!==\s*null\b/.test(expr)) {
      violations.push(
        `(D) ${FORM}: quantityDisabled не содержит usbHost !== null: массовое создание потеряет хост (bulkCreate блок принтера не несёт).`,
      );
    }
  }

  // (H, часть формы)
  if (!/\bshowUsbHost\s*=\s*\$derived\(\s*isUsbHostFieldVisible\(/.test(sc)) {
    violations.push(
      `(H) ${FORM}: showUsbHost не строится вызовом isUsbHostFieldVisible(: условие видимости не пишется в разметке вручную.`,
    );
  }

  // (I, часть формы): вызов не передаёт IP.
  const callAt = sc.indexOf('isUsbHostFieldVisible(');
  if (callAt !== -1) {
    const callArgs = extractBalanced(sc, callAt + 'isUsbHostFieldVisible'.length, '(', ')') ?? '';
    if (/\b(ipAddress|loadedPrinterIp|hasSavedHost)\b/.test(callArgs)) {
      violations.push(
        `(I) ${FORM}: вызов isUsbHostFieldVisible передаёт IP. Видимость хоста не зависит от IP (D-15).`,
      );
    }
  }

  // (J): блок IP/community доступен при правке и ограничен типом «Принтер».
  for (const id of ['f-printer-ip', 'f-printer-community']) {
    const hit = new RegExp(`\\bid="${id}"`).exec(code);
    if (!hit) {
      violations.push(`(J) ${FORM}: нет поля id="${id}".`);
      continue;
    }
    const tag = openingTagAt(code, hit.index);
    if (!tag) {
      violations.push(`(J) ${FORM}: не удалось разобрать тег поля ${id}.`);
      continue;
    }
    const conds = enclosingIfConditions(code, tag.start);
    for (const cond of conds) {
      if (forcesNotEdit(cond)) {
        violations.push(
          `(J) ${FORM}: поле ${id} обёрнуто условием «${cond}», которое требует !isEdit. Блок IP/community обязан быть доступен при правке (D-13).`,
        );
      }
    }
    if (!conds.some((c) => /\bPRINTER_TYPE_ID\b/.test(c))) {
      violations.push(
        `(J) ${FORM}: поле ${id} не ограничено типом «Принтер»: DOM обязан отсутствовать вне него.`,
      );
    }
  }

  // (K): предзаполнение IP при правке.
  const thenM = /\.getByDeviceId\([^)]*\)\s*\.then\(\(?(\w+)\)?\s*=>\s*\{/.exec(sc);
  if (!thenM) {
    violations.push(`(K) ${FORM}: не найден обработчик printers.getByDeviceId(...).then.`);
  } else {
    const thenBody = extractBalanced(sc, thenM.index + thenM[0].length - 1, '{', '}') ?? '';
    if (!new RegExp(`\\bipAddress\\s*=[^;]*\\b${thenM[1]}\\.ipAddress\\b`).test(thenBody)) {
      violations.push(
        `(K) ${FORM}: в обработчике getByDeviceId нет предзаполнения ipAddress из ${thenM[1]}.ipAddress: при правке поле IP осталось бы пустым.`,
      );
    }
  }

  // (L), (M): ключи IP и community в submitEdit, начальное значение community.
  if (edit) {
    const ipHits = [...edit.body.matchAll(/\bprinter_ip_address\b/g)];
    if (ipHits.length === 0) {
      violations.push(
        `(L) ${FORM}: в submitEdit нет printer_ip_address: IP нельзя изменить при правке.`,
      );
    }
    for (const h of ipHits) {
      const headers = enclosingHeaders(edit.body, h.index).filter((hd) => /\bif\b/.test(hd));
      if (!headers.some((hd) => /\bPRINTER_TYPE_ID\b/.test(hd) && /\bprinterLoaded\b/.test(hd))) {
        violations.push(
          `(L) ${FORM}: printer_ip_address в submitEdit вне блока с PRINTER_TYPE_ID и printerLoaded: до ответа сервера пустой IP стёр бы сохранённый.`,
        );
      }
      if (!headers.some((hd) => /\bloadedPrinterIp\b/.test(hd))) {
        violations.push(
          `(L) ${FORM}: printer_ip_address в submitEdit вне блока со сравнением с loadedPrinterIp: правка названия пересылала бы IP.`,
        );
      }
      const stmt = edit.body.slice(h.index, edit.body.indexOf(';', h.index));
      if (!/\bnull\b/.test(stmt)) {
        violations.push(
          `(L) ${FORM}: присваивание printer_ip_address без null: стёртый IP не отправится как null.`,
        );
      }
    }
    const cHits = [...edit.body.matchAll(/\bprinter_community\b/g)];
    if (cHits.length === 0) {
      violations.push(`(M) ${FORM}: в submitEdit нет printer_community.`);
    }
    for (const h of cHits) {
      const headers = enclosingHeaders(edit.body, h.index).filter((hd) => /\bif\b/.test(hd));
      if (
        !headers.some(
          (hd) => /\bcommunity\.trim\(\)\s*!==\s*''/.test(hd) && /\bipNow\s*!==\s*''/.test(hd),
        )
      ) {
        violations.push(
          `(M) ${FORM}: printer_community в submitEdit вне блока «community непуста И IP непуст»: пустая community затёрла бы настоящую.`,
        );
      }
    }
  }
  const comm = /\blet\s+community\s*=\s*\$state\(/.exec(sc);
  if (!comm) {
    violations.push(`(M) ${FORM}: не найдено объявление community.`);
  } else {
    const init = (extractBalanced(sc, comm.index + comm[0].length - 1, '(', ')') ?? '').trim();
    if (init === "'public'" || !/\b(target|isEdit)\b/.test(init)) {
      violations.push(
        `(M) ${FORM}: community инициализируется «${init}»: при правке 'public' затёр бы настоящую community, начальное значение обязано зависеть от target.`,
      );
    }
  }

  // (N): отмена D-03 фазы 40.3 записана у кода (сырой текст, с комментариями).
  if (!/\bD-13\b/.test(raw) || !/\bD-15\b/.test(raw)) {
    violations.push(`(N) ${FORM}: в комментариях нет D-13 и D-15.`);
  }
  if (/create\+Принтер ONLY/.test(raw)) {
    violations.push(
      `(N) ${FORM}: вернулось «create+Принтер ONLY»: решение D-03 фазы 40.3 отменено D-13.`,
    );
  }

  // (O): подсказка блока SNMP без ложной посылки.
  if (/Оставьте\s+пустым\s+для\s+USB/.test(code)) {
    violations.push(
      `(O) ${FORM}: в разметке подсказка «Оставьте пустым для USB…»: IP и USB-хост независимы (D-15).`,
    );
  }
}

function scanDetail(raw, violations) {
  const code = stripComments(raw);
  if (/устройство\s*#/.test(code)) {
    violations.push(
      `(G) ${DETAIL}: найден шаблон «устройство #»: вместо имени хоста печатается сырой id.`,
    );
  }
  if (/\{(?![:#/@])[^{}]*\busbHostDeviceId\b/.test(code)) {
    violations.push(
      `(G) ${DETAIL}: usbHostDeviceId печатается в тексте разметки: нужно имя хоста.`,
    );
  }
  if (/\{:else if\s+printer\.usbHostDeviceId\s*\}/.test(code)) {
    violations.push(
      `(P) ${DETAIL}: строка хоста стала веткой {:else if} после строки IP. IP и хост независимы (D-15): принтер с обоими значениями не покажет хост.`,
    );
  }
  if (!/\{#if\s+printer\.usbHostDeviceId\s*\}/.test(code)) {
    violations.push(`(P) ${DETAIL}: нет самостоятельного {#if printer.usbHostDeviceId}.`);
  }
  if (!/\{[^{}]*\bprinter\.usbHostName\b[^{}]*\}/.test(code)) {
    violations.push(
      `(G) ${DETAIL}: нет usbHostName: деталь принтера обязана показывать имя хоста.`,
    );
  }
}

function scanPicker(raw, violations) {
  const code = stripComments(raw);
  if (!/\bexport\s+const\s+SECTION_ALL\s*=\s*'Все устройства'\s*;/.test(code)) {
    violations.push(`(H) ${PICKER}: нет export const SECTION_ALL = 'Все устройства'.`);
  }
  if (!/\bexport\s+function\s+isUsbHostFieldVisible\s*\(/.test(code)) {
    violations.push(`(H) ${PICKER}: isUsbHostFieldVisible не экспортирована.`);
  }
  const vis = /\bexport\s+function\s+isUsbHostFieldVisible\s*\(/.exec(code);
  const visBody = functionBody(code, 'isUsbHostFieldVisible');
  if (vis && visBody) {
    const text = code.slice(vis.index, visBody.start + visBody.body.length + 1);
    const bad = /\b(ipAddress|loadedPrinterIp|hasSavedHost)\b/.exec(text);
    if (bad) {
      violations.push(
        `(I) ${PICKER}: в сигнатуре или теле isUsbHostFieldVisible есть «${bad[1]}». Видимость хоста не зависит от IP ни в одной ветке (D-15).`,
      );
    }
  } else if (vis) {
    violations.push(`(I) ${PICKER}: не удалось разобрать isUsbHostFieldVisible.`);
  }
  const sf = functionBody(code, 'sectionFor');
  if (!sf) {
    violations.push(`(H) ${PICKER}: не найдена функция sectionFor.`);
    return;
  }
  const all = String.raw`(?:SECTION_ALL|'Все устройства')`;
  const ternary = new RegExp(String.raw`\bhasSectionOne\s*\?\s*${all}\s*:\s*undefined\b`);
  const early = new RegExp(
    String.raw`if\s*\(\s*!\s*ctx\.hasSectionOne\s*\)\s*return\s+undefined\s*;[\s\S]*return\s+${all}\s*;`,
  );
  if (!ternary.test(sf.body) && !early.test(sf.body)) {
    violations.push(
      `(H) ${PICKER}: sectionFor не возвращает «Все устройства» ТОЛЬКО при hasSectionOne (условие в теле, ветви в правильном порядке).`,
    );
  }
}

function scanAll(files, violations) {
  scanForm(files.form, violations);
  scanDetail(files.detail, violations);
  scanPicker(files.picker, violations);
}

function readAll(root, violations) {
  const out = {};
  for (const [key, rel] of [
    ['form', FORM],
    ['detail', DETAIL],
    ['picker', PICKER],
  ]) {
    try {
      out[key] = fs.readFileSync(path.join(root, rel), 'utf8');
    } catch {
      violations.push(`(A) ${rel}: файл не читается; отсутствие файла — нарушение, не SKIP.`);
      return null;
    }
  }
  return out;
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

function runSelfTest() {
  const violations = [];
  const real = readAll(UI_ROOT, violations);
  if (!real) {
    for (const v of violations) console.error(`${TAG} [selftest] FAIL — ${v}`);
    return false;
  }
  const clean = [];
  scanAll(real, clean);
  if (clean.length > 0) {
    console.error(`${TAG} [selftest] FAIL — реальный исходник обязан проходить:`);
    for (const v of clean) console.error(`${TAG} [selftest]   ${v}`);
    return false;
  }

  const form = (from, to, label) => ({ ...real, form: mutate(real.form, from, to, label) });
  const detail = (from, to, label) => ({ ...real, detail: mutate(real.detail, from, to, label) });
  const picker = (from, to, label) => ({ ...real, picker: mutate(real.picker, from, to, label) });
  /** Несколько последовательных замен в picker: каждая со счётом ровно одного вхождения. */
  const picker2 = (steps) => ({
    ...real,
    picker: steps.reduce((acc, [from, to, label]) => mutate(acc, from, to, label), real.picker),
  });

  const TOUCH_COND = 'if (usbHostTouched && typeId === PRINTER_TYPE_ID && showUsbHost) {';
  const TOUCH_BLOCK = `${TOUCH_COND}\n      patch.usb_host_device_id = usbHost?.id ?? null;\n    }`;

  let mutants;
  try {
    mutants = [
      {
        name: '(1) variant="combobox" -> variant="select"',
        files: form('variant="combobox"', 'variant="select"', 'select'),
        rules: ['A'],
      },
      {
        name: '(2) убран flat={true} у поля хоста',
        files: form(`flat={true}\n              id="${FIELD_ID}"`, `id="${FIELD_ID}"`, 'no-flat'),
        rules: ['A'],
      },
      {
        name: '(3) убран getGroupSection',
        files: form(
          '              getGroupSection={(h: HostItem) => sectionFor(h, { hasSectionOne, sectionOneTitle })}\n',
          '',
          'no-section',
        ),
        rules: ['A'],
      },
      {
        name: '(4) поле собрано на <Select>, а не на <Dropdown>',
        files: form(
          '<Dropdown\n              variant="combobox"',
          '<Select\n              variant="combobox"',
          'tag-select',
        ),
        rules: ['A'],
      },
      {
        name: '(5) usbHostTouched убран из условия патча',
        files: form(TOUCH_COND, 'if (typeId === PRINTER_TYPE_ID && showUsbHost) {', 'no-touched'),
        rules: ['B'],
      },
      {
        name: '(6) ПЕРЕСТАНОВКА: присваивание хоста вынесено ПЕРЕД условием',
        files: form(
          TOUCH_BLOCK,
          `patch.usb_host_device_id = usbHost?.id ?? null;\n    ${TOUCH_COND}\n    }`,
          'swap-assign',
        ),
        rules: ['B'],
      },
      {
        name: '(7) showUsbHost убран из условия патча (скрытое поле затрёт хост)',
        files: form(
          TOUCH_COND,
          'if (usbHostTouched && typeId === PRINTER_TYPE_ID) {',
          'no-visible',
        ),
        rules: ['B'],
      },
      {
        name: '(8) usbHostDeviceId убран из блока принтера',
        files: form('usbHostDeviceId: usbHost?.id ?? null,', '', 'no-create-host'),
        rules: ['C'],
      },
      {
        name: '(9) ЛОЖНАЯ ПОСЫЛКА ВЕРНУЛАСЬ: хост при создании только при пустом IP',
        files: form(
          'usbHostDeviceId: usbHost?.id ?? null,',
          "usbHostDeviceId: ipAddress.trim() === '' ? (usbHost?.id ?? null) : null,",
          'host-only-without-ip',
        ),
        rules: ['C'],
      },
      {
        name: '(10) usbHost убран из quantityDisabled',
        files: form('(typeId === PRINTER_TYPE_ID && usbHost !== null),', 'false,', 'no-qty-block'),
        rules: ['D'],
      },
      {
        name: '(11) поле обёрнуто условием !isEdit',
        files: form('{#if showUsbHost}', '{#if !isEdit && showUsbHost}', 'not-edit'),
        rules: ['E'],
      },
      {
        name: '(12) поле больше не зависит от showUsbHost',
        files: form('{#if showUsbHost}', '{#if true}', 'always-visible'),
        rules: ['E'],
      },
      {
        name: '(13) member_of_group_id в форме',
        files: form(
          'let hostSeq = 0;',
          'let hostSeq = 0;\n  void { member_of_group_id: 1 };',
          'anchor-flag',
        ),
        rules: ['F'],
      },
      {
        name: '(14) в детали принтера вернулось «устройство #»',
        files: detail(
          '<span class="meta-label">Подключение</span>',
          '<span class="meta-label">Подключение</span><span>устройство #{printer.usbHostDeviceId}</span>',
          'device-hash',
        ),
        rules: ['G'],
      },
      {
        name: '(15) usbHostDeviceId напечатан в тексте разметки',
        files: detail(
          '<span class="meta-label">Подключение</span>',
          '<span class="meta-label">Подключение</span><span>{printer.usbHostDeviceId}</span>',
          'raw-id',
        ),
        rules: ['G'],
      },
      {
        name: '(16) деталь больше не использует usbHostName',
        files: {
          ...real,
          detail: mutate(
            mutate(real.detail, '{#if printer.usbHostName}', '{#if printer.vendor}', 'if-name'),
            '{printer.usbHostName}',
            '{printer.vendor}',
            'print-name',
          ),
        },
        rules: ['G'],
      },
      {
        name: '(17) sectionFor без hasSectionOne',
        files: picker(
          'return ctx.hasSectionOne ? SECTION_ALL : undefined;',
          'return SECTION_ALL;',
          'no-has',
        ),
        rules: ['H'],
      },
      {
        name: '(18) ПЕРЕСТАНОВКА: ветви тернарника sectionFor поменяны местами',
        files: picker(
          'return ctx.hasSectionOne ? SECTION_ALL : undefined;',
          'return ctx.hasSectionOne ? undefined : SECTION_ALL;',
          'swap-ternary',
        ),
        rules: ['H'],
      },
      {
        name: '(19) isUsbHostFieldVisible( переименована в форме',
        files: form('isUsbHostFieldVisible(', 'isUsbHostFieldVisibleX(', 'rename-call'),
        rules: ['H'],
      },
      {
        name: '(20) isUsbHostFieldVisible больше не экспортирована',
        files: picker(
          'export function isUsbHostFieldVisible(',
          'function isUsbHostFieldVisible(',
          'no-export',
        ),
        rules: ['H'],
      },
      {
        name: '(21) нет instance-блока <script> в форме (остался только module)',
        files: {
          ...real,
          form: (() => {
            const m = mutate(real.form, '<script lang="ts">', '<div lang="ts">', 'no-script');
            const k = m.lastIndexOf('</script>');
            return m.slice(0, k) + '</div>' + m.slice(k + '</script>'.length);
          })(),
        },
        rules: ['B'],
      },
      {
        // `/*` внутри строчного комментария не открывает блок: иначе вычистка стёрла бы
        // код до ближайшего `*/` и нарушение спряталось бы.
        name: '(22) `/*` в строчном комментарии не прячет нарушение',
        files: form(
          'let hostSeq = 0;',
          'let hostSeq = 0;\n  // шаблоны лежат в templates/*.html\n  void { member_of_group_id: 1 };\n  /* настоящий блочный комментарий */',
          'comment-trap',
        ),
        rules: ['F'],
      },
      {
        name: '(23) внешнее условие блока принтера требует !isEdit',
        files: form(
          '{#if typeId === PRINTER_TYPE_ID && (!isEdit || printerLoaded)}',
          '{#if typeId === PRINTER_TYPE_ID && !isEdit}',
          'outer-not-edit',
        ),
        rules: ['J'],
      },
      {
        name: '(24) ВОЗВРАТ УСЛОВИЯ ПО IP В ВЕТКУ СОЗДАНИЯ isUsbHostFieldVisible',
        files: picker2([
          ['  isEdit: boolean;\n', '  isEdit: boolean;\n  ipAddress: string;\n', 'param-ip'],
          [
            'if (!ctx.isEdit) return true;',
            "if (!ctx.isEdit) return ctx.ipAddress.trim() === '';",
            'create-ip',
          ],
        ]),
        rules: ['I'],
      },
      {
        name: '(24b) ВОЗВРАТ УСЛОВИЯ ПО loadedPrinterIp В ВЕТКУ ПРАВКИ isUsbHostFieldVisible',
        files: picker2([
          [
            '  isEdit: boolean;\n',
            '  isEdit: boolean;\n  loadedPrinterIp: string | null;\n',
            'param-lpi',
          ],
          [
            '\n  return ctx.printerLoaded;\n}',
            "\n  return ctx.printerLoaded && (ctx.loadedPrinterIp ?? '').trim() === '';\n}",
            'edit-ip',
          ],
        ]),
        rules: ['I'],
      },
      {
        name: '(24c) поле хоста в разметке снова обёрнуто условием по IP',
        files: form(
          '{#if showUsbHost}',
          "{#if showUsbHost && ipAddress.trim() === ''}",
          'host-ip-wrap',
        ),
        rules: ['E'],
      },
      {
        name: '(24d) форма снова передаёт ipAddress в isUsbHostFieldVisible',
        files: form(
          '      readonly,\n      printerLoaded,\n    }),',
          '      readonly,\n      ipAddress,\n      printerLoaded,\n    }),',
          'call-ip',
        ),
        rules: ['I'],
      },
      {
        name: '(25) предзаполнение IP убрано',
        files: form("ipAddress = p.ipAddress ?? '';", "ipAddress = '';", 'no-prefill'),
        rules: ['K'],
      },
      {
        name: '(26) ПЕРЕСТАНОВКА: присваивание IP вынесено из блока сравнения с сохранённым',
        files: form(
          "if (ipNow !== (loadedPrinterIp ?? '').trim()) {\n        patch.printer_ip_address = ipNow === '' ? null : ipNow;\n      }",
          "patch.printer_ip_address = ipNow === '' ? null : ipNow;\n      if (ipNow !== (loadedPrinterIp ?? '').trim()) {\n      }",
          'swap-ip-assign',
        ),
        rules: ['L'],
      },
      {
        name: '(26b) ПЕРЕСТАНОВКА: присваивание IP добавлено ПЕРЕД охраняющим блоком',
        files: form(
          'if (typeId === PRINTER_TYPE_ID && printerLoaded && !readonly) {',
          "patch.printer_ip_address = ipAddress.trim() === '' ? null : ipAddress.trim();\n    if (typeId === PRINTER_TYPE_ID && printerLoaded && !readonly) {",
          'ip-before-guard',
        ),
        rules: ['L'],
      },
      {
        name: '(27) из охраны убрана проверка printerLoaded',
        files: form(
          'if (typeId === PRINTER_TYPE_ID && printerLoaded && !readonly) {',
          'if (typeId === PRINTER_TYPE_ID && !readonly) {',
          'no-loaded',
        ),
        rules: ['L'],
      },
      {
        name: '(28) сравнение с сохранённым IP заменено на true',
        files: form("if (ipNow !== (loadedPrinterIp ?? '').trim()) {", 'if (true) {', 'no-compare'),
        rules: ['L'],
      },
      {
        name: '(29) присваивание IP без null-ветки',
        files: form(
          "patch.printer_ip_address = ipNow === '' ? null : ipNow;",
          'patch.printer_ip_address = ipNow;',
          'no-null',
        ),
        rules: ['L'],
      },
      {
        name: "(30) начальное значение community снова голое 'public'",
        files: form(
          "$state(target === null ? 'public' : '')",
          "$state('public')",
          'community-public',
        ),
        rules: ['M'],
      },
      {
        name: '(30b) printer_community уходит и при пустой community',
        files: form(
          "if (community.trim() !== '' && ipNow !== '') {",
          "if (ipNow !== '') {",
          'community-empty',
        ),
        rules: ['M'],
      },
      {
        name: '(30c) printer_community уходит и при пустом IP',
        files: form(
          "if (community.trim() !== '' && ipNow !== '') {",
          "if (community.trim() !== '') {",
          'community-no-ip',
        ),
        rules: ['M'],
      },
      {
        name: '(31) вернулось «create+Принтер ONLY» в комментарии',
        files: form(
          'let hostSeq = 0;',
          'let hostSeq = 0;\n  // Optional, create+Принтер ONLY (D-03)',
          'create-only',
        ),
        rules: ['N'],
      },
      {
        name: '(32) вернулась подсказка «Оставьте пустым для USB/локального принтера»',
        files: form(
          '<div class="snmp-section">',
          '<div class="snmp-section">\n      <p>Оставьте пустым для\n USB/локального принтера.</p>',
          'old-hint',
        ),
        rules: ['O'],
      },
      {
        name: '(33) блок принтера при создании строится только по IP (хост без IP теряется)',
        files: form(
          "if (typeId === PRINTER_TYPE_ID && (ipAddress.trim() !== '' || usbHost !== null)) {",
          "if (typeId === PRINTER_TYPE_ID && ipAddress.trim() !== '') {",
          'create-ip-only',
        ),
        rules: ['C'],
      },
      {
        name: '(35) строка хоста в детали снова ветка {:else if} после строки IP',
        files: detail(
          '        {/if}\n        <!-- D-15: USB-хост и IP независимы — принтер бывает и сетевым, и USB-подключённым. -->\n        {#if printer.usbHostDeviceId}',
          '        {:else if printer.usbHostDeviceId}',
          'detail-else-if',
        ),
        rules: ['P'],
      },
      {
        name: '(36) к охране ключа хоста в submitEdit приписано условие по IP',
        files: form(
          TOUCH_COND,
          "if (usbHostTouched && typeId === PRINTER_TYPE_ID && showUsbHost && ipAddress.trim() === '') {",
          'edit-host-ip-guard',
        ),
        rules: ['B2'],
      },
    ];
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  let failed = 0;
  for (const m of mutants) {
    const v = [];
    scanAll(m.files, v);
    const hit = m.rules.every((r) => v.some((line) => line.startsWith(`(${r})`)));
    console.error(
      `${TAG} [selftest] ${hit ? 'PASS' : 'FAIL'} — ${m.name} (ожидалось ${m.rules.join('+')}, получено ${v.length})`,
    );
    if (!hit) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  }

  // Зелёный контроль: запретные токены внутри КОММЕНТАРИЕВ не краснят.
  try {
    const commented = {
      ...real,
      form: mutate(
        real.form,
        'let hostSeq = 0;',
        'let hostSeq = 0;\n  // раньше был member_of_group_id и variant="select"\n  /* member_of_group_id */\n  // ipAddress, loadedPrinterIp, printer_ip_address, Оставьте пустым для USB',
        'comment-ok',
      ),
      detail: mutate(
        real.detail,
        '<span class="meta-label">Подключение</span>',
        '<span class="meta-label">Подключение</span><!-- было: устройство #{printer.usbHostDeviceId} -->',
        'comment-ok-detail',
      ),
      picker: mutate(
        real.picker,
        'export function isUsbHostFieldVisible(',
        '// раньше: ipAddress, loadedPrinterIp, hasSavedHost\nexport function isUsbHostFieldVisible(',
        'comment-ok-picker',
      ),
    };
    commented.form = mutate(
      commented.form,
      '<div class="snmp-section">',
      '<div class="snmp-section">\n      <!-- было: Оставьте пустым для USB/локального принтера -->',
      'comment-ok-html',
    );
    const v = [];
    scanAll(commented, v);
    const ok = v.length === 0;
    console.error(
      `${TAG} [selftest] ${ok ? 'PASS' : 'FAIL'} — запретные токены в комментариях не краснят (ожидалось 0, получено ${v.length})`,
    );
    if (!ok) {
      failed++;
      for (const line of v) console.error(`${TAG} [selftest]   ${line}`);
    }
  } catch (e) {
    console.error(`${TAG} [selftest] FAIL — ${e.message}`);
    return false;
  }

  if (failed > 0) {
    console.error(`${TAG} [selftest] FAIL — ${failed} проверок не совпали.`);
    return false;
  }
  console.error(
    `${TAG} [selftest] PASS — реальный исходник чист, ${mutants.length} мутантов пойманы.`,
  );
  return true;
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help') || args.includes('-h')) {
    console.error(
      `${TAG} Usage: node scripts/check-printer-usb-host-field.mjs [--src=<ui-dir>|--selftest]`,
    );
    process.exit(0);
  }
  if (args.includes('--selftest')) {
    process.exit(runSelfTest() ? 0 : 1);
  }
  const argSrc = args.find((a) => a.startsWith('--src='));
  const root = argSrc ? path.resolve(process.cwd(), argSrc.slice('--src='.length)) : UI_ROOT;

  const violations = [];
  const files = readAll(root, violations);
  if (files) scanAll(files, violations);

  if (violations.length > 0) {
    for (const v of violations) console.error(`${TAG} ${v}`);
    console.error(`${TAG} FAIL — ${violations.length} нарушений.`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

main();
