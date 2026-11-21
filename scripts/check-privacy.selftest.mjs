#!/usr/bin/env node
// [check-privacy-selftest] Фикстур-driven регрессионный self-test для
// scripts/check-privacy.mjs (PRIV-02).
//
// Почему он существует: сам гейт приватности не должен зависеть от
// production-файла хэшей (scripts/privacy-tokens.sha256 ещё не существует —
// его создаёт план 37-04) или от реального содержимого HEAD, чтобы быть
// проверяемым уже сейчас, только на вымышленных данных (C-01). Каждая
// проверка ниже вызывает check-privacy.mjs как отдельный процесс через
// execFileSync с явными fixture-путями и сверяет только exit-код и
// отсутствие утечки значения токена в выводе (D-16) — никогда не сам вывод
// целиком.
//
// Покрывает: R6 (n-грамм хэши), R7 (fail-closed --hashes), R8 (контроль
// бинарных расширений), C-01 (никогда не production-файл), C-02 (регрессия
// режима 1), D-16 (нет утечки значения при срабатывании), а также
// регрессии код-ревью фазы 37: CR-01 (--add и сканер обязаны считать один
// и тот же хэш), CR-02 (нечитаемая цель — отказ, а не молчаливый пропуск),
// WR-01/WR-02/WR-03 (режим 1 видит структурированные конфиги, регистр
// ключа и значение без кавычек), WR-04 (состав fixture-каталога, который
// авто-сканирование не проверяет, зафиксирован явным списком), а также
// quick 261121-hjw (todo 2026-09-26): межстрочный проход (блок 11),
// cp1251/UTF-16 (12), текст PNG-чанков (13), разложение длинных значений
// для --add (14), охват каталогов (15), расширенный класс D (16).
//
// Zero-dependency: только node:fs/os/path/url/child_process/zlib.
//
// Usage:
//   node scripts/check-privacy.selftest.mjs

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
// Импорт из самого гейта: файл выполняет main() только при прямом вызове,
// поэтому импорт безопасен. Нужен, чтобы проверить CR-01 сквозным
// round-trip'ом — --add требует TTY и напрямую из теста не вызывается.
import zlib from 'node:zlib';
import {
  allNgrams,
  canonicalizeAddValue,
  decodeCp1251,
  decomposeLongValue,
  extractPngText,
  isAutoScanExcludedFixture,
  isExcludedPath,
  NGRAM_MAX_WORDS,
  normalize,
  scanForbiddenPaths,
  sha256Hex,
} from './check-privacy.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const GATE = path.join(REPO_ROOT, 'scripts', 'check-privacy.mjs');
const FIXTURES = path.join(REPO_ROOT, 'scripts', 'fixtures', 'privacy');
const TAG = '[check-privacy-selftest]';

const TOKENS_HASHES = path.join(FIXTURES, 'tokens.fixture.sha256');
const EMPTY_HASHES = path.join(FIXTURES, 'empty.sha256');
const NONEXISTENT_HASHES = path.join(FIXTURES, 'this-file-does-not-exist.sha256');

let failures = 0;

/** Запускает check-privacy.mjs как дочерний процесс; никогда не бросает —
 * возвращает {code, output} с объединённым stdout+stderr для инспекции. */
