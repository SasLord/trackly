#!/usr/bin/env node
// [check-privacy] Единый durable-гейт приватности (PRIV-01/PRIV-02).
//
// Почему он существует: репозиторий ПУБЛИЧНЫЙ, и CLAUDE.md формулирует
// требование явно: «Всё закоммиченное остаётся в истории git даже после
// удаления из HEAD — проверять ДО коммита, а не после». Чистка HEAD (PRIV-01,
// фаза 37, планы 37-01/37-02) необходима, но недостаточна — единственный
// durable-контроль — не пускать реквизит-, ФИО- и инфраструктура-подобные
// значения в репозиторий вообще. Этот гейт вбирает старый
// scripts/check-privacy-requisites.sh (режим 1) и добавляет режим 2 (хэши
// n-грамм) + контроль бинарных расширений (R8) — вместе покрывают PRIV-02
// (R5–R9).
//
// Режимы:
//   - Режим 1 (allowlist) — литералы requisite-ключей (inn/kpp/okpo/ogrn/
//     phone/fax) в *.rs/*.html должны входить в явный список ALLOWED —
//     миграция scripts/check-privacy-requisites.sh без регрессии (C-02).
//   - Режим 2 (n-грамм хэши) — весь текстовый HEAD/staged токенизируется
//     1–3-словными n-граммами, нормализуется (lowercase + ё→е + NFC) и
//     сверяется по SHA-256 со списком из файла --hashes (D-05/D-06/D-07).
//   - Контроль бинарных расширений (R8) — .docx/.xlsx/.pdf/.png/.jpg/.jpeg
//     вне явного BINARY_ALLOWLIST — нарушение (класс D).
//
// На срабатывании гейт печатает ТОЛЬКО «путь:строка — маркер класса X»
// (D-16) — никогда не значение и не исходную строку.
//
// --hashes <path> обязателен (нет дефолтного пути к продовому файлу в этом
// плане — scripts/privacy-tokens.sha256 ещё не существует, это план 37-04).
//
// Zero-dependency: только node:fs/path/url/crypto/child_process/readline.
//
// Usage:
//   node scripts/check-privacy.mjs --hashes <path>              # весь HEAD
//   node scripts/check-privacy.mjs --staged --hashes <path>     # staged-файлы
//   node scripts/check-privacy.mjs --hashes <path> <file...>    # конкретные файлы (fixtures/self-test)
//   node scripts/check-privacy.mjs --add --hashes <path>        # интерактивно добавить токен

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import readline from 'node:readline';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-privacy]';

// ---------------------------------------------------------------------------
// Режим 1 (allowlist) — портировано verbatim из
// scripts/check-privacy-requisites.sh (C-02), без изменений значений.
// ---------------------------------------------------------------------------

const ALLOWED = [
  // Preview demo context (template_service.rs)
  '+7 495 123-45-67',
  '+7 495 123-45-68',
  '7700000000',
  '770000000',
  '12345678',
  '1027700123456',
  // Integration-test fixtures
  '+7 495 000-00-00',
  '+7 495 000-00-01',
  '+7 495 000-00-02',
  '7712345678',
  '771001001',
  '771201001',
  '87654321',
  '1027700654321',
  '1027700000000',
  // First-run placeholder org (organization_service.rs::placeholder)
  '0000000000',
  '000000000',
  // organization_io.rs fixtures («ООО Ромашка», path-traversal lure)
  '770001001',
  '1',
  '2',
  // pdf_render_act.rs org.json fixture
  '1234567890',
  '111222333',
  // Structurally-empty values
  '',
];
const ALLOWED_SET = new Set(ALLOWED);

