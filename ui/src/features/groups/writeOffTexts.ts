// Фаза 41.8, план 22 (D-26): тексты списания участника группы через форму
// устройства. Живут в features/groups (словарь «группа», GRD-06).
export const WRITE_OFF_GROUP_FALLBACK_NAME = 'группа';

export function anchorWriteOffRefusal(groupName: string | null): string {
  return `Устройство — якорь группы «${groupName ?? ''}»; спишите его через группу.`;
}