function runGate(args) {
  try {
    const output = execFileSync('node', [GATE, ...args], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { code: 0, output };
  } catch (err) {
    const code = typeof err.status === 'number' ? err.status : 1;
    const output = `${err.stdout || ''}${err.stderr || ''}`;
    return { code, output };
  }
}

function assertExitCode(name, args, expectedCode) {
  const { code } = runGate(args);
  if (code !== expectedCode) {
    console.error(`${TAG} FAIL — ${name}: ожидался exit ${expectedCode}, получен ${code}`);
    failures++;
    return null;
  }
  console.error(`${TAG} ok — ${name}`);
  return code;
}

function assertTrue(name, condition) {
  if (!condition) {
    console.error(`${TAG} FAIL — ${name}`);
    failures++;
    return;
  }
  console.error(`${TAG} ok — ${name}`);
}

/** Одноразовый каталог вне репозитория: файлы-регрессии CR-01/CR-02/WR-*
 * намеренно «грязные», их нельзя оставлять в дереве проекта. */
function withTempDir(fn) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-privacy-selftest-'));
  try {
    return fn(dir);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

function assertNoLeak(name, args, secret) {
  const { output } = runGate(args);
  if (output.includes(secret)) {
    console.error(`${TAG} FAIL — ${name}: вывод гейта содержит значение токена (утечка D-16)`);
    failures++;
    return;
  }
  console.error(`${TAG} ok — ${name} (значение токена не встречается в выводе)`);
}

// ---------------------------------------------------------------------------
// 1. with-marker.md — вымышленный маркер смежными словами → exit 1, без
//    утечки значения в выводе (D-05, D-16).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'with-marker.md');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('with-marker.md: смежный маркер найден', args, 1);
  assertNoLeak('with-marker.md: значение маркера не утекает в вывод', args, 'Пчёлкин Артём');
}

// ---------------------------------------------------------------------------
// 2. without-marker.md — те же слова, но не смежные → exit 0.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('without-marker.md: несмежные слова проходят гейт', args, 0);
}

// ---------------------------------------------------------------------------
// 3. Несуществующий --hashes путь → exit 1 (R7, fail-closed).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', NONEXISTENT_HASHES, target];
  assertExitCode('несуществующий --hashes падает закрыто (R7)', args, 1);
}

// ---------------------------------------------------------------------------
// 4. Пустой --hashes (empty.sha256) → exit 1 (R7, fail-closed).
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'without-marker.md');
  const args = ['--hashes', EMPTY_HASHES, target];
  assertExitCode('пустой --hashes падает закрыто (R7)', args, 1);
}

// ---------------------------------------------------------------------------
// 5. Регрессия режима 1 (allowlist) — C-02.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'allowlist-regression.rs.txt');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('allowlist-regression.rs.txt: неразрешённый литерал ловится (C-02)', args, 1);
  assertNoLeak('allowlist-regression.rs.txt: значение литерала не утекает в вывод', args, '9998887770');
}

// ---------------------------------------------------------------------------
// 6. Контроль бинарных расширений — R8.
// ---------------------------------------------------------------------------
{
  const target = path.join(FIXTURES, 'binary-regression.docx');
  const args = ['--hashes', TOKENS_HASHES, target];
  assertExitCode('binary-regression.docx вне allowlist падает (R8)', args, 1);
}
{
  const iconsDir = path.join(REPO_ROOT, 'crates/trackly-app/icons');
  const iconFiles = fs.readdirSync(iconsDir).map((f) => path.join(iconsDir, f));
  const logoFixture = path.join(REPO_ROOT, 'crates/trackly-app/tests/fixtures/logo_test.png');
  const args = ['--hashes', TOKENS_HASHES, ...iconFiles, logoFixture];
  assertExitCode('icons/ + logo_test.png в BINARY_ALLOWLIST проходят (R8)', args, 0);
}

// ---------------------------------------------------------------------------
// 6b. Вложения (D-23, ATT-03): запрет по пути, не по содержимому.
// ---------------------------------------------------------------------------
{
  const hex64 = 'a1'.repeat(32); // вымышленный литерал
  const flagged = (p) => scanForbiddenPaths([{ path: p }]).length === 1;
  assertTrue('attachments/ab/<64hex> в корне — нарушение', flagged(`attachments/ab/${hex64}`));
  assertTrue('data/attachments/ab/<64hex> (вложенный шард) — нарушение', flagged(`data/attachments/ab/${hex64}`));
  assertTrue('attachments/readme.txt (корень, любой файл) — нарушение', flagged('attachments/readme.txt'));
  assertTrue(
    'контроль: ui/src/features/attachments/AttachmentsBlock.svelte проходит',
    !flagged('ui/src/features/attachments/AttachmentsBlock.svelte'),
  );
  assertTrue(
    'контроль: .planning/phases/41.3-attachments-storage/41.3-PLAN.md проходит',
    !flagged('.planning/phases/41.3-attachments-storage/41.3-PLAN.md'),
  );
  // Сквозной прогон: реальный файл в attachments/ в корне репозитория.
  const dir = path.join(REPO_ROOT, 'attachments');
  const existed = fs.existsSync(dir);
  const file = path.join(dir, 'selftest-probe.txt');
  try {
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(file, 'fictional probe\n');
    const { code, output } = runGate(['--hashes', TOKENS_HASHES, file]);
    assertTrue('сквозной: файл в attachments/ роняет гейт', code === 1);
    assertTrue('сквозной: вывод называет путь, не содержимое', !output.includes('fictional probe'));
  } finally {
    fs.rmSync(file, { force: true });
    if (!existed) fs.rmSync(dir, { recursive: true, force: true });
  }
}

