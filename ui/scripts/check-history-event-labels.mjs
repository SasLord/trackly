#!/usr/bin/env node
// [check-history-event-labels] Гейт паритета видов и оснований событий истории
// Rust ↔ фронт (фаза 41.10, D-23).
//
// Читает НАСТОЯЩИЕ исходники:
//   - crates/trackly-core/src/domain/history_events.rs — enum HistoryEventKind и
//     HistoryReason, токены as_str(), массивы ALL;
//   - ui/src/lib/historyEvents.ts — словари HISTORY_KIND_LABELS / HISTORY_REASON_LABELS,
//     kindText, reasonLabel (выполняется: типы снимаются typescript, Node .ts не грузит).
// Проверки:
//   [H0] ни одно множество не разобрано пустым;
//   [H1] варианты enum == варианты в ALL, длина ALL == объявленному размеру (оба enum);
//   [H2] у каждого варианта есть токен в as_str(), токены уникальны;
//   [H3] ключи словарей == токены соответствующего enum;
//   [H4] kindText по каждому токену ≠ «событие не определено», для garbage — равен;
//        reasonLabel для garbage — «причина не определена»; для каждого токена — не она;
//   [H5] (ревью 41.10 WR-01) образование безъякорной группы в ленте группы
//        (subject_type 'group', subject_label = имя самой группы) не называет группу
//        устройством; от якоря-устройства — называет устройство.
//
// Название не check-history-labels: оно занято событиями заявки (41.5).
//
// Usage:
//   node scripts/check-history-event-labels.mjs
//   node scripts/check-history-event-labels.mjs --selftest   (мутанты копий настоящих исходников)

import { Buffer } from 'node:buffer';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-history-event-labels]';
const ROOT = path.resolve(__dirname, '../..');
const TS_FILE = path.join(ROOT, 'ui/src/lib/historyEvents.ts');
const RS_FILE = path.join(ROOT, 'crates/trackly-core/src/domain/history_events.rs');
const UNKNOWN_EVENT = 'событие не определено';
const UNKNOWN_REASON = 'причина не определена';

