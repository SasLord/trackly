#!/usr/bin/env node
// [check-repair-card-device] Гейт: блок «Информация» карточки заявки
// (src/features/requests/RequestDetail.svelte) обязан иметь ветку для
// request_type = 'repair', показывающую устройство (UAT 41.4, пункт 1).
//
// Почему: без отдельной ветки ремонтная заявка падает в общую ветку свободной
// заявки (категория и описание) и устройство в карточке не видно. svelte-check
// и build этого не видят.
//
// D-38 (раунд 3): RepairDeviceStatus.svelte не обращается к справочнику статусов
// (ensureLoaded, devices.statuses, device_statuses_list) ни для какой роли — сотруднику
// он закрыт; RequestDetail передаёт ему deviceStatusName из DTO состояния ремонта.
//
// Проверяется: ветка `{:else if request.requestType === 'repair'}` есть внутри
// блока «Информация», стоит ДО общего `{:else}` и содержит вывод deviceName и
// deviceInventoryNo. Гейт структурный, рантайм не доказывает.
//
// Usage:
//   node scripts/check-repair-card-device.mjs
//   node scripts/check-repair-card-device.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripComments } from './_strip.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TARGET = path.resolve(__dirname, '../src/features/requests/RequestDetail.svelte');
const STATUS_TARGET = path.resolve(__dirname, '../src/features/requests/RepairDeviceStatus.svelte');
const TAG = '[check-repair-card-device]';

const BRANCH = /\{:else if request\.requestType === 'repair'\}/;