// 

// ---------------------------------------------------------------------------
// 7. CR-01 — --add обязан хэшировать ту же каноническую форму, что строит
//    сканер. Значения только вымышленные (C-01).
// ---------------------------------------------------------------------------
{
  // Дефис внутри значения: токенизатор сканера его отбрасывает, поэтому
  // канонической формой становится «Иванов Петров» — и именно её хэш
  // обязан записывать --add. До исправления хэшировался сырой ввод, и
  // запись не срабатывала никогда (мёртвая строка в доверенном списке).
  const raw = 'Иванов-Петров';
  const canon = canonicalizeAddValue(raw);
  assertTrue('CR-01: значение с дефисом принимается и канонизируется', canon.ok);
  assertTrue(
    'CR-01: каноническая форма — слова через один пробел ASCII',
    canon.ok && canon.canonical === 'Иванов Петров',
  );
  assertTrue(
    'CR-01: хэш считается от канонической формы, а не от сырого ввода',
    canon.ok && canon.hash !== sha256Hex(normalize(raw)),
  );

  withTempDir((dir) => {
    const doc = path.join(dir, 'cr01-doc.md');
    fs.writeFileSync(doc, `Акт подписан представителем ${raw} по доверенности.\n`, 'utf8');

    const fixedHashes = path.join(dir, 'cr01-canonical.sha256');
    fs.writeFileSync(fixedHashes, `# scratch\n${canon.hash} B\n`, 'utf8');
    assertExitCode(
      'CR-01: хэш из --add срабатывает на дефисной форме в тексте',
      ['--hashes', fixedHashes, doc],
      1,
    );
    assertNoLeak('CR-01: значение не утекает в вывод', ['--hashes', fixedHashes, doc], raw);

    // Контроль «до исправления»: хэш сырого ввода не воспроизводим сканером.
    const buggyHashes = path.join(dir, 'cr01-raw.sha256');
    fs.writeFileSync(buggyHashes, `# scratch\n${sha256Hex(normalize(raw))} B\n`, 'utf8');
    assertExitCode(
      'CR-01: хэш сырого ввода (старое поведение) не срабатывает — потому он и запрещён',
      ['--hashes', buggyHashes, doc],
      0,
    );
  });

  // Непредставимые значения обязаны отклоняться, а не записываться молча.
  const tooLong = canonicalizeAddValue('улица Вымышленная дом двенадцать строение три');
  assertTrue(
    'CR-01: значение длиннее окна n-грамм отклоняется',
    tooLong.ok === false && tooLong.reason === 'too_many_words',
  );
  const noWords = canonicalizeAddValue('--- ...');
  assertTrue(
    'CR-01: значение без буквенно-цифровых слов отклоняется',
    noWords.ok === false && noWords.reason === 'no_words',
  );
}

