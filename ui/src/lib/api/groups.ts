// Единственная точка вызова команд раздела «Группы» (типы групп, их свойства и
// сами группы). Компоненты зовут эти методы, а не `apiCall` напрямую: имена
// методов `groups.move(`, `groups.addDevices(`, `groups.setParent(` — маркеры
// реестрового гейта INV-7 (`check-place-tree-invalidation.mjs`), поэтому их
// нельзя переименовывать, не обновив реестр.
//
// Обёртка — тонкая: ни логики, ни кэша. Источник истины — серверная
// валидация и проверка прав (планы 41-09 и 41-13); сокрытие кнопок в UI —
// только UX-слой. Типы запросов и ответов — из сгенерированного `bindings.ts`,
// имена аргументов — camelCase, как в HTTP-payload'ах роутов
// (`POST /api/v1/<команда>`) и в Tauri-вызовах.
import { apiCall } from './client';
import type {
  DeviceMembershipDto,
  GroupAddDevicesDto,
  GroupAddDevicesResultDto,
  GroupCardDto,
  GroupCompositionDto,
  GroupCreateDto,
  GroupDeleteResultDto,
  GroupDto,
  GroupMoveDto,
  GroupMoveResultDto,
  GroupRefDto,
  GroupRemoveDevicesDto,
  GroupSearchHitDto,
  GroupSetParentDto,
  GroupSetValuesDto,
  GroupTypeCreateDto,
  GroupTypeDto,
  GroupTypePropertyDto,
  GroupTypeUpdateDto,
  PropertyCreateDto,
  PropertyDeleteOutcomeDto,
  PropertyUpdateDto,
  UserOptionDto,
} from '../../bindings';

/** Типы групп и их свойства (10 команд; мутации — только администратор). */
export const groupTypes = {
  list: (includeArchived: boolean) =>
    apiCall<GroupTypeDto[]>('group_types_list', { includeArchived }),

  create: (dto: GroupTypeCreateDto) => apiCall<GroupTypeDto>('group_types_create', { dto }),

  update: (id: number, version: number, dto: GroupTypeUpdateDto) =>
    apiCall<GroupTypeDto>('group_types_update', { id, version, dto }),

  delete: (id: number) => apiCall<null>('group_types_delete', { id }),

  createProperty: (dto: PropertyCreateDto) =>
    apiCall<GroupTypePropertyDto>('group_type_properties_create', { dto }),

  updateProperty: (id: number, version: number, dto: PropertyUpdateDto) =>
    apiCall<GroupTypePropertyDto>('group_type_properties_update', { id, version, dto }),

  deleteProperty: (id: number) =>
    apiCall<PropertyDeleteOutcomeDto>('group_type_properties_delete', { id }),

  unarchiveProperty: (id: number) =>
    apiCall<GroupTypePropertyDto>('group_type_properties_unarchive', { id }),

  // Сервер двигает только `updated_at_utc`, не `version` свойства (41-04):
  // открытый редактор свойства не теряет CAS после перестановки.
  reorderProperties: (typeId: number, orderedIds: number[]) =>
    apiCall<GroupTypePropertyDto[]>('group_type_properties_reorder', { typeId, orderedIds }),

  emptyGroups: (propertyId: number) =>
    apiCall<GroupRefDto[]>('group_type_properties_empty_groups', { propertyId }),
};

/** Группы (15 команд; чтение — ReadGroups, мутации — MutateGroups). */
export const groups = {
  list: () => apiCall<GroupDto[]>('groups_list'),

  get: (id: number) => apiCall<GroupDto>('groups_get', { id }),

  card: (id: number) => apiCall<GroupCardDto>('groups_card', { id }),

  composition: (groupId: number) => apiCall<GroupCompositionDto>('groups_composition', { groupId }),

  search: (query: string, excludeGroupId: number | null) =>
    apiCall<GroupSearchHitDto[]>('groups_search', { query, excludeGroupId }),

  forDevices: (deviceIds: number[]) =>
    apiCall<DeviceMembershipDto[]>('groups_for_devices', { deviceIds }),

  userOptions: (query: string) => apiCall<UserOptionDto[]>('groups_user_options', { query }),

  create: (dto: GroupCreateDto) => apiCall<GroupDto>('groups_create', { dto }),

  update: (id: number, version: number, name: string) =>
    apiCall<GroupDto>('groups_update', { id, version, name }),

  delete: (id: number) => apiCall<GroupDeleteResultDto>('groups_delete', { id }),

  setParent: (dto: GroupSetParentDto) => apiCall<GroupMoveResultDto>('groups_set_parent', { dto }),

  addDevices: (dto: GroupAddDevicesDto) =>
    apiCall<GroupAddDevicesResultDto>('groups_add_devices', { dto }),

  removeDevices: (dto: GroupRemoveDevicesDto) => apiCall<number>('groups_remove_devices', { dto }),

  move: (dto: GroupMoveDto) => apiCall<GroupMoveResultDto>('groups_move', { dto }),

  // Сервер сверяет `version` ДО проверки значений: устаревшая версия всегда
  // даёт OptimisticLockMismatch. Нормализация ip/mac/number/text — серверная,
  // в JS не зеркалится.
  setValues: (dto: GroupSetValuesDto) => apiCall<GroupCardDto>('groups_set_values', { dto }),
};
