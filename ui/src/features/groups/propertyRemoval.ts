// Копирайт скрытия/удаления свойства типа группы (фаза 41, GAP-2, план 41-31).
// Единственный источник текстов пункта меню и модалки подтверждения.
//
// Правило ветвления — зеркало серверного `delete_property`
// (`group_type_service.rs`: `filled_group_count(id) > 0` — архив, иначе
// физическое удаление, D-13). Заполненное свойство скрывается и возвращается,
// пустое удаляется безвозвратно: интерфейс обязан сказать об этом ДО решения.
// Зеркало закреплено golden-фикстурой
// `ui/scripts/fixtures/property-removal/cases.json`: её читают и JS-гейт
// `ui/scripts/check-property-removal.mjs`, и Rust-тест
// `groups_property_removal_parity`. Модуль самодостаточен (ни одного import).

export interface PropertyRemovalInput {
  name: string;
  filledGroupCount: number;
  isRequired: boolean;
}

export interface PropertyRemovalCopy {
  kind: 'hide' | 'delete';
  menuLabel: string;
  modalTitle: string;
  body: string;
  confirmLabel: string;
  confirmVariant: 'primary' | 'destructive';
  errorToast: string;
}

export function propertyRemovalCopy(input: PropertyRemovalInput): PropertyRemovalCopy {
  if (input.filledGroupCount > 0) {
    const required = input.isRequired ? ' Признак «Обязательное» при скрытии снимается.' : '';
    return {
      kind: 'hide',
      menuLabel: 'Скрыть',
      modalTitle: 'Скрыть свойство',
      body: `Свойство «${input.name}» исчезнет из форм групп. Заполненные значения останутся в базе, свойство можно будет вернуть.${required}`,
      confirmLabel: 'Скрыть',
      confirmVariant: 'primary',
      errorToast: 'Не удалось скрыть свойство.',
    };
  }
  return {
    kind: 'delete',
    menuLabel: 'Удалить свойство',
    modalTitle: 'Удалить свойство',
    body: `Свойство «${input.name}» ещё нигде не заполнено, поэтому будет удалено безвозвратно: вернуть его через «Показать скрытые» нельзя, создать можно только заново.`,
    confirmLabel: 'Удалить безвозвратно',
    confirmVariant: 'destructive',
    errorToast: 'Не удалось удалить свойство.',
  };
}