// ---------------------------------------------------------------------------
// 8. CR-02 — нечитаемая цель обязана давать exit 1, а не «PASS — 0 нарушений».
// ---------------------------------------------------------------------------
{
  withTempDir((dir) => {
    const missing = path.join(dir, 'this-target-does-not-exist.md');
    assertExitCode(
      'CR-02: нечитаемая цель падает закрыто, а не пропускается молча',
      ['--hashes', TOKENS_HASHES, missing],
      1,
    );

    const clean = path.join(dir, 'clean.md');
    fs.writeFileSync(clean, 'Совершенно обычный текст без маркеров.\n', 'utf8');
    assertExitCode(
      'CR-02: чистая цель рядом с нечитаемой всё равно роняет прогон',
      ['--hashes', TOKENS_HASHES, clean, missing],
      1,
    );
    assertExitCode(
      'CR-02: одна чистая цель по-прежнему проходит (нет ложного отказа)',
      ['--hashes', TOKENS_HASHES, clean],
      0,
    );
  });
}

// ---------------------------------------------------------------------------
// 9. WR-01/WR-02/WR-03 — режим 1 видит структурированные конфиги, регистр
//    ключа и значение без кавычек. Цифры ниже вымышлены и отсутствуют в
//    ALLOWED (C-01).
// ---------------------------------------------------------------------------
{
  withTempDir((dir) => {
    const json = path.join(dir, 'org.json');
    fs.writeFileSync(json, '{ "inn": "9998887774", "kpp": "999888777" }\n', 'utf8');
    assertExitCode(
      'WR-01: неразрешённый литерал в .json ловится режимом 1',
      ['--hashes', TOKENS_HASHES, json],
      1,
    );

    const upper = path.join(dir, 'upper.rs');
    fs.writeFileSync(upper, 'struct FakeOrg {\n    INN: "9998887773",\n}\n', 'utf8');
    assertExitCode(
      'WR-02: ключ в верхнем регистре ловится режимом 1',
      ['--hashes', TOKENS_HASHES, upper],
      1,
    );

    const bare = path.join(dir, 'bare.rs');
    fs.writeFileSync(bare, 'struct FakeOrg {\n    inn: 9998887772,\n}\n', 'utf8');
    assertExitCode(
      'WR-03: значение без кавычек ловится режимом 1',
      ['--hashes', TOKENS_HASHES, bare],
      1,
    );

    // Разрешённое значение не должно ловиться ни в одной из новых форм —
    // расширялось только обнаружение, ALLOWED не менялся.
    const allowedShapes = path.join(dir, 'allowed.rs');
    fs.writeFileSync(
      allowedShapes,
      'struct Demo {\n    INN: "7700000000",\n    inn: 7700000000,\n    kpp: 7_700_000_00,\n}\n',
      'utf8',
    );
    assertExitCode(
      'WR-01/02/03: разрешённые значения в новых формах не дают ложных срабатываний',
      ['--hashes', TOKENS_HASHES, allowedShapes],
      0,
    );
  });
}

// ---------------------------------------------------------------------------
// 10. WR-04 — scripts/fixtures/privacy/ исключён из авто-сканирования
//     (AUTO_SCAN_EXCLUDED_PREFIXES), поэтому реальные данные, попавшие сюда,
//     не поймал бы ни хук, ни CI. Состав каталога зафиксирован явным
//     списком: любое добавление файла обязано пройти через правку этого
//     теста, то есть через code review.
// ---------------------------------------------------------------------------
{
  const EXPECTED_FIXTURES = [
    'README.md',
    'allowlist-regression.rs.txt',
    'binary-regression.docx',
    'empty.sha256',
    'tokens.fixture.sha256',
    'with-marker.md',
    'without-marker.md',
    'wrapped-marker.md',
    'paragraph-break.md',
  ];
  const actual = fs.readdirSync(FIXTURES).sort();
  const expected = [...EXPECTED_FIXTURES].sort();
  const same =
    actual.length === expected.length && actual.every((f, i) => f === expected[i]);
  if (!same) {
    const added = actual.filter((f) => !expected.includes(f));
    const removed = expected.filter((f) => !actual.includes(f));
    console.error(
      `${TAG} FAIL — состав scripts/fixtures/privacy/ изменился (добавлено: ${added.join(', ') || '—'}; удалено: ${removed.join(', ') || '—'}). Этот каталог исключён из авто-сканирования гейта: убедись, что новый файл полностью вымышлен (C-01), и обнови EXPECTED_FIXTURES.`,
    );
    failures++;
  } else {
    console.error(`${TAG} ok — состав scripts/fixtures/privacy/ совпадает с ожидаемым (WR-04)`);
  }
}

