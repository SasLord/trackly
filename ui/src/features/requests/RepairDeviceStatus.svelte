<script lang="ts">
  // Бейдж статуса ремонтируемого устройства в карточке заявки. Подпись и тон —
  // только из справочника статусов устройств (по машинному коду).
  import Badge from '$lib/components/Badge.svelte';
  import {
    ensureLoaded as ensureDeviceStatuses,
    statusByCode,
    statusTone,
  } from '$lib/stores/deviceStatuses.svelte';

  interface Props {
    code: string;
  }
  const { code }: Props = $props();

  void ensureDeviceStatuses();
  const name = $derived(statusByCode(code)?.name ?? code);
</script>

<Badge
  variant={statusTone(code).variant}
  appearance={statusTone(code).appearance}
  muted={statusTone(code).muted}>{name}</Badge
>
