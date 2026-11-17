// Разбор текста сообщения на фрагменты «текст / ссылка» (D-08, UI-SPEC §7.2).
// Возвращает массив фрагментов, а не HTML: текст в разметку попадает только как текст.
// Чистый TS без импортов — Node импортирует файл напрямую (check-chat-linkify.mjs).

export type Segment = { type: 'text' | 'link'; value: string };

const SCHEME = /^https?:\/\//i;
const TRAILING = '.,;:!?)»"';

function isSpace(ch: string): boolean {
  return /\s/.test(ch);
}

export function linkify(text: string): Segment[] {
  const out: Segment[] = [];
  let buf = '';
  let i = 0;

  const flush = () => {
    if (buf) out.push({ type: 'text', value: buf });
    buf = '';
  };

  while (i < text.length) {
    const rest = text.slice(i);
    const m = SCHEME.exec(rest);
    if (!m) {
      buf += text[i];
      i += 1;
      continue;
    }
    let end = i;
    while (end < text.length && !isSpace(text[end])) end += 1;
    // Завершающая пунктуация остаётся текстом.
    let linkEnd = end;
    while (linkEnd > i + m[0].length && TRAILING.includes(text[linkEnd - 1])) linkEnd -= 1;
    // После схемы нужен хотя бы один символ, не являющийся пунктуацией.
    const body = text.slice(i + m[0].length, linkEnd);
    if (body.length === 0 || TRAILING.includes(body[0])) {
      // Не ссылка: съедаем схему как текст и идём дальше.
      buf += text.slice(i, i + m[0].length);
      i += m[0].length;
      continue;
    }
    flush();
    out.push({ type: 'link', value: text.slice(i, linkEnd) });
    i = linkEnd;
  }
  flush();
  return out;
}