// ---------------------------------------------------------------------------
// 11. Межстрочный проход режима 2: значение, разорванное переносом строки
//     внутри абзаца (в т.ч. за префиксами `//`), ловится; через пустую
//     строку — нет. Без утечки значения в вывод (D-16).
// ---------------------------------------------------------------------------
{
  const wrapped = ['--hashes', TOKENS_HASHES, path.join(FIXTURES, 'wrapped-marker.md')];
  assertExitCode('wrapped-marker.md: маркер, разорванный переносом, найден', wrapped, 1);
  const out = runGate(wrapped).output;
  assertTrue(
    'wrapped-marker.md: найдены оба случая (проза и комментарий //)',
    (out.match(/маркер класса B/g) || []).length === 2,
  );
  for (const word of ['Пчёлкин', 'Артём', 'пчелкин', 'артем']) {
    assertNoLeak(`wrapped-marker.md: слово не утекает в вывод (${word.length} букв)`, wrapped, word);
  }
  assertExitCode(
    'paragraph-break.md: слова через пустую строку проходят гейт',
    ['--hashes', TOKENS_HASHES, path.join(FIXTURES, 'paragraph-break.md')],
    0,
  );
}

// ---------------------------------------------------------------------------
// 12. Кодировки: cp1251 и UTF-16 (с BOM) больше не слепая зона режима 2.
// ---------------------------------------------------------------------------
{
  // Таблица cp1251 гейта сверяется с эталоном Node, если ICU его даёт.
  let ref = null;
  try {
    ref = new TextDecoder('windows-1251');
  } catch {
    ref = null;
  }
  if (ref) {
    const all = Buffer.alloc(256);
    for (let i = 0; i < 256; i++) all[i] = i;
    assertTrue('cp1251: таблица гейта совпадает с TextDecoder(windows-1251) на 256 байтах', decodeCp1251(all) === ref.decode(all));
  } else {
    console.error(`${TAG} ok — cp1251: эталон TextDecoder недоступен (нет ICU), сверка пропущена`);
  }
  const marker = 'Заметка: исполнитель Пчёлкин Артём, отдел снабжения.\n';
  const cp1251Bytes = Buffer.from(
    [...marker].map((ch) => {
      const cp = ch.codePointAt(0);
      if (cp < 0x80) return cp;
      if (cp >= 0x0410 && cp <= 0x044f) return cp - 0x0410 + 0xc0;
      if (cp === 0x0401) return 0xa8;
      if (cp === 0x0451) return 0xb8;
      throw new Error(`cp1251 fixture: нет байта для U+${cp.toString(16)}`);
    }),
  );
  withTempDir((dir) => {
    const cp = path.join(dir, 'cp1251.csv');
    fs.writeFileSync(cp, cp1251Bytes);
    assertExitCode('cp1251: маркер в windows-1251 найден', ['--hashes', TOKENS_HASHES, cp], 1);
    const le = path.join(dir, 'utf16le.txt');
    fs.writeFileSync(le, Buffer.concat([Buffer.from([0xff, 0xfe]), Buffer.from(marker, 'utf16le')]));
    assertExitCode('UTF-16LE с BOM: маркер найден', ['--hashes', TOKENS_HASHES, le], 1);
    const beBody = Buffer.from(marker, 'utf16le');
    beBody.swap16();
    const be = path.join(dir, 'utf16be.txt');
    fs.writeFileSync(be, Buffer.concat([Buffer.from([0xfe, 0xff]), beBody]));
    assertExitCode('UTF-16BE с BOM: маркер найден', ['--hashes', TOKENS_HASHES, be], 1);
  });
}

