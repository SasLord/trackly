#!/usr/bin/env node
// [check-movement-source-labels] Гейт паритета источников перемещения Rust ↔ фронт
// (quick 261121-hed, INT-09).
//
// Читает НАСТОЯЩИЕ исходники:
//   - crates/trackly-core/src/domain/place_movements.rs — варианты `pub enum MovementSource`,
//     токены `as_str()`, массив `MovementSource::ALL`;
//   - crates/trackly-app/src/services/report_service.rs — подписи-литералы отчёта
//     (`Some(MovementSource::X) => "…"`);
//   - ui/src/lib/movementReason.ts — словарь MOVEMENT_SOURCE_LABELS + reasonText (выполняется:
//     типы снимаются typescript, как в check-history-labels — Node в CI не импортирует .ts).
// Проверки:
//   [M0] ни одно множество не разобрано пустым (гейт не зеленеет на нулевом измерении);
//   [M1] варианты enum == варианты в ALL, длина ALL == объявленному размеру;
//   [M2] у каждого варианта есть токен в as_str(), токены уникальны;
//   [M3] ключи MOVEMENT_SOURCE_LABELS == токены as_str();
//   [M4] reasonText для каждого токена ≠ «причина не определена», для «garbage» — равен;
//   [M5] где отчёт подписывает источник литералом, таймлайн даёт ту же строку.
//
// Usage:
//   node scripts/check-movement-source-labels.mjs
//   node scripts/check-movement-source-labels.mjs --selftest   (мутанты копий настоящих исходников)

import { Buffer } from 'node:buffer';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TAG = '[check-movement-source-labels]';
const ROOT = path.resolve(__dirname, '../..');
const TS_FILE = path.join(ROOT, 'ui/src/lib/movementReason.ts');
const RS_FILE = path.join(ROOT, 'crates/trackly-core/src/domain/place_movements.rs');
const REPORT_FILE = path.join(ROOT, 'crates/trackly-app/src/services/report_service.rs');
const UNKNOWN = 'причина не определена';

async function loadTs(src) {
  const ts = createRequire(import.meta.url)('typescript');
  const js = ts.transpileModule(src, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: 'movementReason.ts',
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

/** Текст от `start` до парной закрывающей скобки (по первой `{` или `[` после start). */
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

function parseRust(rs) {
  const code = stripLineComments(rs);
  const enumBody = block(code, 'pub enum MovementSource {') ?? '';
  const variants = [...enumBody.matchAll(/^\s*([A-Z]\w*)\s*,?\s*$/gm)].map((m) => m[1]);
  const asStr = block(code, "pub fn as_str(&self) -> &'static str {") ?? '';
  // Первый as_str в файле — у MovementSource (MovementEntityKind объявлен ниже).
  const tokenOf = new Map(
    [...asStr.matchAll(/Self::(\w+)\s*=>\s*"([^"]*)"/g)].map((m) => [m[1], m[2]]),
  );
  const allDecl = code.match(/pub const ALL:\s*\[MovementSource;\s*(\d+)\]\s*=\s*\[([^\]]*)\]/);
  const allSize = allDecl ? Number(allDecl[1]) : -1;
  const all = allDecl ? [...allDecl[2].matchAll(/Self::(\w+)/g)].map((m) => m[1]) : [];
  return { variants, tokenOf, all, allSize };
}

function parseReportLiterals(report) {
  return new Map(
    [...report.matchAll(/Some\(MovementSource::(\w+)\)\s*=>\s*"([^"]*)"\.to_string\(\)/g)].map(
      (m) => [m[1], m[2]],
    ),
  );
}

function entry(source) {
  return {
    id: 1,
    entity_type: 'device',
    entity_id: 1,
    from_place_id: 1,
    from_place_path: 'Склад А',
    from_place_path_short: 'Склад А',
    to_place_id: 2,
    to_place_path: 'Кабинет 12',
    to_place_path_short: 'Кабинет 12',
    actor_display: 'Петров П.П.',
    source,
    note: null,
    act_id: null,
    act_number: null,
    batch_id: null,
    group_id: null,
    group_label: null,
    request_id: null,
    created_at_utc: 1_700_000_000,
  };
}

function setEq(a, b) {
  const sa = new Set(a);
  const sb = new Set(b);
  return sa.size === sb.size && [...sa].every((x) => sb.has(x));
}

