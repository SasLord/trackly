// Режимы списка «Устройства» (фаза 41.1, план 19; GRD-02, GRD-03, D-10, D-11, D-34).
//
// Чистый модуль: НИ ОДНОГО import, кроме типов, никакого DOM и рантайма Svelte.
// Его транспилирует и гоняет по фикстуре `scripts/fixtures/device-list-mode/cases.json`
// гейт `scripts/check-device-list-mode.mjs`.
//
// Серверный контракт (план 12): флаги hide_anchor_members / anchors_only /
// exclude_anchors / member_of_group_id в `DeviceFilter` ОПТ-ИН. Единственный
// вызывающий, кто их ставит, — DevicesPage, и только через `deviceListFlags`.
// Пикеры актов, автодополнение, массовое добавление и экспорт CSV флагов не ставят.

/** Контекст страницы, от которого зависит, прячутся ли члены якорных групп. */
export interface ListModeContext {
  /** Введён непустой поисковый запрос (после trim). */
  searchActive: boolean;
  /** Выбранная вкладка статуса; null — вкладка «Все». */
  statusFilter: number | null;
  /** Устройство, к которому перешли по `#/devices?id=…`; null — перехода нет. */
  focusId: number | null;
}

/** Какой запрос собирается: плоский список, свёртка одинаковых, якоря рядом со свёрткой, состав якоря. */
export type ListKind = 'flat' | 'grouped' | 'anchors' | 'members';

/** Флаги `DeviceFilter`, которые ставит страница. */
export interface ListFlags {
  hide_anchor_members: boolean;
  anchors_only: boolean;
  exclude_anchors: boolean;
}

/**
 * Скрывать ли членов якорных групп (D-10). Поиск сильнее опрятности списка:
 * деталь, найденная по инвентарному или серийному номеру, обязана быть видна;
 * на вкладке статуса и при переходе «Перейти к устройству» у детали должна быть
 * собственная строка, иначе её нечем подсветить.
 */
export function shouldHideAnchorMembers(ctx: ListModeContext): boolean {
  return !ctx.searchActive && ctx.statusFilter === null && ctx.focusId === null;
}

/**
 * Флаги запроса по виду списка.
 * - flat: скрытие по правилу; якоря остаются в списке.
 * - grouped: скрытие по правилу и якоря исключены из свёртки (D-11).
 * - anchors: только якоря (выводятся отдельно рядом со свёрткой).
 * - members: состав якоря — без каких-либо флагов, `member_of_group_id` задаёт вызывающий.
 */
export function deviceListFlags(kind: ListKind, ctx: ListModeContext): ListFlags {
  switch (kind) {
    case 'flat':
      return {
        hide_anchor_members: shouldHideAnchorMembers(ctx),
        anchors_only: false,
        exclude_anchors: false,
      };
    case 'grouped':
      return {
        hide_anchor_members: shouldHideAnchorMembers(ctx),
        anchors_only: false,
        exclude_anchors: true,
      };
    case 'anchors':
      return { hide_anchor_members: false, anchors_only: true, exclude_anchors: false };
    case 'members':
      return { hide_anchor_members: false, anchors_only: false, exclude_anchors: false };
  }
}

/** Вход формулы подписи футера плоского списка. */
export interface FooterInput {
  shown: number;
  total: number;
  /** K — сколько устройств скрыто (`hidden_member_count` сервера). */
  hidden: number;
  /** Скрытие сейчас включено. */
  hideActive: boolean;
}

/**
 * Подпись футера плоского списка: «Показано N из M», а когда скрытие включено и
 * что-то скрыто — «(+K в составе групп)» (D-34). Счётчики вкладок физические и
 * включают скрытых членов, хвост объясняет расхождение чисел.
 */
export function footerText(input: FooterInput): string {
  const base = `Показано ${input.shown} из ${input.total}`;
  if (input.hideActive && input.hidden > 0) return `${base} (+${input.hidden} в составе групп)`;
  return base;
}