/** Нарушения: список строк. */
export function check(raw, statusRaw) {
  const src = stripComments(raw);
  const bad = [];
  // D-38: бейдж статуса не ходит в справочник; подпись приходит prop'ом из DTO.
  const st = stripComments(statusRaw ?? '');
  if (st === '') bad.push('RepairDeviceStatus.svelte не прочитан');
  else {
    for (const needle of [
      'ensureLoaded',
      'devices.statuses',
      'device_statuses_list',
      'statusByCode',
    ])
      if (st.includes(needle)) bad.push(`RepairDeviceStatus обращается к справочнику: ${needle}`);
  }
  // Алиас импорта (ensureLoaded as ...) обходит поиск по имени: из стора разрешён один statusTone.
  for (const m of st.matchAll(
    /import\s*\{([^}]*)\}\s*from\s*'\$lib\/stores\/deviceStatuses\.svelte'/g,
  )) {
    const names = m[1]
      .split(',')
      .map((x) => x.trim())
      .filter(Boolean);
    if (names.some((n) => n !== 'statusTone'))
      bad.push(
        `RepairDeviceStatus импортирует из стора статусов не только statusTone: ${names.join(', ')}`,
      );
  }
  if (!/<RepairDeviceStatus\b[^>]*\bname=\{[^}]*deviceStatusName/.test(src))
    bad.push('RequestDetail не передаёт deviceStatusName в RepairDeviceStatus');
  const start = src.indexOf('heading="Информация"');
  if (start < 0) return ['блок «Информация» не найден'];
  const end = src.indexOf('</DetailSection>', start);
  const block = src.slice(start, end < 0 ? undefined : end);
  const m = BRANCH.exec(block);
  if (!m) return ["нет ветки {:else if request.requestType === 'repair'} в блоке «Информация»"];
  const after = block.slice(m.index + m[0].length);
  // Конец ветки: следующее {:else} верхнего уровня (общая ветка) или {/if} блока.
  let depth = 0;
  let branchEnd = after.length;
  const re = /\{#if\b|\{\/if\}|\{:else\}|\{:else if\b/g;
  let t;
  while ((t = re.exec(after))) {
    if (t[0] === '{#if') depth++;
    else if (t[0] === '{/if}') {
      if (depth === 0) {
        branchEnd = t.index;
        break;
      }
      depth--;
    } else if (depth === 0) {
      branchEnd = t.index;
      break;
    }
  }
  const body = after.slice(0, branchEnd);
  if (!/value=\{repairDeviceLine\}/.test(body))
    bad.push('ветка repair не выводит repairDeviceLine');
  const def = /const repairDeviceLine = \$derived\.by\(\(\) => \{([\s\S]*?)\n {2}\}\);/.exec(src);
  if (!def) bad.push('нет определения repairDeviceLine');
  else {
    if (!/deviceName/.test(def[1])) bad.push('repairDeviceLine не использует deviceName');
    if (!/deviceInventoryNo/.test(def[1]))
      bad.push('repairDeviceLine не использует deviceInventoryNo');
  }
  // Ветка должна стоять до общего {:else} верхнего уровня.
  const before = block.slice(0, m.index);
  let d = 0;
  const re2 = /\{#if\b|\{\/if\}|\{:else\}/g;
  let u;
  let generalBefore = false;
  while ((u = re2.exec(before))) {
    if (u[0] === '{#if') d++;
    else if (u[0] === '{/if}') d--;
    else if (d === 1) generalBefore = true;
  }
  if (generalBefore) bad.push('ветка repair стоит после общего {:else}: недостижима');
  return bad;
}

function selftest() {
  const real = fs.readFileSync(TARGET, 'utf8');
  const realSt = fs.readFileSync(STATUS_TARGET, 'utf8');
  const count = (s, needle) => s.split(needle).length - 1;
  const mutate = (s, find, repl) => {
    if (count(s, find) !== 1) {
      console.error(`${TAG} FAIL: selftest: якорь «${find}» не уникален (${count(s, find)})`);
      process.exit(1);
    }
    return s.replace(find, repl);
  };
  // Реальный файл проходит.
  const results = [['real', check(real, realSt).length === 0]];
  // D-38: возврат обращения к справочнику в бейдже и потеря передачи подписи.
  results.push([
    'status-ensureLoaded',
    check(real, realSt.replace('const label =', 'void ensureLoaded();\n  const label =')).length >
      0,
  ]);
  results.push([
    'status-alias-import',
    check(
      real,
      realSt.replace('import { statusTone }', 'import { ensureLoaded as ensureX, statusTone }'),
    ).length > 0,
  ]);
  results.push([
    'status-statuses',
    check(real, realSt.replace('const label =', 'void devices.statuses;\n  const label =')).length >
      0,
  ]);
  results.push([
    'no-name-passed',
    check(mutate(real, 'name={curState?.deviceStatusName ?? null}', ''), realSt).length > 0,
  ]);
  // Мутант 1: ветка удалена (переименована в несуществующую).
  results.push([
    'branch-removed',
    check(
      mutate(
        real,
        "{:else if request.requestType === 'repair'}",
        "{:else if request.requestType === 'x'}",
      ),
      realSt,
    ).length > 0,
  ]);
  // Мутант 2: вывод deviceName убран из определения строки устройства.
  results.push([
    'no-deviceName',
    check(
      mutate(real, 'curState?.deviceName ?? request?.printerName', 'request?.printerName'),
      realSt,
    ).length > 0,
  ]);
  // Мутант 2b: ветка не выводит строку устройства.
  results.push([
    'branch-no-line',
    check(mutate(real, '<DetailField label="Устройство" value={repairDeviceLine} />', ''), realSt)
      .length > 0,
  ]);
  // Мутант 3 (синтетика): ветка после общего else.
  const DEF = `const repairDeviceLine = $derived.by(() => {\n deviceName deviceInventoryNo\n  });`;
  const synth = `${DEF}<DetailSection heading="Информация">{#if a}x{:else}общая{:else if request.requestType === 'repair'}value={repairDeviceLine}{/if}</DetailSection>`;
  results.push(['after-else', check(synth, realSt).length > 0]);
  const NAME_OK = '<RepairDeviceStatus code={c} name={s.deviceStatusName} />';
  const synthOk = `${DEF}<DetailSection heading="Информация">{#if a}x{:else if request.requestType === 'repair'}value={repairDeviceLine}{:else}общая{/if}</DetailSection>`;
  results.push(['synth-ok', check(synthOk + NAME_OK, realSt).length === 0]);
  const failed = results.filter(([, ok]) => !ok).map(([n]) => n);
  if (failed.length > 0) {
    console.error(`${TAG} FAIL: selftest не отличает плохие фикстуры: ${failed.join(', ')}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest OK`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const bad = check(fs.readFileSync(TARGET, 'utf8'), fs.readFileSync(STATUS_TARGET, 'utf8'));
  if (bad.length > 0) {
    for (const b of bad) console.error(`${TAG} FAIL: ${b}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}
