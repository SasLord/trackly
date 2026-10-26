#!/usr/bin/env node
// [check-act-delete-notices] Структурный гейт клиентской части отката акта
// (ActsPage.svelte, MovementTimeline.svelte; фаза 41.2, план 15, GAM-03,
// D-09/D-14, UI-SPEC §10.1 и §10.4).
//
// Почему он существует: svelte-check / eslint / build не видят ни ПОРЯДКА
// тостов, ни того, что строка самой группы проверяется РАНЬШЕ строки члена.
// Все правила ниже ломаются молча — приложение собирается и показывает чтото
// похожее:
//   - предупреждения откатa показаны ДО success-тоста: пользователь читает
//     «не вернулся в группу» над «Акт удалён» и считает удаление неудачным;
//   - одинаковые уведомления сворачиваются ПОСЛЕ отсечения по пяти: сводка
//     «И ещё K» считает дубли, которых стор всё равно не покажет;
//   - предупреждение не sticky (исчезнет за 5 секунд, до прочтения) или не
//     warning (красный error-тост на успешное удаление);
//   - строка самой группы получает хвост «в составе группы» вместо «перенос
//     группы»: у неё тоже есть group_id (свой), и проверка члена, поставленная
//     ПЕРВОЙ, перехватывает её;
//   - хвост «перенос группы» стал ссылкой (группа на себя не ссылается);
//   - в разметку из строк сервера попал {@html}.
//
// ВАЖНО: это СТРУКТУРНЫЙ гейт. Он не доказывает, что тосты появляются в живом
// приложении, — это проверяется живой проверкой фазы (план 41.2-16).
//
// Правила (над текстом без комментариев):
//   (A) handleDelete: первый pushToast('success' стоит РАНЬШЕ вызова
//       showRollbackNotices(result.notices), а тот — ПОСЛЕ `await acts.delete(`;
//       notifyPlaceContentChanged(result.changed_place_ids) на месте.
//   (B) showRollbackNotices: MAX_NOTICE_TOASTS = 5; сворачивание дублей
//       (`const unique`) стоит РАНЬШЕ отсечения (`unique.slice(0, MAX_NOTICE_TOASTS)`)
//       и цикл идёт по `unique`; оба тоста — kind 'warning' и `sticky: true`;
//       ссылка собирается из числового group_id: '#/groups?id=' + n.group_id.
//   (C) ни ActsPage.svelte, ни MovementTimeline.svelte не содержат {@html.
//   (D) MovementTimeline: actTail проверяет entity_type === 'group' РАНЬШЕ
//       group_id !== null; ветка source === 'act' рисует хвост ПОСЛЕ ссылки на
//       акт (handleNavigateToAct) и ДО ветки source === 'group'; в хвосте
//       «перенос группы» нет <button; в хвосте члена ссылка ведёт на
//       handleNavigateToGroup.
//
// Отсутствующий файл — нарушение, а не SKIP (гейт, пропускающий файл молча,
// нем навсегда).
//
// Self-test (`--selftest`): чистые РЕАЛЬНЫЕ файлы проходят; мутанты по реальному
// исходнику через mutate() с проверкой ровно одного вхождения якоря. Порядок
// защищён мутантами-ПЕРЕСТАНОВКАМИ (swap), а не удалением: удаление проходит
// гейт, поставленный ПОСЛЕ охраняемого.
//
// Zero-dependency: node:fs/node:path/node:url.
//
// Usage:
//   node scripts/check-act-delete-notices.mjs              # проверить репозиторий
//   node scripts/check-act-delete-notices.mjs --src=<dir>  # копия каталога ui/
//   node scripts/check-act-delete-notices.mjs --selftest   # встроенный self-test

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-act-delete-notices]';

const FILES = {
  page: 'src/features/acts/ActsPage.svelte',
  timeline: 'src/lib/components/MovementTimeline.svelte',
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
  // Тело функции начинается с `{` ПОСЛЕ закрывающей `)` параметров (и после
  // возможной аннотации возврата): ищем первую `{` за скобками параметров.
  const braceOpen = code.indexOf('{', parenOpen + params.length + 2);
  if (braceOpen === -1) return null;
  return extractBalanced(code, braceOpen, '{', '}');
}

function before(text, a, b) {
  const ia = text.indexOf(a);
  const ib = text.indexOf(b);
  return ia !== -1 && ib !== -1 && ia < ib;
}

