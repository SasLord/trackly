#!/usr/bin/env node
// [check-history-labels] Поведенческий гейт общего словаря подписей событий заявки
// (фаза 41.5, план 10 т.3; Nyquist 41.5 G6; MSG-02, UI-SPEC §9.1, REP-07).
//
// Выполняет НАСТОЯЩИЙ historyLabels.ts (типы снимаются typescript, как в check-chat-linkify:
// Node 20 в CI не импортирует .ts напрямую) и проверяет:
//   [H1] «История» и «Чат» — один словарь: chatEventText = entryText с заменой «; ФИО» на « · ФИО»;
//   [H2] сырые коды (create, custom:*, repair_*) пользователю не показываются (REP-07);
//   [H3] пример UI-SPEC §9.1: «Устройство передано в сервис «Сервис-центр» · Петров П.П.»;
//   [H4] mergedIntoId отдаёт relatedRequestId только для repair_merged_into (и custom:-варианта);
//   [H5] пустое/пробельное примечание хвоста не даёт, непустое — обрезается;
//   [H6] завершение ремонта (custom:repair_complete) в «Истории» = «Выполнена» с итогом работ.
//
// Usage:
//   node scripts/check-history-labels.mjs
//   node scripts/check-history-labels.mjs --selftest   (мутанты настоящего исходника)

import { Buffer } from 'node:buffer';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-history-labels]';
const TARGET = path.resolve(__dirname, '../src/features/requests/historyLabels.ts');

const ACTOR = 'Петров П.П.';
const CODES = [
  'create',
  'accept',
  'complete',
  'reject',
  'cancel',
  'custom:create',
  'custom:accept',
  'custom:complete',
  'custom:reject',
  'custom:cancel',
  'repair_accept',
  'repair_move',
  'repair_send',
  'repair_receive',
  'repair_cancel',
  'repair_join',
  'repair_merge',
  'repair_merged_into',
  'repair_assign_device',
  'repair_complete',
  'custom:repair_send',
  'custom:repair_complete',
  'repair_something_new',
];

