#!/usr/bin/env node
// [check-release-isolation] Гейт изоляции выпусков Trackly и Trackly Link (D-11, D-20).
//
// Зачем: выпуск Link не должен ни запускаться тегами Trackly (v*.*.*), ни публиковать
// GitHub Release, ни задевать release.yml. Нарушение молча ломает релиз Trackly или
// публикует недоделанный Link, поэтому условие держится гейтом в ci-fast.
//
// Правила (комментарии `#` в workflow до проверок отбрасываются):
//   [I0] оба файла (release.yml, release-link.yml) существуют и непусты;
//   [I1] у release.yml on.push.tags содержит только `v*.*.*`;
//   [I2] у release-link.yml все теги начинаются с `link-v`, паттерна `v*.*.*` нет;
//   [I3] glob-сопоставление в духе GitHub (`*` не пересекает `/`, паттерн покрывает
//        имя тега целиком): `v1.4.0` — release.yml да, release-link.yml нет;
//        `link-v0.1.0` — release-link.yml да, release.yml нет;
//   [I4] в release-link.yml нет releaseId, releaseDraft, createRelease, `gh release`,
//        softprops, tagName, `contents: write`; есть `contents: read` и upload-artifact;
//   [I5] в release-link.yml нет `--features heic` и vcpkg (Link не линкует libheif);
//   [I6] в release.yml нет `link-app`, `link-v`, `crates/link-app`.
//
// Чего не видит: фактическую работу GitHub-фильтров (проверяется только текст и
// собственная модель glob) и сам запуск workflow — это dry-run плана 19.
//
// Self-test (`--selftest`): мутанты на строках реальных файлов; каждый обязан дать
// своё правило. Мутатор требует РОВНО одно вхождение якоря, иначе мутация могла бы
// сработать не там, где задумано.
//
// Zero-dependency: node:fs, node:path, node:url.
//
// Usage:
//   node scripts/check-release-isolation.mjs
//   node scripts/check-release-isolation.mjs --selftest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const TAG = '[check-release-isolation]';
const TRACKLY_REL = '.github/workflows/release.yml';
const LINK_REL = '.github/workflows/release-link.yml';

