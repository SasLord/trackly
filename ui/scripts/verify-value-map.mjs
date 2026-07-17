#!/usr/bin/env node
// [verify-value-map] One-shot git-diff верификатор value-preserving замены space/radius
// (Phase 23, план 02, D-08). НЕ встраивается в постоянный pnpm lint — после мержа диапазону
// "текущий коммит vs base" нечего будет проверять против main, так что это ручной/one-shot
// инструмент для планов 23-04 (space/radius sweep), не постоянный CI-гейт.
//
// Usage: node scripts/verify-value-map.mjs <base-ref>
//
// Принимает git ref (branch/tag/SHA) как единственный позиционный аргумент, диффует
// .svelte/.scss файлы (кроме _tokens.scss — файл легитимно вводит новые значения, не rename,
// sweep по нему бессмысленен) от <base-ref> до рабочего дерева, разбивает вывод по
// @@-хункам, сравнивает удалённые --space-*/--radius-* с добавленными --tr-space-*/
// --tr-radius-* токенами по позиции внутри хунка против SPACE_MAP/RADIUS-функции ниже.
//
// Zero-dependency: только node:child_process (execSync для `git diff`) и node:url (fileURLToPath
// для run-if-main guard, позволяющего безопасно импортировать именованные экспорты без запуска
// main()/process.exit()).

import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Скопировано дословно из UI-SPEC/REQUIREMENTS.md — не пересчитывать.
const SPACE_MAP = {
  '--space-xs': '--tr-space-2xs',
  '--space-sm': '--tr-space-xs',
  '--space-md': '--tr-space-md',
  '--space-lg': '--tr-space-xl',
  '--space-xl': '--tr-space-2xl',
  '--space-2xl': '--tr-space-4xl',
  '--space-3xl': '--tr-space-5xl',
};

// D-07: split --radius-sm. Безопасный дефолт --tr-radius-xs; только эти 4 файла (field-chrome
// shared-компоненты) получают --tr-radius-sm.
// Note: `git diff` path headers are always repo-root-relative (this repo's root is the parent of
// `ui/`, not `ui/` itself), so entries here must carry the `ui/` prefix to match `filePath` as
// extracted by splitIntoFileHunks() below (b/... side of `diff --git a/... b/...`).
const RADIUS_EXCEPTION_FILES = new Set([
  'ui/src/lib/components/Button.svelte',
  'ui/src/lib/components/Input.svelte',
  'ui/src/lib/components/Select.svelte',
  'ui/src/lib/components/Textarea.svelte',
]);

function expectedRadiusTarget(oldToken, filePath) {
  if (oldToken === '--radius-md') return '--tr-radius-md';
  if (oldToken === '--radius-lg') return '--tr-radius-lg'; // QA-01 fix, "old" был undefined
  if (oldToken === '--radius-sm') {
    return RADIUS_EXCEPTION_FILES.has(filePath) ? '--tr-radius-sm' : '--tr-radius-xs';
  }
  return null;
}

function expectedTarget(oldToken, filePath) {
  if (oldToken.startsWith('--space-')) return SPACE_MAP[oldToken] ?? null;
  if (oldToken.startsWith('--radius-')) return expectedRadiusTarget(oldToken, filePath);
  return null;
}

/** Разбивает unified diff одного файла на @@-хунки, возвращает [{filePath, hunkText}]. */
function splitIntoFileHunks(diffText) {
  const fileSections = diffText.split(/^diff --git /m).slice(1);
  const result = [];
  for (const section of fileSections) {
    const headerMatch = section.match(/^a\/(\S+) b\/(\S+)/);
    const filePath = headerMatch ? headerMatch[2] : '(unknown)';
    const hunks = section.split(/^@@/m).slice(1);
    for (const hunk of hunks) {
      result.push({ filePath, hunkText: hunk });
    }
  }
  return result;
}