// ---------------------------------------------------------------------------
// 13. Текст PNG-чанков (tEXt / zTXt / iTXt) — метаданные скриншотов.
// ---------------------------------------------------------------------------
{
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc32 = (buf) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type, 'latin1'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(td));
    return Buffer.concat([len, td, crc]);
  };
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = chunk('IHDR', Buffer.from([0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0]));
  const iend = chunk('IEND', Buffer.alloc(0));
  const text = 'Пчёлкин Артём';
  const pngWith = (c) => Buffer.concat([sig, ihdr, c, iend]);
  const kw = (k) => Buffer.from(`${k}\0`, 'latin1');
  const variants = {
    tEXt: pngWith(chunk('tEXt', Buffer.concat([kw('Author'), Buffer.from(text, 'utf8')]))),
    zTXt: pngWith(
      chunk('zTXt', Buffer.concat([kw('Comment'), Buffer.from([0]), zlib.deflateSync(Buffer.from(text, 'utf8'))])),
    ),
    iTXt: pngWith(
      chunk(
        'iTXt',
        Buffer.concat([kw('Description'), Buffer.from([1, 0]), Buffer.from('ru\0\0', 'latin1'), zlib.deflateSync(Buffer.from(text, 'utf8'))]),
      ),
    ),
  };
  const markerHash = sha256Hex(normalize('Пчёлкин Артём'));
  for (const [type, png] of Object.entries(variants)) {
    const extracted = extractPngText(png);
    assertTrue(
      `PNG ${type}: извлечённый текст даёт n-грамму маркера`,
      allNgrams(extracted).some(({ ngram }) => sha256Hex(normalize(ngram)) === markerHash),
    );
  }
  assertTrue('PNG: битый буфер не роняет извлечение', extractPngText(Buffer.from('not a png')) === '');
  assertTrue(
    'PNG: обрезанный чанк не роняет извлечение',
    typeof extractPngText(variants.zTXt.subarray(0, variants.zTXt.length - 20)) === 'string',
  );
  withTempDir((dir) => {
    const p = path.join(dir, 'shot.png');
    fs.writeFileSync(p, variants.zTXt);
    const { code, output } = runGate(['--hashes', TOKENS_HASHES, p]);
    assertTrue(
      'PNG вне allowlist: и класс D, и маркер класса B из zTXt',
      code === 1 && output.includes('маркер класса D') && output.includes('маркер класса B'),
    );
    assertTrue('PNG: значение не утекает в вывод', !output.includes('Пчёлкин') && !output.includes('Артём'));
  });
}

// ---------------------------------------------------------------------------
// 14. --add: значение длиннее NGRAM_MAX_WORDS раскладывается на смежные окна
//     (п.2 todo 2026-09-26). Только вымышленное значение (C-01).
// ---------------------------------------------------------------------------
{
  const value = 'Пчёлкин Артём Ильич из отдела снабжения';
  const d = decomposeLongValue(value);
  const words = [...value.matchAll(/[\p{L}\p{N}]+/gu)].map((m) => m[0]);
  assertTrue('decompose: значение разложено', d.ok === true && d.words === words.length);
  assertTrue(
    `decompose: ${words.length - NGRAM_MAX_WORDS + 1} смежных окон по ${NGRAM_MAX_WORDS} слова`,
    d.ok && d.phrases.length === words.length - NGRAM_MAX_WORDS + 1 && d.phrases.length === 4,
  );
  const pairsCovered = words
    .slice(0, -1)
    .every((w, i) => d.phrases.some((ph) => ph.canonical.includes(`${w} ${words[i + 1]}`)));
  assertTrue('decompose: каждая пара соседних слов покрыта окном', pairsCovered);
  assertTrue(
    'decompose: хэш каждой фразы равен хэшу n-граммы сканера',
    d.ok &&
      d.phrases.every((ph) =>
        allNgrams(value).some(({ ngram }) => sha256Hex(normalize(ngram)) === ph.hash),
      ),
  );
  assertTrue(
    'decompose: короткое значение — одна фраза, как canonicalizeAddValue',
    (() => {
      const short = decomposeLongValue('Пчёлкин-Артём');
      const c = canonicalizeAddValue('Пчёлкин-Артём');
      return short.ok && short.phrases.length === 1 && short.phrases[0].hash === c.hash;
    })(),
  );
  assertTrue('decompose: значение без слов отклоняется', decomposeLongValue('— … —').ok === false);
  withTempDir((dir) => {
    const hashes = path.join(dir, 'decomposed.sha256');
    fs.writeFileSync(hashes, `${d.phrases.map((ph) => `${ph.hash} B`).join('\n')}\n`, 'utf8');
    let caughtAll = true;
    for (let cut = 1; cut < words.length; cut++) {
      const f = path.join(dir, `cut-${cut}.md`);
      fs.writeFileSync(
        f,
        `Запись: ${words.slice(0, cut).join(' ')}\n${words.slice(cut).join(' ')} — конец.\n`,
        'utf8',
      );
      if (runGate(['--hashes', hashes, f]).code !== 1) caughtAll = false;
    }
    assertTrue('decompose: разрыв переносом в любой точке ловится хэшами разложения', caughtAll);
  });
}

