<script lang="ts">
  // Фаза 41.8, план 09 (D-21, R-12, UI-SPEC §11.3): подтверждение списания якоря
  // группы со списком состава. Без загруженного состава «Списать» недоступна:
  // подтверждение вслепую — не подтверждение.
  //
  // Modal может стоять внутри <td>: тело явно задаёт white-space: normal и
  // text-align: left. Монтируется родителем через {#if}.
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { groups } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { notifyPlaceContentChanged } from '$lib/stores/placeContentEvents.svelte';
  import type { GroupDto, GroupCompositionDto } from '../../bindings';

  interface Props {
    group: GroupDto;
    onDone: () => void;
    onClose: () => void;
  }

  const { group, onDone, onClose }: Props = $props();

  let comp = $state<GroupCompositionDto | null>(null);
  let loading = $state(true);
  let loadErr = $state(false);
  let saving = $state(false);
  let serverErr = $state<string | null>(null);

  const isTeardown = $derived(group.type_behavior === 'teardown');
  const anchorName = $derived(group.anchor_device_name ?? group.name);

  interface Row {
    key: string;
    name: string;
    inv: string | null;
    gone: boolean;
  }

  const rows = $derived.by<Row[]>(() => {
    if (comp === null) return [];
    // Якорь списывается, а не освобождается: в список состава не входит (D-21).
    const members = isTeardown
      ? comp.devices
      : comp.devices.filter((d) => d.device_id !== group.anchor_device_id);
    const live = members.map(
      (d): Row => ({
        key: 'd' + d.device_id,
        name: d.name,
        inv: d.inventory_number,
        gone: false,
      }),
    );
    if (!isTeardown) {
      const children = comp.child_groups.map(
        (c): Row => ({
          key: 'c' + c.id,
          name: `группа «${c.name}» станет самостоятельной`,
          inv: null,
          gone: false,
        }),
      );
      return [...live, ...children];
    }
    const ghosts = comp.ghosts.map(
      (g): Row => ({
        key: 'g' + g.device_id,
        name: g.name,
        inv: g.inventory_number,
        gone: true,
      }),
    );
    return [...live, ...ghosts];
  });

  // Двоеточие, если ниже будет список (или он ещё грузится); иначе точка.
  const tail = $derived(loading || loadErr || rows.length > 0 ? ':' : '.');

  async function load() {
    loading = true;
    loadErr = false;
    try {
      comp = await groups.composition(group.id);
    } catch {
      loadErr = true;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  async function confirm() {
    if (saving || comp === null) return;
    saving = true;
    serverErr = null;
    try {
      const res = await groups.writeOffGroup({ id: group.id, version: group.version });
      notifyPlaceContentChanged(res.changed_place_ids);
      pushToast(
        'success',
        isTeardown
          ? `Донор «${anchorName}» списан, запись разбора «${group.name}» сохранена`
          : `Устройство «${anchorName}» списано, группа «${group.name}» расформирована`,
      );
      onDone();
    } catch (e) {
      const err = e as { message?: string } | undefined;
      serverErr = err?.message ?? 'Не удалось списать. Проверьте подключение и повторите.';
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  open={true}
  title={`Списать «${anchorName}»?`}
  size="md"
  {onClose}
  initialFocus=".modal-footer .btn-secondary"
>
  <div class="writeoff">
    <p class="body">
      {#if isTeardown}
        Донор «{anchorName}» получит статус «Списано». Запись разбора «{group.name}» сохранится
        вместе с деталями{tail}
      {:else}
        Устройство «{anchorName}» — якорь группы «{group.name}» — получит статус «Списано». Группа
        будет расформирована, её состав освободится; статус и место этих устройств не изменятся{tail}
      {/if}
    </p>

    {#if loading}
      <div class="state">
        <Spinner size="sm" />
        <span>Загрузка…</span>
      </div>
    {:else if loadErr}
      <div class="state state-error">
        <span class="err-text">Не удалось загрузить состав группы. Нажмите «Повторить».</span>
        <Button variant="secondary" size="sm" onclick={load}>Повторить</Button>
      </div>
    {:else if rows.length === 0}
      <p class="empty">{isTeardown ? 'Деталей в разборе нет.' : 'Состав группы пуст.'}</p>
    {:else}
      <ul class="members">
        {#each rows as r (r.key)}
          <li>
            <span class="m-name">{r.name}</span>
            {#if r.inv}<span class="tr-mono"> · {r.inv}</span>{/if}
            {#if r.gone}<span class="gone"> — выведена</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отменить</Button>
    <Button
      variant="destructive"
      loading={saving}
      disabled={loading || loadErr || comp === null}
      onclick={confirm}>Списать</Button
    >
  {/snippet}
</Modal>

<style lang="scss">
  .writeoff {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }

  .body,
  .empty {
    margin: 0;
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
    white-space: normal;
    text-align: left;
  }

  .state {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    color: var(--tr-text-secondary);
  }

  .state-error {
    justify-content: space-between;
    color: var(--tr-danger-text);
  }

  .err-text {
    color: var(--tr-danger-text);
  }

  .members {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    white-space: normal;
    text-align: left;
  }

  .m-name {
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .gone {
    color: var(--tr-text-secondary);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    color: var(--tr-danger);
  }
</style>
