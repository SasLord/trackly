#!/usr/bin/env node
// [check-focus-outline] Постоянный CI-гейт голого `outline: none` без парного `box-shadow`
// в охватывающем правиле (Phase 30, план 01, D-04).
//
// Сканирует <style>-блок каждого .svelte-файла, находит все вхождения `outline: none;` и
// для каждого определяет НЕПОСРЕДСТВЕННО ОХВАТЫВАЮЩЕЕ CSS-правило подсчётом глубины фигурных
// скобок (не regex по имени селектора — подсчёт скобок корректно обрабатывает как
// same-block паттерн: `&:focus-visible { outline: none; box-shadow: ...; }`, так и
// cross-nested-block паттерн: `.tab { outline: none; &:focus-visible { box-shadow: ...; } }`,
// где outline:none и box-shadow — в разных вложенных под-правилах одного охватывающего блока).
//
// Если `box-shadow` встречается где угодно внутри охватывающего правила (включая вложенные
// под-правила) — вхождение безопасно. Если нет — нарушение. Whitelist-исключение: строка с
// `outline: none;` (или строка непосредственно перед ней) содержит `// check-focus-outline:
// ignore` — вхождение пропускается без нарушения (механизм на случай будущих ложных
// срабатываний, ни разу не понадобившийся на момент написания).
//
// Zero-dependency: только node:fs/node:path/node:url.
//
// Usage: node scripts/check-focus-outline.mjs

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const UI_ROOT = path.resolve(__dirname, '..');

/** Собирает список .svelte/.scss файлов рекурсивно. Устойчив к отсутствующей директории. */
function collectSourceFiles(srcDir) {
  if (!fs.existsSync(srcDir)) return [];
  let entries;
  try {
    entries = fs.readdirSync(srcDir, { recursive: true });
  } catch {
    return [];
  }
  const files = [];
  for (const entry of entries) {
    const rel = typeof entry === 'string' ? entry : String(entry);
    if (!rel.endsWith('.svelte') && !rel.endsWith('.scss')) continue;
    const full = path.join(srcDir, rel);
    try {
      if (fs.statSync(full).isFile()) files.push(full);
    } catch {
      // race/broken symlink — skip
    }
  }
  return files;
}

function readFileSafe(filePath) {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}

function lineNumberAt(content, index) {
  let line = 1;
  for (let i = 0; i < index && i < content.length; i++) {
    if (content[i] === '\n') line++;
  }
  return line;
}

function relPath(filePath) {
  return path.relative(UI_ROOT, filePath);
}

const STYLE_BLOCK_RE = /<style[^>]*>([\s\S]*?)<\/style>/g;
const OUTLINE_NONE_RE = /outline:\s*none\s*;/g;
const IGNORE_MARKER = 'check-focus-outline: ignore';

/**
 * Находит непосредственно охватывающее правило для позиции `matchIndex` внутри `block`
 * подсчётом глубины скобок: строит стек индексов открывающих `{` от начала блока до
 * matchIndex (закрывающие `}` снимают со стека) — вершина стека это открывающая скобка
 * охватывающего правила. Затем от неё находит парную закрывающую скобку счётчиком глубины.
 * Возвращает текст всего правила (включая вложенные под-правила).
 */
function findEnclosingRule(block, matchIndex) {
  const stack = [];
  for (let i = 0; i < matchIndex; i++) {
    if (block[i] === '{') stack.push(i);
    else if (block[i] === '}') stack.pop();
  }
  if (stack.length === 0) return null;
  const openIdx = stack[stack.length - 1];
  let depth = 0;
  let i = openIdx;
  for (; i < block.length; i++) {
    if (block[i] === '{') depth++;
    else if (block[i] === '}') {
      depth--;
      if (depth === 0) break;
    }
  }
  return block.slice(openIdx, i + 1);
}

function isIgnored(content, matchStartIndexInContent) {
  const lines = content.split('\n');
  const matchLine = lineNumberAt(content, matchStartIndexInContent); // 1-based
  const currentLine = lines[matchLine - 1] ?? '';
  const previousLine = matchLine >= 2 ? (lines[matchLine - 2] ?? '') : '';
  return currentLine.includes(IGNORE_MARKER) || previousLine.includes(IGNORE_MARKER);
}

function checkFocusOutline(files) {
  const violations = [];
  for (const filePath of files) {
    if (!filePath.endsWith('.svelte')) continue;
    const content = readFileSafe(filePath);
    if (content == null) continue;

    for (const styleMatch of content.matchAll(STYLE_BLOCK_RE)) {
      const block = styleMatch[1];
      const blockStart = styleMatch.index + styleMatch[0].indexOf(block);

      for (const outlineMatch of block.matchAll(OUTLINE_NONE_RE)) {
        const absoluteIndex = blockStart + outlineMatch.index;

        if (isIgnored(content, absoluteIndex)) continue;

        const enclosingRule = findEnclosingRule(block, outlineMatch.index);
        const hasBoxShadow = enclosingRule != null && /box-shadow/.test(enclosingRule);
        if (hasBoxShadow) continue;

        violations.push({
          file: filePath,
          line: lineNumberAt(content, absoluteIndex),
        });
      }
    }
  }
  return violations;
}

function main() {
  const files = collectSourceFiles(path.join(UI_ROOT, 'src'));
  const violations = checkFocusOutline(files);

  for (const v of violations) {
    console.error(
      `[check-focus-outline] ${relPath(v.file)}:${v.line} — bare outline: none without a paired box-shadow in the enclosing rule`,
    );
  }

  if (violations.length > 0) {
    console.error(`[check-focus-outline] FAIL — ${violations.length} нарушений`);
    process.exit(1);
  }

  console.error('[check-focus-outline] PASS — 0 нарушений');
  process.exit(0);
}

main();