// Word-boundary anchored; matches JSON-ish ("phone": "…") and Rust
// struct-init (phone: "…") assignment shapes. Ported verbatim (translated
// [[:space:]] -> \s) from check-privacy-requisites.sh's PATTERN.
const REQUISITE_PATTERN =
  /(^|[^A-Za-z0-9_"])"?(inn|kpp|okpo|ogrn|phone|fax)"?\s*:\s*"([^"]*)"/g;

// Matches a real `*.rs`/`*.html` file (extension at the very end), AND a
// `*.rs.txt`/`*.html.txt`-shaped fixture (extension chain, not just the
// final one) — the latter shape is used by
// scripts/fixtures/privacy/allowlist-regression.rs.txt (C-02 self-test): it
// needs a trailing `.txt` so `cargo build` never picks it up as real Rust
// source, while still being recognized here as "Rust-shaped" content.
const REQUISITE_FILE_RE = /\.(rs|html)(\.|$)/;

// ---------------------------------------------------------------------------
// Режим 2 (n-грамм хэши) — токенизатор D-05/D-06.
// ---------------------------------------------------------------------------

const WORD_RE = /[\p{L}\p{N}]+/gu;

/** lowercase + ё→е + Unicode NFC. Пунктуация уже отсечена самим WORD_RE
 * (он извлекает только буквенно-цифровые последовательности), отдельного
 * шага очистки пунктуации не требуется (D-06). */
function normalize(s) {
  return s.toLowerCase().replace(/ё/g, 'е').normalize('NFC');
}

/** Скользящее окно 1..3 слов по совпадениям WORD_RE в строке. */
function extractNgrams(line) {
  const words = [...line.matchAll(WORD_RE)].map((m) => m[0]);
  const ngrams = [];
  for (let n = 1; n <= 3; n++) {
    for (let i = 0; i + n <= words.length; i++) {
      ngrams.push(words.slice(i, i + n).join(' '));
    }
  }
  return ngrams;
}

function sha256Hex(s) {
  return crypto.createHash('sha256').update(s, 'utf8').digest('hex');
}

// ---------------------------------------------------------------------------
// Контроль бинарных расширений (R8, класс D)
// ---------------------------------------------------------------------------

const BINARY_EXTENSIONS = new Set(['.docx', '.xlsx', '.pdf', '.png', '.jpg', '.jpeg']);

// Пути (repo-root-relative, POSIX-разделители), которым явно разрешено
// нести контролируемое бинарное расширение — всё остальное с таким
// расширением есть нарушение. Добавлять новые записи только для заведомо
// не-чувствительных бинарников (иконки, известные безопасные тестовые
// фикстуры), никогда как разрешение на весь каталог без разбора.
const BINARY_ALLOWLIST_PREFIXES = ['crates/trackly-app/icons/'];
const BINARY_ALLOWLIST_EXACT = new Set([
  'crates/trackly-app/tests/fixtures/logo_test.png',
]);

function isBinaryAllowed(relPath) {
  if (BINARY_ALLOWLIST_EXACT.has(relPath)) return true;
  return BINARY_ALLOWLIST_PREFIXES.some((prefix) => relPath.startsWith(prefix));
}

// ---------------------------------------------------------------------------
// Исключения путей (D-13/D-14)
// ---------------------------------------------------------------------------

// Жёстко закодированные константы, а не отдельный конфиг-файл (D-14) —
// изменение видно в diff при code review, никакого per-токен механизма
// отключения нигде в исходнике нет.
//
// `.planning/phases/37-data-privacy/` СОЗНАТЕЛЬНО НЕ входит в этот список
// (D-13): планировочные артефакты самой этой фазы (PLAN/SUMMARY/RESEARCH и
// т.д.) должны сканироваться наравне со всеми остальными файлами. Не
// добавляй его сюда «для удобства» при будущих правках.
const EXCLUDED_PATH_PREFIXES = ['node_modules/', 'target/'];
const EXCLUDED_PATH_EXACT = new Set(['Cargo.lock']);

function isExcludedPath(relPath, hashesRelPath) {
  if (hashesRelPath && relPath === hashesRelPath) return true;
  if (EXCLUDED_PATH_EXACT.has(relPath)) return true;
  return EXCLUDED_PATH_PREFIXES.some((prefix) => relPath.startsWith(prefix));
}

// ---------------------------------------------------------------------------
// Сбор целей сканирования (git-плампинг, NUL-delimited — в репозитории есть
// имена файлов с пробелами/кириллицей).
// ---------------------------------------------------------------------------

function toPosix(relPath) {
  return relPath.split(path.sep).join('/');
}

function collectStagedTargets() {
  let raw;
  try {
    raw = execFileSync(
      'git',
      ['diff', '--cached', '--name-status', '-z', '--diff-filter=ACMR'],
      { cwd: REPO_ROOT, encoding: 'utf8' },
    );
  } catch (err) {
    console.error(`${TAG} FAIL — не удалось получить список staged-файлов: ${err.message}`);
    process.exit(1);
  }
  const records = raw.split('\0').filter((r) => r.length > 0);
  const targets = [];
  let i = 0;
  while (i < records.length) {
    const status = records[i];
    i++;
    let relPath;
    if (status[0] === 'R' || status[0] === 'C') {
      // Rename/copy: старый путь, затем новый путь.
      i++; // skip old path
      relPath = records[i];
      i++;
    } else {
      relPath = records[i];
      i++;
    }
    if (relPath === undefined) continue;
    let content = '';
    try {
      content = execFileSync('git', ['show', `:${relPath}`], {
        cwd: REPO_ROOT,
        encoding: 'utf8',
        maxBuffer: 64 * 1024 * 1024,
      });
    } catch {
      continue;
    }
    targets.push({ path: relPath, content });
  }
  return targets;
}

function collectHeadTargets() {
  let raw;
  try {
    raw = execFileSync('git', ['ls-files', '-z'], { cwd: REPO_ROOT, encoding: 'utf8' });
  } catch (err) {
    console.error(`${TAG} FAIL — не удалось получить список файлов HEAD: ${err.message}`);
    process.exit(1);
  }
  const paths = raw.split('\0').filter((p) => p.length > 0);
  const targets = [];
  for (const relPath of paths) {
    const abs = path.join(REPO_ROOT, relPath);
    let content;
    try {
      content = fs.readFileSync(abs, 'utf8');
    } catch {
      continue;
    }
    targets.push({ path: relPath, content });
  }
  return targets;
}

/** Явно перечисленные файлы (fixtures/self-test): читаются напрямую через
 * fs, без git-плампинга, чтобы работать даже с некоммиченными файлами. */
function collectExplicitTargets(files) {
  const targets = [];
  for (const file of files) {
    const abs = path.resolve(process.cwd(), file);
    const relPath = toPosix(path.relative(REPO_ROOT, abs));
    let content = '';
    try {
      content = fs.readFileSync(abs, 'utf8');
    } catch {
      content = '';
    }
    targets.push({ path: relPath, content });
  }
  return targets;
}

// ---------------------------------------------------------------------------
// Сканеры
// ---------------------------------------------------------------------------

function scanAllowlist(targets) {
  const violations = [];
  for (const { path: p, content } of targets) {
    if (!REQUISITE_FILE_RE.test(p)) continue;
    const lines = content.split('\n');
    lines.forEach((line, idx) => {
      REQUISITE_PATTERN.lastIndex = 0;
      let m;
      while ((m = REQUISITE_PATTERN.exec(line)) !== null) {
        const value = m[3];
        if (!ALLOWED_SET.has(value)) {
          violations.push({ path: p, line: idx + 1, cls: 'requisite' });
        }
        if (m.index === REQUISITE_PATTERN.lastIndex) REQUISITE_PATTERN.lastIndex++;
      }
    });
  }
  return violations;
}

function scanHashes(targets, hashMap) {
  const violations = [];
  for (const { path: p, content } of targets) {
    const lines = content.split('\n');
    lines.forEach((line, idx) => {
      const seenClasses = new Set();
      for (const ngram of extractNgrams(line)) {
        const norm = normalize(ngram);
        const hash = sha256Hex(norm);
        const cls = hashMap.get(hash);
        if (cls && !seenClasses.has(cls)) {
          violations.push({ path: p, line: idx + 1, cls });
          seenClasses.add(cls);
        }
      }
    });
  }
  return violations;
}

function scanBinaries(targets) {
  const violations = [];
  for (const { path: p } of targets) {
    const ext = path.extname(p).toLowerCase();
    if (!BINARY_EXTENSIONS.has(ext)) continue;
    if (isBinaryAllowed(p)) continue;
    violations.push({ path: p, line: 0, cls: 'D' });
  }
  return violations;
}

// ---------------------------------------------------------------------------
// Загрузка файла хэшей — fail-closed (R7)
// ---------------------------------------------------------------------------

function loadHashes(hashesPath) {
  let raw;
  try {
    raw = fs.readFileSync(hashesPath, 'utf8');
  } catch (err) {
    console.error(
      `${TAG} FAIL — не удалось прочитать файл хэшей: ${hashesPath} (${err.code || err.message})`,
    );
    console.error(
      `${TAG} Восстановление: проверь путь к --hashes; см. scripts/fixtures/privacy/README.md за примером формата. Пустой/отсутствующий файл хэшей намеренно приводит к отказу (fail-closed, R7), а не к молчаливому пропуску проверки.`,
    );
    process.exit(1);
  }
  const map = new Map();
  for (const rawLine of raw.split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const m = line.match(/^([0-9a-f]{64})\s+([ABC])$/);
    if (!m) continue; // некорректная строка данных — пропускается, не фатально
    map.set(m[1], m[2]);
  }
  if (map.size === 0) {
    console.error(`${TAG} FAIL — файл хэшей пуст или не содержит валидных строк: ${hashesPath}`);
    console.error(
      `${TAG} Восстановление: файл хэшей обязателен и не может быть пустым (fail-closed, R7).`,
    );
    process.exit(1);
  }
  return map;
}

// ---------------------------------------------------------------------------
// --add: интерактивное добавление токена (D-15).
// ---------------------------------------------------------------------------

/** Приглашение с подавленным эхом на raw-mode stdin — значение никогда не
 * отображается на экране и не попадает в ~/.zsh_history (читается через
 * process.stdin, не как аргумент командной строки). */
function promptHidden(question) {
  return new Promise((resolve) => {
    process.stdout.write(question);
    const stdin = process.stdin;
    stdin.setRawMode(true);
    stdin.resume();
    stdin.setEncoding('utf8');
    let input = '';
    const cleanup = () => {
      stdin.setRawMode(false);
      stdin.pause();
      stdin.removeListener('data', onData);
    };
    // Сравниваем по коду символа (не по литералу управляющего байта), чтобы
    // в исходнике не было буквальных непечатаемых байтов CR/LF/EOF/backspace.
    function onData(chunk) {
      const char = chunk.toString();
      const code = char.charCodeAt(0);
      if (code === 13 || code === 10 || code === 4) {
        // Enter (CR/LF) или Ctrl+D (EOF)
        cleanup();
        process.stdout.write('\n');
        resolve(input);
      } else if (code === 3) {
        // Ctrl+C
        cleanup();
        process.stdout.write('\n');
        process.exit(1);
      } else if (code === 127 || code === 8) {
        // Backspace / Delete
        input = input.slice(0, -1);
      } else {
        input += char;
      }
    }
    stdin.on('data', onData);
  });
}

function readExistingHashLines(hashesPath) {
  let raw;
  try {
    raw = fs.readFileSync(hashesPath, 'utf8');
  } catch {
    return { headerLines: [], dataLines: new Set() };
  }
  const headerLines = [];
  const dataLines = new Set();
  for (const rawLine of raw.split('\n')) {
    const line = rawLine.trim();
    if (!line) continue;
    if (line.startsWith('#')) {
      headerLines.push(rawLine);
    } else if (/^[0-9a-f]{64}\s+[ABC]$/.test(line)) {
      dataLines.add(line);
    }
  }
  return { headerLines, dataLines };
}

async function runAdd(args) {
  // Pitfall 4: process.stdin.setRawMode is undefined off a real TTY (CI,
  // pipes, non-interactive wrappers) — check BEFORE ever touching raw mode,
  // so the failure is an explicit message, never an uncaught TypeError.
  if (!process.stdin.isTTY) {
    console.error(
      `${TAG} FAIL — --add требует интерактивного терминала (stdin не TTY). Запусти "node scripts/check-privacy.mjs --add --hashes <path>" напрямую в терминале, не из пайпа/CI.`,
    );
    process.exit(1);
  }

  if (!args.hashesPath) {
    console.error(
      `${TAG} FAIL — --add требует --hashes <path> — файл, в который добавляется токен.`,
    );
    process.exit(1);
  }

  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const clsAnswer = await new Promise((resolve) => {
    rl.question('Класс (A/B/C): ', (answer) => resolve(answer.trim().toUpperCase()));
  });
  rl.close();

  if (!['A', 'B', 'C'].includes(clsAnswer)) {
    console.error(`${TAG} FAIL — класс должен быть одним из A/B/C, получено: "${clsAnswer}"`);
    process.exit(1);
  }

  const value = await promptHidden('Значение (ввод не отображается на экране): ');
  if (!value) {
    console.error(`${TAG} FAIL — пустое значение, нечего добавлять.`);
    process.exit(1);
  }

  const normalized = normalize(value);
  const hash = sha256Hex(normalized);
  const newLine = `${hash} ${clsAnswer}`;

  const { headerLines, dataLines } = readExistingHashLines(args.hashesPath);
  dataLines.add(newLine);
  const sortedData = [...dataLines].sort();

  const output = `${[...headerLines, ...sortedData].join('\n')}\n`;
  fs.writeFileSync(args.hashesPath, output, 'utf8');

  console.error(`${TAG} PASS — токен класса ${clsAnswer} добавлен в ${args.hashesPath}`);
  process.exit(0);
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = { staged: false, hashesPath: null, add: false, files: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--staged') {
      args.staged = true;
    } else if (a === '--hashes') {
      args.hashesPath = argv[++i] ?? null;
    } else if (a === '--add') {
      args.add = true;
    } else {
      args.files.push(a);
    }
  }
  return args;
}

function formatViolation(v) {
  if (v.line && v.line > 0) return `${TAG} ${v.path}:${v.line} — маркер класса ${v.cls}`;
  return `${TAG} ${v.path} — маркер класса ${v.cls}`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.add) {
    await runAdd(args);
    return;
  }

  if (!args.hashesPath) {
    console.error(
      `${TAG} FAIL — флаг --hashes <path> обязателен (в этом плане нет дефолтного пути к продовому файлу — см. план 37-04).`,
    );
    process.exit(1);
  }

  const hashMap = loadHashes(args.hashesPath);

  let targets;
  if (args.files.length > 0) {
    targets = collectExplicitTargets(args.files);
  } else if (args.staged) {
    targets = collectStagedTargets();
  } else {
    targets = collectHeadTargets();
  }

  const hashesAbs = path.resolve(process.cwd(), args.hashesPath);
  const hashesRel = toPosix(path.relative(REPO_ROOT, hashesAbs));
  targets = targets.filter((t) => !isExcludedPath(t.path, hashesRel));

  const violations = [
    ...scanAllowlist(targets),
    ...scanHashes(targets, hashMap),
    ...scanBinaries(targets),
  ];

  for (const v of violations) console.error(formatViolation(v));

  if (violations.length > 0) {
    console.error(`${TAG} FAIL — ${violations.length} нарушений`);
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

await main();