/**
 * Извлекает ВСЕ совпадения `re` на строках хунка, начинающихся с `marker` ('-' или '+').
 * CR-01 fix: старая реализация использовала один анкорённый `^` + ленивый `.*?` паттерн на
 * весь текст хунка с `m`-флагом, что даёт максимум одно совпадение на строку — второй и
 * последующие токены на многотокенной строке (например `padding: var(--x) var(--y);`)
 * молча терялись. Здесь построчный обход: для каждой строки, начинающейся с marker, `re`
 * (без `m`-флага, т.к. применяется к уже выделенной одной строке) находит все вхождения.
 */
function tokensOnSide(hunkText, marker, re) {
  const tokens = [];
  for (const line of hunkText.split('\n')) {
    if (!line.startsWith(marker)) continue;
    for (const m of line.matchAll(re)) tokens.push(m[1]);
  }
  return tokens;
}

/**
 * Плоское сравнение токенов внутри каждого @@ hunk, а не построчная пара — устойчиво к тому,
 * что prettier/reflow может сдвинуть многотокенные строки на другое число строк.
 */
function checkHunk(filePath, hunkText) {
  const removedTokens = tokensOnSide(hunkText, '-', /(--(?:space|radius)-[a-z0-9]+)/g);
  const addedTokens = tokensOnSide(hunkText, '+', /(--tr-(?:space|radius)-[a-z0-9]+)/g);

  if (removedTokens.length === 0 && addedTokens.length === 0) {
    return { violations: [], checked: false };
  }

  const violations = [];
  if (removedTokens.length !== addedTokens.length) {
    violations.push({
      reason: 'count-mismatch',
      file: filePath,
      removedTokens,
      addedTokens,
    });
    return { violations, checked: true };
  }

  for (let i = 0; i < removedTokens.length; i++) {
    const oldToken = removedTokens[i];
    const newToken = addedTokens[i];
    const expected = expectedTarget(oldToken, filePath);
    if (expected == null) {
      // Токен вне известной карты (не space/radius family мы отслеживаем, либо новое
      // неотображённое имя) — не наша забота этого верификатора, пропускаем без violation.
      continue;
    }
    if (newToken !== expected) {
      violations.push({
        reason: 'value-mismatch',
        file: filePath,
        oldToken,
        newToken,
        expected,
      });
    }
  }
  return { violations, checked: true };
}

function main() {
  const baseRef = process.argv[2];
  if (!baseRef) {
    console.error('[verify-value-map] Usage: node scripts/verify-value-map.mjs <base-ref>');
    process.exit(1);
  }

  let diffText;
  try {
    diffText = execSync(
      `git diff --unified=0 ${JSON.stringify(baseRef)} -- '*.svelte' '*.scss' ':!src/styles/_tokens.scss'`,
      {
        cwd: new URL('..', import.meta.url).pathname,
        encoding: 'utf8',
        maxBuffer: 1024 * 1024 * 64,
      },
    );
  } catch (err) {
    console.error(`[verify-value-map] git diff failed: ${err.message}`);
    process.exit(1);
  }

  const fileHunks = splitIntoFileHunks(diffText);
  let checkedCount = 0;
  let violationCount = 0;

  for (const { filePath, hunkText } of fileHunks) {
    const { violations, checked } = checkHunk(filePath, hunkText);
    if (checked) checkedCount++;
    for (const v of violations) {
      violationCount++;
      if (v.reason === 'count-mismatch') {
        console.error(
          `[verify-value-map] ${v.file} — count-mismatch: removed=[${v.removedTokens.join(', ')}] added=[${v.addedTokens.join(', ')}]`,
        );
      } else {
        console.error(
          `[verify-value-map] ${v.file} — value-mismatch: ${v.oldToken} → ${v.newToken} (expected ${v.expected})`,
        );
      }
    }
  }

  if (violationCount > 0) {
    console.error(
      `[verify-value-map] FAIL — ${checkedCount} хунков проверено, ${violationCount} нарушений`,
    );
    process.exit(1);
  }

  console.error(`[verify-value-map] PASS — ${checkedCount} хунков проверено, 0 нарушений`);
  process.exit(0);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();

export { tokensOnSide, checkHunk };