function runChecks({ rs, report, mod }) {
  const bad = [];
  const { variants, tokenOf, all, allSize } = parseRust(rs);
  const labels = mod.MOVEMENT_SOURCE_LABELS ?? {};
  const keys = Object.keys(labels);
  const literals = parseReportLiterals(report);

  if (variants.length === 0) bad.push('[M0] варианты enum MovementSource не разобраны');
  if (tokenOf.size === 0) bad.push('[M0] токены as_str() не разобраны');
  if (all.length === 0) bad.push('[M0] MovementSource::ALL не разобран');
  if (keys.length === 0) bad.push('[M0] MOVEMENT_SOURCE_LABELS пуст');
  if (literals.size === 0) bad.push('[M0] подписи отчёта (report_service.rs) не разобраны');

  if (!setEq(variants, all))
    bad.push(`[M1] варианты enum ${JSON.stringify(variants)} ≠ ALL ${JSON.stringify(all)}`);
  if (allSize !== all.length)
    bad.push(`[M1] объявленный размер ALL ${allSize} ≠ числу элементов ${all.length}`);

  const tokens = [];
  for (const v of variants) {
    const t = tokenOf.get(v);
    if (t === undefined) bad.push(`[M2] у варианта ${v} нет токена в as_str()`);
    else tokens.push(t);
  }
  if (new Set(tokens).size !== tokens.length)
    bad.push(`[M2] токены as_str() не уникальны: ${JSON.stringify(tokens)}`);

  if (!setEq(keys, tokens))
    bad.push(
      `[M3] ключи MOVEMENT_SOURCE_LABELS ${JSON.stringify(keys.sort())} ≠ токены Rust ${JSON.stringify([...tokens].sort())}`,
    );

  for (const t of tokens) {
    let got;
    try {
      got = mod.reasonText(entry(t));
    } catch (e) {
      got = `исключение: ${e}`;
    }
    if (
      got === UNKNOWN ||
      typeof got !== 'string' ||
      got.length === 0 ||
      got.startsWith('исключение')
    )
      bad.push(`[M4] источник «${t}» без подписи в таймлайне: ${JSON.stringify(got)}`);
  }
  if (mod.reasonText?.(entry('garbage')) !== UNKNOWN)
    bad.push('[M4] нераспознанный токен должен давать «причина не определена»');

  for (const [variant, lit] of literals) {
    const t = tokenOf.get(variant);
    if (t === undefined) continue; // покрыто M2
    const got = mod.reasonText?.(entry(t));
    if (got !== lit)
      bad.push(`[M5] «${t}»: таймлайн ${JSON.stringify(got)} ≠ отчёт ${JSON.stringify(lit)}`);
  }
  return bad;
}

async function check(rs, report, tsSrc) {
  return runChecks({ rs, report, mod: await loadTs(tsSrc) });
}

async function main() {
  const rs = fs.readFileSync(RS_FILE, 'utf8');
  const report = fs.readFileSync(REPORT_FILE, 'utf8');
  const tsSrc = fs.readFileSync(TS_FILE, 'utf8');

  if (process.argv.includes('--selftest')) {
    const base = await check(rs, report, tsSrc);
    if (base.length > 0) {
      console.error(
        `${TAG} SELFTEST FAIL — настоящие исходники не зелёные:\n  ${base.join('\n  ')}`,
      );
      process.exit(1);
    }
    // [имя, файл ('ts' | 'rs'), якорь, замена, ожидаемый код]
    const mutants = [
      ['(а) из словаря удалён ключ map', 'ts', "  map: () => 'по карте',\n", '', 'M3'],
      [
        '(б) в Rust добавлен вариант Survey с токеном "survey"',
        'rs',
        '            Self::ActDelete => "act_delete",\n',
        '            Self::ActDelete => "act_delete",\n            Self::Survey => "survey",\n',
        'M3',
        // второй якорь — сам вариант в enum
        ['    ActDelete,\n}', '    ActDelete,\n    /// Инвентаризация.\n    Survey,\n}'],
      ],
      [
        '(в) подпись workstation заменена возвратом в fallback',
        'ts',
        "  workstation: () => 'назначение АРМ',\n",
        '  workstation: () => UNKNOWN_MOVEMENT_REASON,\n',
        'M4',
      ],
      ['(г) из ALL убран ActDelete', 'rs', '        Self::ActDelete,\n    ];', '    ];', 'M1'],
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
      const bad = await check(mRs, report, mTs);
      const ok = bad.some((b) => b.startsWith(`[${code}]`));
      console.log(`${TAG} мутант «${name}»: ${ok ? 'обнаружен' : 'НЕ ОБНАРУЖЕН'} [${code}]`);
      if (!ok) missed++;
    }
    if (missed > 0) process.exit(1);
    console.log(`${TAG} selftest OK — ${mutants.length} мутанта красные`);
    return;
  }

  const bad = await check(rs, report, tsSrc);
  if (bad.length > 0) {
    console.error(`${TAG} FAIL:\n  ${bad.join('\n  ')}`);
    process.exit(1);
  }
  console.log(`${TAG} OK`);
}

main();