/** Убирает строки-комментарии и хвостовые `# ...` (в кавычках `#` у нас не встречается). */
function stripComments(text) {
  return text
    .split('\n')
    .map((l) => l.replace(/(^|\s)#.*$/, ''))
    .join('\n');
}

/** Теги из on.push.tags: блочный список или inline `[a, b]`. */
function parseTags(text) {
  const lines = stripComments(text).split('\n');
  const tags = [];
  for (let i = 0; i < lines.length; i++) {
    const inline = lines[i].match(/^\s*tags:\s*\[(.*)\]\s*$/);
    if (inline) {
      for (const t of inline[1].split(',')) {
        const v = t.trim().replace(/^['"]|['"]$/g, '');
        if (v) tags.push(v);
      }
      continue;
    }
    if (/^\s*tags:\s*$/.test(lines[i])) {
      for (let j = i + 1; j < lines.length; j++) {
        const m = lines[j].match(/^\s*-\s*['"]?([^'"\s]+)['"]?\s*$/);
        if (!m) {
          if (lines[j].trim() === '') continue;
          break;
        }
        tags.push(m[1]);
      }
    }
  }
  return tags;
}

/** Модель glob GitHub: `**` — что угодно, `*` — без `/`; совпадение по всему имени. */
function globMatches(pattern, name) {
  let re = '';
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === '*') {
      if (pattern[i + 1] === '*') {
        re += '.*';
        i++;
      } else re += '[^/]*';
    } else re += c.replace(/[.+^${}()|[\]\\?]/g, '\\$&');
  }
  return new RegExp(`^${re}$`).test(name);
}

const matchesAny = (patterns, name) => patterns.some((p) => globMatches(p, name));

/** Возвращает список нарушений вида `[Ix] текст`. */
function check(trackly, link) {
  const v = [];
  if (!trackly || !trackly.trim()) v.push(`[I0] ${TRACKLY_REL} отсутствует или пуст`);
  if (!link || !link.trim()) v.push(`[I0] ${LINK_REL} отсутствует или пуст`);
  if (v.length) return v;

  const tTags = parseTags(trackly);
  const lTags = parseTags(link);
  if (tTags.length !== 1 || tTags[0] !== 'v*.*.*')
    v.push(`[I1] release.yml: теги должны быть ровно ['v*.*.*'], найдено [${tTags.join(', ')}]`);

  if (lTags.length === 0) v.push('[I2] release-link.yml: нет push.tags');
  for (const t of lTags) {
    if (!t.startsWith('link-v')) v.push(`[I2] release-link.yml: тег «${t}» не начинается с link-v`);
  }
  if (lTags.includes('v*.*.*')) v.push('[I2] release-link.yml: есть паттерн v*.*.*');

  if (!matchesAny(tTags, 'v1.4.0')) v.push('[I3] v1.4.0 не совпал с release.yml');
  if (matchesAny(lTags, 'v1.4.0')) v.push('[I3] v1.4.0 совпал с release-link.yml');
  if (!matchesAny(lTags, 'link-v0.1.0')) v.push('[I3] link-v0.1.0 не совпал с release-link.yml');
  if (matchesAny(tTags, 'link-v0.1.0')) v.push('[I3] link-v0.1.0 совпал с release.yml');

  const l = stripComments(link);
  for (const bad of [
    'releaseId',
    'releaseDraft',
    'createRelease',
    'gh release',
    'softprops',
    'tagName',
    'contents: write',
  ]) {
    if (l.includes(bad)) v.push(`[I4] release-link.yml содержит «${bad}»`);
  }
  if (!l.includes('contents: read')) v.push('[I4] release-link.yml: нет «contents: read»');
  if (!l.includes('upload-artifact')) v.push('[I4] release-link.yml: нет upload-artifact');

  for (const bad of ['--features heic', 'vcpkg']) {
    if (l.includes(bad)) v.push(`[I5] release-link.yml содержит «${bad}»`);
  }

  const t = stripComments(trackly);
  for (const bad of ['link-app', 'link-v', 'crates/link-app']) {
    if (t.includes(bad)) v.push(`[I6] release.yml содержит «${bad}»`);
  }
  return v;
}

function readOrNull(rel) {
  try {
    return fs.readFileSync(path.join(REPO_ROOT, rel), 'utf8');
  } catch {
    return null;
  }
}

function selftest() {
  const trackly = readOrNull(TRACKLY_REL);
  const link = readOrNull(LINK_REL);
  let failed = 0;
  const expect = (name, cond) => {
    console.log(`${cond ? 'ok  ' : 'FAIL'} ${name}`);
    if (!cond) failed++;
  };
  const mutate = (text, anchor, repl) => {
    const n = text.split(anchor).length - 1;
    if (n !== 1) throw new Error(`якорь «${anchor}» встречается ${n} раз(а), нужно ровно 1`);
    return text.replace(anchor, repl);
  };
  const has = (list, rule) => list.some((x) => x.startsWith(rule));

  expect('реальные файлы: PASS', check(trackly, link).length === 0);
  expect(
    'пустой release-link.yml: [I0]',
    has(check(trackly, ''), '[I0]'),
  );
  expect(
    'мутант: release-link.yml с тегом v*.*.* -> [I2]',
    has(check(trackly, mutate(link, "'link-v*.*.*'", "'v*.*.*'")), '[I2]'),
  );
  expect(
    'мутант: release-link.yml с softprops -> [I4]',
    has(
      check(trackly, mutate(link, 'contents: read', 'contents: read\n  # x\n  uses: softprops/action-gh-release')),
      '[I4]',
    ),
  );
  expect(
    'мутант: release-link.yml с releaseId -> [I4]',
    has(check(trackly, mutate(link, 'contents: read', 'contents: read\n  releaseId: 1')), '[I4]'),
  );
  expect(
    'мутант: release.yml упоминает link-app -> [I6]',
    has(check(mutate(trackly, 'projectPath: crates/trackly-app', 'projectPath: crates/link-app'), link), '[I6]'),
  );
  expect(
    'мутант: release.yml с лишним тегом -> [I1]',
    has(check(mutate(trackly, "- 'v*.*.*'", "- 'v*.*.*'\n      - 'x*'"), link), '[I1]'),
  );
  expect(
    'мутант: release-link.yml с vcpkg -> [I5]',
    has(check(trackly, mutate(link, 'contents: read', 'contents: read\n  VCPKG: vcpkg')), '[I5]'),
  );
  expect(
    'комментарий с softprops не считается',
    check(trackly, mutate(link, 'contents: read', 'contents: read\n  # softprops')).length === 0,
  );
  if (failed) {
    console.error(`${TAG} selftest: провалено ${failed}`);
    process.exit(1);
  }
  console.log(`${TAG} selftest: PASS`);
}

if (process.argv.includes('--selftest')) {
  selftest();
} else {
  const violations = check(readOrNull(TRACKLY_REL), readOrNull(LINK_REL));
  if (violations.length) {
    for (const x of violations) console.error(`${TAG} ${x}`);
    process.exit(1);
  }
  console.log(`${TAG} PASS`);
}