async function load(src) {
  const ts = createRequire(import.meta.url)('typescript');
  const js = ts.transpileModule(src, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: 'historyLabels.ts',
    reportDiagnostics: false,
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

function runChecks(m) {
  const bad = [];
  const eq = (code, name, got, want) => {
    if (got !== want)
      bad.push(`[${code}] ${name}: ${JSON.stringify(got)} ≠ ${JSON.stringify(want)}`);
  };
  for (const action of CODES) {
    for (const notes of [null, 'Сервис-центр']) {
      const e = { action, notes, actorName: ACTOR };
      const hist = m.entryText(e);
      const chat = m.chatEventText(e);
      // H1: одинаковый текст, отличается только разделитель перед ФИО.
      if (!hist.includes(`; ${ACTOR}`))
        bad.push(`[H1] ${action}: в «Истории» нет «; ${ACTOR}»: ${hist}`);
      eq('H1', `${action}/${notes}`, chat, hist.replace(`; ${ACTOR}`, ` · ${ACTOR}`));
      // Без ФИО разделителя нет вовсе.
      const anon = m.chatEventText({ action, notes });
      if (anon.includes(' · ') || anon.includes('undefined') || anon.includes('null'))
        bad.push(`[H1] ${action}: без ФИО лишний разделитель или пустое значение: ${anon}`);
      // H2: сырые коды не видны.
      const raw = action.replace(/^custom:/, '');
      for (const s of [hist, chat]) {
        if (s.includes('custom:') || s.includes(raw) || /\brepair_/.test(s))
          bad.push(`[H2] ${action}: сырой код в подписи: ${s}`);
      }
    }
  }
  // H3: пример из UI-SPEC §9.1.
  eq(
    'H3',
    'repair_send',
    m.chatEventText({ action: 'repair_send', notes: 'Сервис-центр', actorName: ACTOR }),
    `Устройство передано в сервис «Сервис-центр» · ${ACTOR}`,
  );
  // H4: mergedIntoId.
  eq('H4', 'merged_into', m.mergedIntoId({ action: 'repair_merged_into', relatedRequestId: 7 }), 7);
  eq(
    'H4',
    'custom:merged_into',
    m.mergedIntoId({ action: 'custom:repair_merged_into', relatedRequestId: 8 }),
    8,
  );
  eq('H4', 'merged_into без id', m.mergedIntoId({ action: 'repair_merged_into' }), null);
  for (const action of ['repair_merge', 'create', 'repair_join']) {
    eq('H4', action, m.mergedIntoId({ action, relatedRequestId: 9 }), null);
  }
  // H5: примечание.
  eq(
    'H5',
    'пробелы',
    m.entryText({ action: 'complete', notes: '   ', actorName: ACTOR }),
    `Выполнена; ${ACTOR}`,
  );
  eq(
    'H5',
    'обрезка',
    m.entryText({ action: 'complete', notes: '  заменён ролик  ', actorName: ACTOR }),
    `Выполнена; ${ACTOR}; заменён ролик`,
  );
  // H6: сервер пишет custom:repair_complete (repair_service.rs) — та же «Выполнена», что в чате.
  for (const action of ['custom:repair_complete', 'repair_complete']) {
    eq(
      'H6',
      action,
      m.entryText({ action, notes: 'заменён блок питания', actorName: ACTOR }),
      `Выполнена; ${ACTOR}; заменён блок питания`,
    );
  }
  return bad;
}

async function main() {
  const src = fs.readFileSync(TARGET, 'utf8');
  if (process.argv.includes('--selftest')) {
    if (runChecks(await load(src)).length > 0) {
      console.error(`${TAG} SELFTEST FAIL — настоящий модуль не зелёный`);
      process.exit(1);
    }
    const mutants = [
      [
        'разделитель чата « · » заменён на «; »',
        "`${p.text}${p.who ? ` · ${p.who}` : ''}${p.suffix}`",
        "`${p.text}${p.who ? `; ${p.who}` : ''}${p.suffix}`",
        'H1',
      ],
      [
        'перестановка: suffix раньше ФИО в entryText',
        "`${p.text}${p.who ? `; ${p.who}` : ''}${p.suffix}`",
        "`${p.text}${p.suffix}${p.who ? `; ${p.who}` : ''}`",
        'H1',
      ],
      [
        'инверсия условия mergedIntoId',
        "!== 'repair_merged_into') return null",
        "=== 'repair_merged_into') return null",
        'H4',
      ],
      [
        'неизвестный repair_* показывает сырой код',
        "return { text: 'Изменение заявки', who, suffix: '' };",
        "return { text: code, who, suffix: '' };",
        'H2',
      ],
      [
        'custom: не снимается в entryParts',
        "const code = e.action.replace(/^custom:/, '');",
        'const code = e.action;',
        'H2',
      ],
      [
        'подпись завершения ремонта удалена',
        "    repair_complete: 'Выполнена',\n    'custom:repair_complete': 'Выполнена',\n",
        '',
        'H6',
      ],
      [
        'примечание не обрезается',
        'const note = e.notes?.trim() ? e.notes.trim() : null;',
        'const note = e.notes ? e.notes : null;',
        'H5',
      ],
    ];
    let missed = 0;
    for (const [name, from, to, code] of mutants) {
      const n = src.split(from).length - 1;
      if (n !== 1) {
        console.error(`${TAG} SELFTEST FAIL — якорь мутанта «${name}» найден ${n} раз`);
        process.exit(1);
      }
      const bad = runChecks(await load(src.replace(from, to)));
      const ok = bad.some((b) => b.startsWith(`[${code}]`));
      console.log(`${TAG} мутант «${name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${code}]`);
      if (!ok) missed++;
    }
    if (missed > 0) process.exit(1);
    console.log(`${TAG} selftest OK — ${mutants.length} мутантов красные`);
    return;
  }
  const bad = runChecks(await load(src));
  if (bad.length > 0) {
    console.error(`${TAG} FAIL:\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}

main();