async function loadTs(src) {
  const ts = createRequire(import.meta.url)('typescript');
  const js = ts.transpileModule(src, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: 'historyEvents.ts',
    reportDiagnostics: false,
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(js, 'utf8').toString('base64')}`);
}

function stripLineComments(s) {
  return s
    .split('\n')
    .map((l) => l.replace(/\/\/.*$/, ''))
    .join('\n');
}

/** Текст от `start` до парной закрывающей скобки (по первой `open` после start). */
function block(src, start, open = '{', close = '}') {
  const i = src.indexOf(start);
  if (i < 0) return null;
  const o = src.indexOf(open, i + start.length - 1);
  if (o < 0) return null;
  let depth = 0;
  for (let k = o; k < src.length; k++) {
    if (src[k] === open) depth++;
    else if (src[k] === close) {
      depth--;
      if (depth === 0) return src.slice(o + 1, k);
    }
  }
  return null;
}

/** Разбор одного enum: варианты, токены as_str(), ALL. Блоки ищутся внутри `impl <Name>`. */
function parseEnum(rs, name) {
  const code = stripLineComments(rs);
  const enumBody = block(code, `pub enum ${name} {`) ?? '';
  const variants = [...enumBody.matchAll(/^\s*([A-Z]\w*)\s*,?\s*$/gm)].map((m) => m[1]);
  const impl = block(code, `impl ${name} {`) ?? '';
  const asStr = block(impl, "pub fn as_str(&self) -> &'static str {") ?? '';
  const tokenOf = new Map(
    [...asStr.matchAll(/Self::(\w+)\s*=>\s*"([^"]*)"/g)].map((m) => [m[1], m[2]]),
  );
  const allDecl = impl.match(
    new RegExp(`pub const ALL:\\s*\\[${name};\\s*(\\d+)\\]\\s*=\\s*\\[([^\\]]*)\\]`),
  );
  const allSize = allDecl ? Number(allDecl[1]) : -1;
  const all = allDecl ? [...allDecl[2].matchAll(/Self::(\w+)/g)].map((m) => m[1]) : [];
  return { variants, tokenOf, all, allSize };
}

function segment(eventKind) {
  return {
    kind: 'membership',
    event_kind: eventKind,
    movement: null,
    subject_type: 'device',
    subject_id: 1,
    subject_label: 'Монитор #2',
    from_group_id: 1,
    from_group_label: 'АРМ-3',
    to_group_id: 2,
    to_group_label: 'АРМ-5',
    status_from_code: 'a',
    status_from_label: 'В работе',
    status_to_code: 'b',
    status_to_label: 'На списание',
  };
}

function entry(source) {
  return {
    key: 'e:op-1',
    created_at_utc: 1_700_000_000,
    actor_display: 'Петров П.П.',
    reason_source: source,
    reason_is_event: true,
    act_id: null,
    act_number: null,
    request_id: null,
    ref_group_id: null,
    ref_group_label: null,
    segments: [],
  };
}

function setEq(a, b) {
  const sa = new Set(a);
  const sb = new Set(b);
  return sa.size === sb.size && [...sa].every((x) => sb.has(x));
}

function checkEnum(bad, label, parsed, keys, dictName) {
  const { variants, tokenOf, all, allSize } = parsed;
  if (variants.length === 0) bad.push(`[H0] варианты enum ${label} не разобраны`);
  if (tokenOf.size === 0) bad.push(`[H0] токены as_str() ${label} не разобраны`);
  if (all.length === 0) bad.push(`[H0] ${label}::ALL не разобран`);
  if (keys.length === 0) bad.push(`[H0] ${dictName} пуст`);
  if (!setEq(variants, all))
    bad.push(`[H1] ${label}: варианты ${JSON.stringify(variants)} ≠ ALL ${JSON.stringify(all)}`);
  if (allSize !== all.length)
    bad.push(`[H1] ${label}: объявленный размер ALL ${allSize} ≠ числу элементов ${all.length}`);
  const tokens = [];
  for (const v of variants) {
    const t = tokenOf.get(v);
    if (t === undefined) bad.push(`[H2] ${label}: у варианта ${v} нет токена в as_str()`);
    else tokens.push(t);
  }
  if (new Set(tokens).size !== tokens.length)
    bad.push(`[H2] ${label}: токены as_str() не уникальны: ${JSON.stringify(tokens)}`);
  if (!setEq(keys, tokens))
    bad.push(
      `[H3] ключи ${dictName} ${JSON.stringify([...keys].sort())} ≠ токены Rust ${JSON.stringify([...tokens].sort())}`,
    );
  return tokens;
}

function runChecks({ rs, mod }) {
  const bad = [];
  const kindKeys = Object.keys(mod.HISTORY_KIND_LABELS ?? {});
  const reasonKeys = Object.keys(mod.HISTORY_REASON_LABELS ?? {});
  const kindTokens = checkEnum(
    bad,
    'HistoryEventKind',
    parseEnum(rs, 'HistoryEventKind'),
    kindKeys,
    'HISTORY_KIND_LABELS',
  );
  const reasonTokens = checkEnum(
    bad,
    'HistoryReason',
    parseEnum(rs, 'HistoryReason'),
    reasonKeys,
    'HISTORY_REASON_LABELS',
  );

  const run = (fn) => {
    try {
      return fn();
    } catch (e) {
      return `исключение: ${e}`;
    }
  };
  const bodyOk = (got, unknown) =>
    typeof got === 'string' && got.length > 0 && got !== unknown && !got.startsWith('исключение');

  for (const variant of ['device', 'group']) {
    const ctx = { variant, currentGroupId: 2 };
    for (const t of kindTokens) {
      const got = run(() => mod.kindText(segment(t), ctx));
      if (!bodyOk(got, UNKNOWN_EVENT))
        bad.push(`[H4] вид «${t}» (${variant}) без подписи: ${JSON.stringify(got)}`);
    }
    const g = run(() => mod.kindText(segment('garbage'), ctx));
    if (g !== UNKNOWN_EVENT)
      bad.push(`[H4] нераспознанный вид (${variant}) должен давать «${UNKNOWN_EVENT}»`);
  }
  for (const t of reasonTokens) {
    const got = run(() => mod.reasonLabel(entry(t)));
    if (!bodyOk(got, UNKNOWN_REASON))
      bad.push(`[H4] основание «${t}» без подписи: ${JSON.stringify(got)}`);
  }
  if (run(() => mod.reasonLabel(entry('garbage'))) !== UNKNOWN_REASON)
    bad.push(`[H4] нераспознанное основание должно давать «${UNKNOWN_REASON}»`);
  if (mod.UNKNOWN_HISTORY_EVENT !== UNKNOWN_EVENT)
    bad.push('[H4] UNKNOWN_HISTORY_EVENT ≠ «событие не определено»');

  // [H5] субъект образования — сама группа (безъякорная): не «от устройства «<группа>»».
  const groupCtx = { variant: 'group', currentGroupId: 2 };
  const selfFormed = {
    ...segment('group_formed'),
    subject_type: 'group',
    subject_id: 2,
    subject_label: 'АРМ-5',
  };
  const selfText = run(() => mod.kindText(selfFormed, groupCtx));
  if (typeof selfText !== 'string' || /устройств/.test(selfText) || selfText.includes('АРМ-5'))
    bad.push(
      `[H5] образование безъякорной группы названо устройством: ${JSON.stringify(selfText)}`,
    );
  const anchorText = run(() => mod.kindText(segment('group_formed'), groupCtx));
  if (typeof anchorText !== 'string' || !anchorText.includes('Монитор #2'))
    bad.push(`[H5] образование от якоря не называет устройство: ${JSON.stringify(anchorText)}`);
  return bad;
}

async function check(rs, tsSrc) {
  return runChecks({ rs, mod: await loadTs(tsSrc) });
}

async function main() {
  const rs = fs.readFileSync(RS_FILE, 'utf8');
  const tsSrc = fs.readFileSync(TS_FILE, 'utf8');

  if (process.argv.includes('--selftest')) {
    const base = await check(rs, tsSrc);
    if (base.length > 0) {
      console.error(
        `${TAG} SELFTEST FAIL — настоящие исходники не зелёные:\n  ${base.join('\n  ')}`,
      );
      process.exit(1);
    }
    // [имя, файл ('ts' | 'rs'), якорь, замена, ожидаемый код, необязательный второй якорь]
    const mutants = [
      [
        '(а) из HISTORY_KIND_LABELS удалён ключ group_left',
        'ts',
        '  group_left: (seg, ctx) =>',
        '  group_left_gone: (seg, ctx) =>',
        'H3',
      ],
      [
        '(б) из HISTORY_REASON_LABELS удалён ключ teardown',
        'ts',
        '  teardown: (e) =>',
        '  teardown_gone: (e) =>',
        'H3',
      ],
      [
        '(в) в Rust убран токен status_changed из as_str()',
        'rs',
        '            Self::StatusChanged => "status_changed",\n',
        '',
        'H2',
      ],
      [
        '(г) из HistoryReason::ALL убран ActDelete',
        'rs',
        '        Self::ActDelete,\n    ];',
        '    ];',
        'H1',
      ],
      [
        '(д) в Rust добавлен вид Survey с токеном survey',
        'rs',
        '            Self::StatusChanged => "status_changed",\n',
        '            Self::StatusChanged => "status_changed",\n            Self::Survey => "survey",\n',
        'H3',
        ['    StatusChanged,\n}', '    StatusChanged,\n    Survey,\n}'],
      ],
      [
        '(е) group_formed возвращает «событие не определено»',
        'ts',
        '  group_formed: (seg, ctx) => {',
        "  group_formed: () => 'событие не определено',\n  group_formed_old: (seg, ctx) => {",
        'H4',
      ],
      [
        '(з) group_formed в ленте группы не смотрит на subject_type (WR-01)',
        'ts',
        "      return seg.subject_type === 'device' && (seg.subject_label?.trim() ?? '').length > 0",
        "      return (seg.subject_label?.trim() ?? '').length > 0",
        'H5',
      ],
      [
        '(ж) нераспознанное основание больше не «причина не определена»',
        'ts',
        "export const UNKNOWN_HISTORY_REASON = 'причина не определена';",
        "export const UNKNOWN_HISTORY_REASON = 'нет причины';",
        'H4',
      ],
    ];
    const replaceOnce = (src, from, to, name) => {
      const n = src.split(from).length - 1;
      if (n !== 1) {
        console.error(`${TAG} SELFTEST FAIL — якорь мутанта «${name}» найден ${n} раз`);
        process.exit(1);
      }
      return src.replace(from, to);
    };
    let missed = 0;
    for (const [name, file, from, to, code, extra] of mutants) {
      let mRs = rs;
      let mTs = tsSrc;
      if (file === 'rs') {
        mRs = replaceOnce(mRs, from, to, name);
        if (extra) mRs = replaceOnce(mRs, extra[0], extra[1], name);
      } else {
        mTs = replaceOnce(mTs, from, to, name);
      }
      const bad = await check(mRs, mTs);
      const ok = bad.some((b) => b.startsWith(`[${code}]`));
      console.log(`${TAG} мутант «${name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${code}]`);
      if (!ok) missed++;
    }
    if (missed > 0) process.exit(1);
    console.log(`${TAG} selftest OK — ${mutants.length} мутантов красные`);
    return;
  }

  const bad = await check(rs, tsSrc);
  if (bad.length > 0) {
    console.error(`${TAG} FAIL:\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}

main();