// ---------------------------------------------------------------------------
// 15. Охват каталогов: ни один предикат исключения не скрывает планы,
//     шаблоны и тестовые фикстуры (п.3 todo 2026-09-26).
// ---------------------------------------------------------------------------
{
  const mustScan = [
    '.planning/todos/pending/x.md',
    '.planning/quick/x/y.md',
    '.planning/debug/x.md',
    '.planning/threads/x.md',
    '.planning/milestones/x.md',
    '.planning/phases/x/y.md',
    'templates/x.html',
    'crates/trackly-app/tests/fixtures/devices/x.csv',
    'ui/scripts/fixtures/x/y.txt',
  ];
  const leaked = mustScan.filter(
    (p) => isExcludedPath(p, 'scripts/privacy-tokens.sha256') || isAutoScanExcludedFixture(p),
  );
  assertTrue(`охват: каталоги не исключены (${leaked.join(', ') || 'все в охвате'})`, leaked.length === 0);
  const mustSkip = [
    ['node_modules/x/y.js', 'excluded'],
    ['target/debug/x', 'excluded'],
    ['Cargo.lock', 'excluded'],
    ['scripts/privacy-tokens.sha256', 'excluded'],
    ['scripts/fixtures/privacy/x.md', 'fixture'],
  ];
  const wrong = mustSkip.filter(([p, kind]) =>
    kind === 'excluded'
      ? !isExcludedPath(p, 'scripts/privacy-tokens.sha256')
      : !isAutoScanExcludedFixture(p) || isExcludedPath(p, 'scripts/privacy-tokens.sha256'),
  );
  assertTrue(
    `охват: исключены ровно node_modules/, target/, Cargo.lock, файл хэшей и фикстуры гейта (${wrong.map(([p]) => p).join(', ') || 'ок'})`,
    wrong.length === 0,
  );
}

// ---------------------------------------------------------------------------
// 16. Класс D расширен: офисные/архивные/графические/БД-форматы.
// ---------------------------------------------------------------------------
withTempDir((dir) => {
  const missed = [];
  for (const ext of ['.doc', '.xls', '.odt', '.rtf', '.zip', '.7z', '.gif', '.webp', '.heic', '.sqlite', '.db-wal', '.eml']) {
    const f = path.join(dir, `sample${ext}`);
    fs.writeFileSync(f, '');
    const { code, output } = runGate(['--hashes', TOKENS_HASHES, f]);
    if (code !== 1 || !output.includes('маркер класса D')) missed.push(ext);
  }
  assertTrue(`класс D: новые расширения ловятся (${missed.join(', ') || 'все'})`, missed.length === 0);
});

// ---------------------------------------------------------------------------

if (failures > 0) {
  console.error(`${TAG} FAIL — ${failures} нарушений`);
  process.exit(1);
}
console.error(`${TAG} PASS — 0 нарушений`);
process.exit(0);
