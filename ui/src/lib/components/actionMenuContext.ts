/** Контекст ActionMenu для вложенных попапов (D-35): ActionMenu остаётся единственным
 *  владельцем закрытия меню, вложенные компоненты только вызывают `close`. */
export const ACTION_MENU_CONTEXT = Symbol('action-menu');

export interface ActionMenuContext {
  close: (returnFocus?: boolean) => void;
}
