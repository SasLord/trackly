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
//     phone/fax, регистронезависимо, в кавычках и без) в исходниках и
//     структурированных конфигах (см. REQUISITE_FILE_RE) должны входить в
//     явный список ALLOWED — миграция scripts/check-privacy-requisites.sh
//     без регрессии (C-02).
//   - Режим 2 (n-грамм хэши) — весь текстовый HEAD/staged токенизируется
//     1–3-словными n-граммами, нормализуется (lowercase + ё→е + NFC) и
//     сверяется по SHA-256 со списком из файла --hashes (D-05/D-06/D-07).
//   - Контроль бинарных расширений (R8) — .docx/.xlsx/.pdf/.png/.jpg/.jpeg
//     вне явного BINARY_ALLOWLIST — нарушение (класс D).
//
// На срабатывании гейт печатает ТОЛЬКО «путь:строка — маркер класса X»
// (D-16) — никогда не значение и не исходную строку.
//
// Fail-closed сквозной (R7): не только отсутствующий/пустой файл --hashes,
// но и любая цель, содержимое которой не удалось прочитать, приводит к
// exit 1 — молчаливый пропуск цели означал бы «PASS», не проверив её.
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

// Word-boundary anchored; matches JSON-ish (quoted-key: quoted-value) and
// Rust struct-init (bare-key: quoted-value) assignment shapes. Ported from
// check-privacy-requisites.sh's PATTERN (translated [[:space:]] -> \s),
// then widened twice, both times verified to add zero matches across the
// whole repository (only detection can widen — ALLOWED is unchanged):
//   - WR-02: the /i flag, so INN:/Inn:-cased keys (plausible in generated
//     or copy-pasted JSON) no longer bypass mode 1 entirely;
//   - WR-03: a second, bare-numeric branch, so an unquoted literal
//     (`inn: 7700000000,` — a Rust field typed i64/u64 rather than String)
//     is checked against ALLOWED too, instead of not matching at all.
const REQUISITE_PATTERN_QUOTED =
  /(^|[^A-Za-z0-9_"])"?(inn|kpp|okpo|ogrn|phone|fax)"?\s*:\s*"([^"]*)"/gi;
const REQUISITE_PATTERN_BARE =
  /(^|[^A-Za-z0-9_"])"?(inn|kpp|okpo|ogrn|phone|fax)"?\s*:\s*(\d[\d_]*)/gi;

// `bare: true` — значение записано без кавычек; перед сверкой с ALLOWED из
// него убираются разделители разрядов Rust (7_700_000_000 → 7700000000),
// иначе разрешённое значение читалось бы как нарушение.
const REQUISITE_PATTERNS = [
  { re: REQUISITE_PATTERN_QUOTED, bare: false },
  { re: REQUISITE_PATTERN_BARE, bare: true },
];

// Matches a real source/config file (extension at the very end), AND an
// `*.rs.txt`/`*.html.txt`-shaped fixture (extension chain, not just the
// final one) — the latter shape is used by
// scripts/fixtures/privacy/allowlist-regression.rs.txt (C-02 self-test): it
// needs a trailing `.txt` so `cargo build` never picks it up as real Rust
// source, while still being recognized here as "Rust-shaped" content.
//
// WR-01: originally `.rs`/`.html` only, which left a real requisite literal
// in a JSON/TOML/YAML/Svelte/TS config or fixture visible to mode 2 alone —
// and mode 2 can only catch a value someone already hashed via --add, a
// chicken-and-egg gap for genuinely new data. Widened to the structured-data
// and source types this project actually uses. Deliberately EXCLUDES `.md`
// and `.mjs`/`.js`: planning docs and this script's own comments carry
// template placeholders and prose in the exact `inn: "…"` shape, and mode 2
// already covers those files' content.
const REQUISITE_FILE_RE =
  /\.(rs|html|json|toml|ya?ml|svelte|ts|tsx|jsx|css|scss|minijinja|sql|txt|csv)(\.|$)/;

// ---------------------------------------------------------------------------
// Режим 2 (n-грамм хэши) — токенизатор D-05/D-06.
// ---------------------------------------------------------------------------

const WORD_RE = /[\p{L}\p{N}]+/gu;

// Верхняя граница скользящего окна n-грамм. Используется И сканером, И
// --add (canonicalizeAddValue) — они обязаны видеть одно и то же число,
// иначе --add снова начнёт принимать значения, которые сканер никогда не
// сможет воспроизвести (CR-01).
const NGRAM_MAX_WORDS = 3;

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
  for (let n = 1; n <= NGRAM_MAX_WORDS; n++) {
    for (let i = 0; i + n <= words.length; i++) {
      ngrams.push(words.slice(i, i + n).join(' '));
    }
  }
  return ngrams;
}

function sha256Hex(s) {
  return crypto.createHash('sha256').update(s, 'utf8').digest('hex');
}

/** Приводит введённое в --add значение К ТОЙ ЖЕ форме, которую построит
 * сканер (CR-01).
 *
 * Сканер никогда не хэширует сырую строку: он режет её по WORD_RE и
 * склеивает 1..NGRAM_MAX_WORDS слов ОДНИМ пробелом ASCII (extractNgrams).
 * Поэтому хэш от сырого ввода совпадает с хэшем сканера только если ввод
 * уже был именно такой формы. Значение с дефисом, точкой, неразрывным
 * пробелом (частая форма русских двойных фамилий) или длиннее
 * NGRAM_MAX_WORDS слов давало мёртвую запись: файл хэшей растёт, оператор
 * видит «PASS — токен добавлен», а гейт по этому значению не сработает
 * никогда.
 *
 * Возвращает {ok:false, reason} для значений, которые сканер не способен
 * воспроизвести — вызывающий обязан отказать, а не записать их молча. */
function canonicalizeAddValue(value) {
  const words = [...value.matchAll(WORD_RE)].map((m) => m[0]);
  if (words.length === 0) {
    return { ok: false, reason: 'no_words', words: 0 };
  }
  if (words.length > NGRAM_MAX_WORDS) {
    return { ok: false, reason: 'too_many_words', words: words.length };
  }
  const canonical = words.join(' ');
  return {
    ok: true,
    canonical,
    words: words.length,
    hash: sha256Hex(normalize(canonical)),
  };
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

// Self-test fixtures (scripts/fixtures/privacy/) are DELIBERATELY-violating
// synthetic data — allowlist-regression.rs.txt carries an intentionally
// unrecognized inn/ogrn literal (C-02 regression) and binary-regression.docx
// sits outside BINARY_ALLOWLIST on purpose (R8 regression). Both must still
// trip when check-privacy.selftest.mjs names them explicitly (positional
// file-argument mode, unaffected by this constant), or the self-test's own
// regression assertions go blind. But once committed to HEAD, an
// auto-discovery scan (--staged / full HEAD — exactly the modes the
// pre-commit hook and ci-fast.yml use) would otherwise trip on this test
// furniture forever, which is not a real privacy violation. This is a
// SEPARATE, narrowly-scoped, mode-aware constant — not an addition to
// EXCLUDED_PATH_PREFIXES/EXCLUDED_PATH_EXACT (R9 keeps those exactly as
// plan 37-03 left them) — and it never disables token-hash checking for any
// real repository path.
const AUTO_SCAN_EXCLUDED_PREFIXES = ['scripts/fixtures/privacy/'];

function isAutoScanExcludedFixture(relPath) {
  return AUTO_SCAN_EXCLUDED_PREFIXES.some((prefix) => relPath.startsWith(prefix));
}

// ---------------------------------------------------------------------------
// Сбор целей сканирования (git-плампинг, NUL-delimited — в репозитории есть
// имена файлов с пробелами/кириллицей).
// ---------------------------------------------------------------------------

function toPosix(relPath) {
  return relPath.split(path.sep).join('/');
}

// Сборщики целей возвращают {targets, unreadable}. Файл, содержимое
// которого прочитать не удалось, НИКОГДА не выбрасывается молча (CR-02):
// раньше bare `catch { continue; }` убирал такой файл из набора
// сканирования, и прогон всё равно заканчивался «PASS — 0 нарушений» с
// кодом 0 — fail-open в гейте, весь контракт которого fail-closed (R7).
// Причины реальны: нет прав на чтение, staged-блоб больше maxBuffer у
// `git show` (64 МБ — ровно тот случайный дамп/выгрузка, ради которых гейт
// и существует), битый симлинк, сбой git. Вызывающий (main) обязан
// превратить непустой unreadable в exit 1.
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
  const unreadable = [];
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
    } catch (err) {
      unreadable.push({ path: relPath, reason: err.code || err.message });
      continue;
    }
    targets.push({ path: relPath, content });
  }
  return { targets, unreadable };
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
  const unreadable = [];
  for (const relPath of paths) {
    const abs = path.join(REPO_ROOT, relPath);
    let content;
    try {
      content = fs.readFileSync(abs, 'utf8');
    } catch (err) {
      unreadable.push({ path: relPath, reason: err.code || err.message });
      continue;
    }
    targets.push({ path: relPath, content });
  }
  return { targets, unreadable };
}

/** Явно перечисленные файлы (fixtures/self-test): читаются напрямую через
 * fs, без git-плампинга, чтобы работать даже с некоммиченными файлами. */
function collectExplicitTargets(files) {
  const targets = [];
  const unreadable = [];
  for (const file of files) {
    const abs = path.resolve(process.cwd(), file);
    const relPath = toPosix(path.relative(REPO_ROOT, abs));
    let content;
    try {
      content = fs.readFileSync(abs, 'utf8');
    } catch (err) {
      // Тот же fail-closed контракт, что и у авто-режимов (CR-02): раньше
      // здесь подставлялась пустая строка, из-за чего явно названный, но
      // нечитаемый файл давал «PASS — 0 нарушений».
      unreadable.push({ path: relPath, reason: err.code || err.message });
      continue;
    }
    targets.push({ path: relPath, content });
  }
  return { targets, unreadable };
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
      for (const { re, bare } of REQUISITE_PATTERNS) {
        re.lastIndex = 0;
        let m;
        while ((m = re.exec(line)) !== null) {
          const value = bare ? m[3].replace(/_/g, '') : m[3];
          if (!ALLOWED_SET.has(value)) {
            violations.push({ path: p, line: idx + 1, cls: 'requisite' });
          }
          if (m.index === re.lastIndex) re.lastIndex++;
        }
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
    //
    // WR-05: один `data`-эвент может нести НЕСКОЛЬКО символов (вставка из
    // буфера, серия быстрых нажатий). Раньше управляющий символ искали
    // только в позиции 0, а в обычной ветке дописывали чанк целиком —
    // хвост чанка после Enter/Backspace терялся или применялся неверно, а
    // управляющий байт внутри вставки попадал в хэшируемое значение. Идём
    // посимвольно и не пропускаем управляющие символы в значение.
    function onData(chunk) {
      for (const char of chunk.toString()) {
        const code = char.charCodeAt(0);
        if (code === 13 || code === 10 || code === 4) {
          // Enter (CR/LF) или Ctrl+D (EOF)
          cleanup();
          process.stdout.write('\n');
          resolve(input);
          return;
        }
        if (code === 3) {
          // Ctrl+C
          cleanup();
          process.stdout.write('\n');
          process.exit(1);
        } else if (code === 127 || code === 8) {
          // Backspace / Delete
          input = input.slice(0, -1);
        } else if (code >= 32) {
          input += char;
        }
        // прочие управляющие символы (Tab, Esc-последовательности и т.п.)
        // сознательно игнорируются, а не попадают в значение
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

  // CR-01: хэшируем НЕ сырой ввод, а каноническую форму сканера. Значение,
  // которое сканер воспроизвести не может, отклоняется — молча записывать
  // мёртвую запись в доверенный список нельзя.
  const canon = canonicalizeAddValue(value);
  if (!canon.ok) {
    if (canon.reason === 'no_words') {
      console.error(
        `${TAG} FAIL — значение не содержит ни одной буквенно-цифровой последовательности. Сканер токенизирует строки по /[\\p{L}\\p{N}]+/u, поэтому такое значение не совпало бы никогда.`,
      );
    } else {
      console.error(
        `${TAG} FAIL — значение токенизируется в ${canon.words} слов(а), максимум ${NGRAM_MAX_WORDS}. Окно n-грамм сканера не превышает ${NGRAM_MAX_WORDS} слов, поэтому такой хэш при сканировании не был бы получен НИКОГДА — запись была бы мёртвой и не защищала бы ничего.`,
      );
      console.error(
        `${TAG} Что делать: добавь по отдельности каждую значимую подфразу длиной ≤${NGRAM_MAX_WORDS} слов(а) — ровно такие n-граммы строит сканер. Значение не записано.`,
      );
    }
    process.exit(1);
  }
  if (canon.canonical !== value) {
    // Значение НЕ печатается (D-16) — только факт нормализации и число слов.
    console.error(
      `${TAG} ПРИМЕЧАНИЕ — значение приведено к канонической форме сканера (${canon.words} слов(а); пунктуация и разделители отброшены, как их отбрасывает токенизатор). Хэш считается именно от неё.`,
    );
  }
  const hash = canon.hash;
  const newLine = `${hash} ${clsAnswer}`;

  const { headerLines, dataLines } = readExistingHashLines(args.hashesPath);

  // IN-02: одно и то же значение под двумя классами оставило бы в файле две
  // строки, а loadHashes() разрешил бы конфликт молча — по порядку строк.
  const existingClasses = new Set(
    [...dataLines].filter((l) => l.split(/\s+/)[0] === hash).map((l) => l.split(/\s+/)[1]),
  );
  if (existingClasses.has(clsAnswer)) {
    console.error(
      `${TAG} PASS — токен уже присутствует в ${args.hashesPath} с классом ${clsAnswer}; файл не изменён.`,
    );
    process.exit(0);
  }
  if (existingClasses.size > 0) {
    console.error(
      `${TAG} FAIL — этот же хэш уже записан с другим классом (${[...existingClasses].sort().join(', ')}). Две строки с разными классами для одного значения разрешались бы молча по порядку сортировки. Исправь класс существующей строки ${hash} вручную и повтори.`,
    );
    process.exit(1);
  }

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

  let collected;
  if (args.files.length > 0) {
    collected = collectExplicitTargets(args.files);
  } else if (args.staged) {
    collected = collectStagedTargets();
  } else {
    collected = collectHeadTargets();
  }

  const hashesAbs = path.resolve(process.cwd(), args.hashesPath);
  const hashesRel = toPosix(path.relative(REPO_ROOT, hashesAbs));

  // Auto-discovery scans (--staged / full HEAD) skip the gate's own
  // self-test fixtures; explicit-file invocations (args.files.length > 0 —
  // check-privacy.selftest.mjs) do NOT go through this filter, so the
  // fixtures still trip exactly as designed when named directly.
  const isInScope = (relPath) =>
    !isExcludedPath(relPath, hashesRel) &&
    (args.files.length > 0 || !isAutoScanExcludedFixture(relPath));

  const targets = collected.targets.filter((t) => isInScope(t.path));
  // Нечитаемые цели фильтруются ТЕМ ЖЕ предикатом: файл, который и так вне
  // области сканирования, не должен ронять гейт.
  const unreadable = collected.unreadable.filter((u) => isInScope(u.path));

  const violations = [
    ...scanAllowlist(targets),
    ...scanHashes(targets, hashMap),
    ...scanBinaries(targets),
  ];

  for (const v of violations) console.error(formatViolation(v));
  for (const u of unreadable) {
    console.error(`${TAG} ${u.path} — не прочитан (${u.reason})`);
  }

  if (violations.length > 0) {
    console.error(`${TAG} FAIL — ${violations.length} нарушений`);
  }
  if (unreadable.length > 0) {
    // CR-02: пропустить нечитаемую цель молча значит завершиться «PASS»,
    // не проверив её, — единственный fail-open путь в fail-closed гейте.
    console.error(
      `${TAG} FAIL — ${unreadable.length} целей не удалось прочитать: гейт не может подтвердить их чистоту, а молчаливый пропуск был бы fail-open (R7).`,
    );
    console.error(
      `${TAG} Восстановление: проверь права доступа и наличие файла (в режиме полного HEAD — \`git checkout -- <path>\` для удалённого из рабочего дерева), либо размер staged-блоба (лимит \`git show\` здесь — 64 МБ; файл такого размера в репозитории почти наверняка сам по себе проблема).`,
    );
  }
  if (violations.length > 0 || unreadable.length > 0) {
    process.exit(1);
  }
  console.error(`${TAG} PASS — 0 нарушений`);
  process.exit(0);
}

// Запускаем main() только при прямом вызове файла: check-privacy.selftest.mjs
// импортирует отсюда canonicalizeAddValue/normalize/sha256Hex, чтобы
// регрессия CR-01 проверялась сквозным round-trip'ом (--add требует TTY и
// напрямую из теста не вызывается). Хук и CI вызывают файл напрямую —
// поведение для них не меняется.
const invokedDirectly =
  process.argv[1] !== undefined &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (invokedDirectly) {
  await main();
}

export { canonicalizeAddValue, extractNgrams, normalize, sha256Hex, NGRAM_MAX_WORDS };
