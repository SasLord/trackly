#!/usr/bin/env node
// [check-chat-dates] Гейт ленты чата (план 41.5-12): время и подписи дней — в ЛОКАЛЬНОМ
// времени (не UTC), группировка ≤ 5 минут, таблица «код события → тон» из UI-SPEC §9.2.
// Даты строятся локальным конструктором Date, поэтому результат не зависит от пояса.
//
// Usage:
//   node scripts/check-chat-dates.mjs
//   node scripts/check-chat-dates.mjs --selftest   (мутант: UTC вместо локального)

import { Buffer } from 'node:buffer';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-chat-dates]';
const CHAT = path.resolve(__dirname, '../src/features/requests/chat');

const sec = (y, mo, d, h = 0, mi = 0) => Math.floor(new Date(y, mo - 1, d, h, mi).getTime() / 1000);

const TONES = {
  create: 'info',
  accept: 'info',
  repair_accept: 'info',
  repair_join: 'info',
  repair_merge: 'info',
  repair_merged_into: 'neutral',
  repair_move: 'warning',
  repair_send: 'warning',
  repair_receive: 'success',
  repair_assign_device: 'neutral',
  complete: 'success',
  repair_complete: 'success',
  reject: 'danger',
  cancel: 'danger',
  repair_cancel: 'danger',
  'custom:repair_send': 'warning',
  something_unknown: 'neutral',
};

export function runChecks(dates, pres) {
  const failed = [];
  const eq = (name, got, want) => {
    if (got !== want) failed.push(`${name}: ${JSON.stringify(got)} != ${JSON.stringify(want)}`);
  };
  const now = sec(2026, 11, 18, 15, 0);
  eq('timeHHmm', dates.timeHHmm(sec(2026, 11, 18, 9, 5)), '09:05');
  eq('timeHHmm late', dates.timeHHmm(sec(2026, 11, 18, 23, 59)), '23:59');
  eq('dayKey', dates.dayKey(sec(2026, 1, 3, 0, 30)), '2026-01-03');
  eq('dayLabel today', dates.dayLabel(sec(2026, 11, 18, 0, 1), now), 'Сегодня');
  eq('dayLabel yesterday', dates.dayLabel(sec(2026, 11, 17, 23, 59), now), 'Вчера');
  eq('dayLabel same year', dates.dayLabel(sec(2026, 11, 16, 12), now), '16 ноября');
  eq('dayLabel other year', dates.dayLabel(sec(2025, 3, 1, 12), now), '1 марта 2025');
  eq('sameGroup 5m', dates.sameGroup(1000, 1300), true);
  eq('sameGroup >5m', dates.sameGroup(1000, 1301), false);
  for (const [code, tone] of Object.entries(TONES)) {
    eq(`tone ${code}`, pres.presentationFor(code).tone, tone);
  }
  eq('resolution complete', pres.presentationFor('complete').resolution, 'complete');
  eq('resolution reject', pres.presentationFor('reject').resolution, 'reject');
  eq('resolution none', pres.presentationFor('create').resolution, null);
  return failed;
}

// Node 20 в CI не импортирует .ts напрямую (ERR_UNKNOWN_FILE_EXTENSION): снимаем типы
// через typescript (прямой devDependency ui/); оба модуля без import.
async function loadTs(file) {
  const ts = createRequire(import.meta.url)('typescript');
  const js = ts.transpileModule(fs.readFileSync(path.join(CHAT, file), 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: file,
    reportDiagnostics: false,
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

async function main() {
  const dates = await loadTs('chatDates.ts');
  const pres = await loadTs('eventPresentation.ts');
  if (process.argv.includes('--selftest')) {
    // Мутант: время по UTC. Красный в любом поясе, где смещение не нулевое; чтобы быть
    // независимым от пояса, мутант ещё и меняет таблицу тонов.
    const mutDates = {
      ...dates,
      timeHHmm: (s) => {
        const d = new Date(s * 1000);
        return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`;
      },
    };
    const mutPres = {
      presentationFor: (a) => ({ ...pres.presentationFor(a), tone: 'neutral' }),
    };
    if (
      runChecks(dates, mutPres).length === 0 ||
      (runChecks(mutDates, pres).length === 0 && new Date().getTimezoneOffset() !== 0)
    ) {
      console.error(`${TAG} FAIL: selftest не ловит мутантов`);
      process.exit(1);
    }
    console.log(`${TAG} selftest OK`);
    return;
  }
  const failed = runChecks(dates, pres);
  if (failed.length > 0) {
    console.error(`${TAG} FAIL:\n  ${failed.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}

main();