export function scanSource(files, violations) {
  const page = stripComments(files.page);
  const timeline = stripComments(files.timeline);

  // (A) порядок тостов в handleDelete.
  const del = functionBody(page, 'handleDelete');
  if (del === null) {
    violations.push(`(A) ${FILES.page}: не найдена функция handleDelete — антивакуум порядка.`);
  } else {
    if (!del.includes('showRollbackNotices(result.notices)')) {
      violations.push(
        `(A) ${FILES.page}: handleDelete не вызывает showRollbackNotices(result.notices) — предупреждения откатa не показываются.`,
      );
    }
    if (!before(del, 'await acts.delete(', 'showRollbackNotices(')) {
      violations.push(
        `(A) ${FILES.page}: showRollbackNotices( должен стоять после await acts.delete( — до ответа сервера уведомлений нет.`,
      );
    }
    if (!before(del, "pushToast(\n        'success'", 'showRollbackNotices(')) {
      violations.push(
        `(A) ${FILES.page}: success-тост обязан стоять РАНЬШЕ showRollbackNotices( — предупреждения показываются после него (UI-SPEC §10.4).`,
      );
    }
    if (!del.includes('notifyPlaceContentChanged(result.changed_place_ids)')) {
      violations.push(
        `(A) ${FILES.page}: handleDelete не вызывает notifyPlaceContentChanged(result.changed_place_ids).`,
      );
    }
  }

  // (B) showRollbackNotices.
  const show = functionBody(page, 'showRollbackNotices');
  if (show === null) {
    violations.push(`(B) ${FILES.page}: не найдена функция showRollbackNotices — антивакуум.`);
  } else {
    if (!/const\s+MAX_NOTICE_TOASTS\s*=\s*5\s*;/.test(page)) {
      violations.push(`(B) ${FILES.page}: MAX_NOTICE_TOASTS обязан быть равен 5 (UI-SPEC §10.4).`);
    }
    const uniqueAt = show.search(/const\s+unique\b/);
    const sliceAt = show.indexOf('unique.slice(0, MAX_NOTICE_TOASTS)');
    if (uniqueAt === -1 || sliceAt === -1) {
      violations.push(
        `(B) ${FILES.page}: showRollbackNotices обязан сворачивать дубли в const unique и отсекать unique.slice(0, MAX_NOTICE_TOASTS).`,
      );
    } else if (uniqueAt > sliceAt) {
      violations.push(
        `(B) ${FILES.page}: сворачивание дублей (const unique) стоит ПОСЛЕ отсечения по пяти — сводка «И ещё K» считает дубли.`,
      );
    }
    const toasts = [...show.matchAll(/pushToast\(\s*('[a-z]+')([\s\S]*?)\);/g)];
    if (toasts.length !== 2) {
      violations.push(
        `(B) ${FILES.page}: в showRollbackNotices ожидалось ровно два pushToast (отдельный и сводный), найдено ${toasts.length}.`,
      );
    }
    for (const t of toasts) {
      if (t[1] !== "'warning'") {
        violations.push(`(B) ${FILES.page}: предупреждение откатa не 'warning': ${t[1]}.`);
      }
      if (!/sticky:\s*true/.test(t[2])) {
        violations.push(
          `(B) ${FILES.page}: предупреждение откатa без sticky: true — исчезнет раньше, чем прочитано.`,
        );
      }
    }
    if (!show.includes("'#/groups?id=' + n.group_id")) {
      violations.push(
        `(B) ${FILES.page}: ссылка тоста обязана собираться как '#/groups?id=' + n.group_id (числовой id, не строка сервера).`,
      );
    }
  }

  // (C) никакого {@html над строками сервера.
  for (const [rel, code] of [
    [FILES.page, page],
    [FILES.timeline, timeline],
  ]) {
    if (code.includes('{@html')) {
      violations.push(
        `(C) ${rel}: найден {@html — тексты уведомлений и имена групп только как текст.`,
      );
    }
  }

  // (D) MovementTimeline.
  const tail = functionBody(timeline, 'actTail');
  if (tail === null) {
    violations.push(`(D) ${FILES.timeline}: не найдена функция actTail — антивакуум.`);
  } else if (!before(tail, "entity_type === 'group'", 'group_id !== null')) {
    violations.push(
      `(D) ${FILES.timeline}: actTail обязан проверять entity_type === 'group' РАНЬШЕ group_id !== null — иначе строка самой группы (у неё тоже есть group_id) получит хвост члена.`,
    );
  }
  const actAt = timeline.indexOf("entry.source === 'act' && entry.act_id !== null");
  const groupAt = timeline.indexOf("{:else if entry.source === 'group'");
  if (actAt === -1 || groupAt === -1 || actAt > groupAt) {
    violations.push(
      `(D) ${FILES.timeline}: ветка source === 'act' обязана стоять в одной цепочке ВЫШЕ ветки source === 'group'.`,
    );
  } else {
    const branch = timeline.slice(actAt, groupAt);
    const linkAt = branch.indexOf('handleNavigateToAct(');
    const groupTailAt = branch.indexOf("actTail(entry) === 'group'");
    const memberTailAt = branch.indexOf("actTail(entry) === 'member'");
    if (linkAt === -1 || groupTailAt === -1 || memberTailAt === -1 || groupTailAt < linkAt) {
      violations.push(
        `(D) ${FILES.timeline}: ветка source === 'act' обязана рисовать хвосты actTail(entry) === 'group' и 'member' ПОСЛЕ ссылки на акт.`,
      );
    } else {
      const groupTail = branch.slice(groupTailAt, memberTailAt);
      if (!groupTail.includes('перенос группы')) {
        violations.push(
          `(D) ${FILES.timeline}: хвост строки самой группы не содержит «перенос группы».`,
        );
      }
      if (groupTail.includes('<button')) {
        violations.push(
          `(D) ${FILES.timeline}: хвост «перенос группы» стал ссылкой — группа на себя не ссылается.`,
        );
      }
      const memberTail = branch.slice(memberTailAt);
      if (
        !memberTail.includes('в составе группы') ||
        !memberTail.includes('handleNavigateToGroup(')
      ) {
        violations.push(
          `(D) ${FILES.timeline}: хвост члена группы обязан содержать «в составе группы» и ссылку handleNavigateToGroup(.`,
        );
      }
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

/** Перестановка двух уникальных фрагментов: `a` и `b` меняются местами. */
function swap(source, a, b, label) {
  const marker = '\u0000SWAP\u0000';
  const step1 = mutate(source, a, marker, `${label} (cut a)`);
  const step2 = mutate(step1, b, a, `${label} (b -> a)`);
  return mutate(step2, marker, b, `${label} (a -> b)`);
}

/** Блок хвостов (`{#if actTail(...)}…{/if}`) переносится ВЫШЕ блока со ссылкой на акт. */
function swapTailAboveLink(src) {
  const linkStart = src.indexOf('<span class="timeline-reason">\n            актом №');
  const tailStart = src.indexOf("{#if actTail(entry) === 'group'");
  const tailEnd = src.indexOf("{:else if entry.source === 'group'");
  if (
    linkStart === -1 ||
    tailStart === -1 ||
    tailEnd === -1 ||
    !(linkStart < tailStart && tailStart < tailEnd)
  ) {
    throw new Error('мутант (14): не найдены блоки ссылки на акт и хвостов в ожидаемом порядке');
  }
  return (
    src.slice(0, linkStart) +
    src.slice(tailStart, tailEnd) +
    src.slice(linkStart, tailStart) +
    src.slice(tailEnd)
  );
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
  const SUCCESS_BLOCK = `      pushToast(
        'success',
        isReturn
          ? \`Акт возврата №\${act.number} удалён. Устройства возвращены к состоянию на момент выдачи.\`
          : \`Акт №\${act.number} удалён. Устройства восстановлены.\`,
      );
`;
  const SHOW_CALL = '      showRollbackNotices(result.notices);\n';
  const DEDUPE = `    const seen = new Set<string>();
    const unique = notices.filter((n) => !seen.has(n.message) && seen.add(n.message));
`;
  const LOOP_HEAD = '    for (const n of unique.slice(0, MAX_NOTICE_TOASTS)) {\n';
  const TAIL_GROUP_FIRST = "    if (entry.entity_type === 'group') return 'group';\n";
  const TAIL_MEMBER = `    if (entry.group_id !== null && onNavigateToGroup && (entry.group_label?.trim() ?? '') !== '') {
      return 'member';
    }
`;

  let mutants;
  try {
    mutants = [
      {
        name: '(1) ПЕРЕСТАНОВКА: showRollbackNotices выше success-тоста',
        files: withFile('page', swap(raw.page, SUCCESS_BLOCK, SHOW_CALL, 'success<->show')),
        rule: 'A',
      },
      {
        name: '(2) удалён вызов showRollbackNotices(result.notices)',
        files: withFile('page', mutate(raw.page, SHOW_CALL, '', 'drop show call')),
        rule: 'A',
      },
      {
        name: '(3) ПЕРЕСТАНОВКА: сворачивание дублей ниже отсечения по пяти',
        files: withFile('page', swap(raw.page, DEDUPE, LOOP_HEAD, 'dedupe<->loop')),
        rule: 'B',
      },
      {
        name: '(4) отсечение по пяти идёт по исходному списку, а не по свёрнутому',
        files: withFile(
          'page',
          mutate(
            raw.page,
            'unique.slice(0, MAX_NOTICE_TOASTS)',
            'notices.slice(0, MAX_NOTICE_TOASTS)',
            'slice notices',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(5) MAX_NOTICE_TOASTS = 50',
        files: withFile(
          'page',
          mutate(raw.page, 'MAX_NOTICE_TOASTS = 5;', 'MAX_NOTICE_TOASTS = 50;', 'max 50'),
        ),
        rule: 'B',
      },
      {
        name: '(6) предупреждение отдельного уведомления не sticky',
        files: withFile('page', mutate(raw.page, '        sticky: true,\n', '', 'drop sticky')),
        rule: 'B',
      },
      {
        name: "(7) предупреждение отдельного уведомления — 'error'",
        files: withFile(
          'page',
          mutate(
            raw.page,
            "pushToast('warning', n.message",
            "pushToast('error', n.message",
            'error',
          ),
        ),
        rule: 'B',
      },
      {
        name: '(8) ссылка тоста собирается из строки сервера',
        files: withFile(
          'page',
          mutate(raw.page, "'#/groups?id=' + n.group_id", 'n.message', 'href from message'),
        ),
        rule: 'B',
      },
      {
        name: '(9) {@html в ActsPage',
        files: withFile(
          'page',
          mutate(
            raw.page,
            '<div class="acts-page">',
            '<div class="acts-page">{@html x}',
            'html page',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(10) {@html в MovementTimeline',
        files: withFile(
          'timeline',
          mutate(
            raw.timeline,
            '<span class="timeline-reason">перенос группы</span>',
            '<span class="timeline-reason">{@html entry.group_label}</span>',
            'html timeline',
          ),
        ),
        rule: 'C',
      },
      {
        name: '(11) ПЕРЕСТАНОВКА в actTail: проверка члена выше проверки строки группы',
        files: withFile(
          'timeline',
          swap(raw.timeline, TAIL_GROUP_FIRST, TAIL_MEMBER, 'tail group<->member'),
        ),
        rule: 'D',
      },
      {
        name: '(12) хвост «перенос группы» стал ссылкой',
        files: withFile(
          'timeline',
          mutate(
            raw.timeline,
            '<span class="timeline-reason">перенос группы</span>',
            '<span class="timeline-reason"><button type="button" class="timeline-link">перенос группы</button></span>',
            'group tail link',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(13) хвост члена группы без ссылки на группу',
        files: withFile(
          'timeline',
          mutate(
            raw.timeline,
            'onclick={() => handleNavigateToGroup(entry.group_id as number)}\n              >\n                {entry.group_label?.trim()}\n              </button>»\n            </span>\n          {/if}',
            '>\n                {entry.group_label?.trim()}\n              </button>»\n            </span>\n          {/if}',
            'member tail no link',
          ),
        ),
        rule: 'D',
      },
      {
        name: '(14) ПЕРЕСТАНОВКА: хвосты выше ссылки на акт',
        files: withFile('timeline', swapTailAboveLink(raw.timeline)),
        rule: 'D',
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
    if (hit) {
      caught++;
      console.error(`${TAG} [selftest] PASS — ${m.name} (поймано правилом ${m.rule})`);
    } else {
      failed++;
      console.error(
        `${TAG} [selftest] FAIL — ${m.name}: гейт не покраснел правилом ${m.rule} (нарушения: ${v.length ? v.join(' | ') : 'нет'})`,
      );
    }
  }

  // Отсутствующий файл — нарушение, не SKIP.
  const missing = [];
  const tmp = fs.mkdtempSync(path.join(UI_ROOT, '.selftest-missing-'));
  try {
    const got = readSources(tmp, missing);
    if (got !== null || missing.length === 0) {
      failed++;
      console.error(`${TAG} [selftest] FAIL — отсутствующий файл не дал нарушения.`);
    } else {
      caught++;
      console.error(`${TAG} [selftest] PASS — отсутствующий файл — нарушение, не SKIP`);
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
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
      `${TAG} Usage: node scripts/check-act-delete-notices.mjs [--src=<ui-dir>|--selftest]`,
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
