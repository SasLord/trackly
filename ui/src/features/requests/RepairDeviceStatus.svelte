<script lang="ts">
  // Бейдж статуса ремонтируемого устройства в карточке заявки. D-38: подпись
  // приходит из DTO состояния ремонта (RepairStateDto.deviceStatusName), справочник
  // статусов не запрашивается ни для одной роли; тон — из статической таблицы по коду.
  import Badge from '$lib/components/Badge.svelte';
  import { statusTone } from '$lib/stores/deviceStatuses.svelte';

  interface Props {
    code: string;
    name: string | null;
  }
  const { code, name }: Props = $props();

  // Код показываем только как запасной вариант, когда подписи нет.
  const label = $derived(name ?? code);
</script>

<Badge
  variant={statusTone(code).variant}
  appearance={statusTone(code).appearance}
  muted={statusTone(code).muted}>{label}</Badge
>
