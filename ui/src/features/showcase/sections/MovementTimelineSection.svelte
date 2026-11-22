<script lang="ts">
  // Фаза 41.10 (HST-05): витрина единой ленты «История». Только вымышленные
  // данные, без apiCall (жёсткое правило приватности). Показывает все виды
  // записей, три варианта (device / group / cartridge), пустые состояния и ошибку.
  import MovementTimeline from '$lib/components/MovementTimeline.svelte';
  import type { HistoryEntryDto, HistorySegmentDto, MovementEntryDto } from '../../../bindings';

  const now = Math.floor(Date.now() / 1000);
  const day = 24 * 60 * 60;

  function movement(overrides: Partial<MovementEntryDto> & { id: number }): MovementEntryDto {
    return {
      entity_type: 'device',
      entity_id: 1,
      from_place_id: 5,
      from_place_path: 'Здание А / 2 этаж / Каб. 12',
      from_place_path_short: 'Каб. 12',
      to_place_id: 6,
      to_place_path: 'Здание А / 2 этаж / Каб. 14',
      to_place_path_short: 'Каб. 14',
      actor_display: 'Иванов И.И.',
      source: 'manual',
      note: null,
      act_id: null,
      act_number: null,
      batch_id: null,
      group_id: null,
      group_label: null,
      request_id: null,
      created_at_utc: now,
      ...overrides,
    };
  }

  function seg(overrides: Partial<HistorySegmentDto> & { kind: string }): HistorySegmentDto {
    return {
      event_kind: null,
      movement: null,
      subject_type: null,
      subject_id: null,
      subject_label: null,
      from_group_id: null,
      from_group_label: null,
      to_group_id: null,
      to_group_label: null,
      status_from_code: null,
      status_from_label: null,
      status_to_code: null,
      status_to_label: null,
      ...overrides,
    };
  }

  function entry(
    key: string,
    daysAgo: number,
    segments: HistorySegmentDto[],
    overrides: Partial<HistoryEntryDto> = {},
  ): HistoryEntryDto {
    return {
      key,
      created_at_utc: now - daysAgo * day,
      actor_display: 'Иванов И.И.',
      reason_source: 'manual',
      reason_is_event: true,
      act_id: null,
      act_number: null,
      request_id: null,
      ref_group_id: null,
      ref_group_label: null,
      segments,
      ...overrides,
    };
  }

  const moveSeg = (id: number, source = 'manual', extra: Partial<MovementEntryDto> = {}) =>
    seg({ kind: 'move', movement: movement({ id, source, ...extra }) });

  // Лента устройства со всеми видами (новые сверху, как отдаёт сервер).
  const deviceEntries: HistoryEntryDto[] = [
    entry(
      'e:12',
      0,
      [
        seg({
          kind: 'membership',
          event_kind: 'group_disbanded',
          from_group_id: 4,
          from_group_label: 'Системный блок #4',
        }),
      ],
      { reason_source: 'device_write_off', actor_display: 'Петров П.П.' },
    ),
    entry(
      'e:11',
      1,
      [
        seg({
          kind: 'membership',
          event_kind: 'group_formed',
          to_group_id: 4,
          to_group_label: 'Системный блок #4',
        }),
      ],
      { reason_source: 'group_create' },
    ),
    entry(
      'e:10',
      2,
      [
        moveSeg(30, 'act', { act_id: 123, act_number: '123', from_place_path_short: 'Склад' }),
        seg({
          kind: 'status',
          event_kind: 'status_changed',
          status_from_code: 'in_stock',
          status_from_label: 'На складе',
          status_to_code: 'in_use',
          status_to_label: 'В работе',
        }),
      ],
      { reason_source: 'act', act_id: 123, act_number: '123', actor_display: 'Петров П.П.' },
    ),
    entry('e:9', 3, [
      seg({
        kind: 'status',
        event_kind: 'status_changed',
        status_from_code: 'in_use',
        status_from_label: 'В работе',
        status_to_code: 'write_off_pending',
        status_to_label: 'На списание',
      }),
    ]),
    entry(
      'e:8',
      4,
      [
        seg({
          kind: 'membership',
          event_kind: 'group_transferred',
          from_group_id: 3,
          from_group_label: 'АРМ-3',
          to_group_id: 5,
          to_group_label: 'АРМ-5',
        }),
        moveSeg(29),
      ],
      { reason_source: 'manual' },
    ),
    entry(
      'e:7',
      5,
      [
        seg({
          kind: 'membership',
          event_kind: 'group_joined',
          to_group_id: 5,
          to_group_label: 'АРМ-5',
        }),
      ],
      { reason_source: 'act', act_id: 99, act_number: '99' },
    ),
    entry('e:6', 6, [seg({ kind: 'garbage', event_kind: 'future_kind' })], {
      reason_source: 'future_reason',
    }),
    entry('m:5', 7, [moveSeg(5, 'repair', { request_id: 17 })], {
      reason_is_event: false,
      reason_source: 'repair',
      request_id: 17,
    }),
    entry('m:4', 8, [moveSeg(4, 'manual', { note: 'уточнение' })], {
      reason_is_event: false,
      reason_source: 'manual',
      actor_display: 'Петров П.П.',
    }),
  ];

  // Лента группы (variant group): события из UI-SPEC §9, без чипа «Статус».
  const groupEntries: HistoryEntryDto[] = [
    entry('e:22', 0, [
      seg({
        kind: 'membership',
        event_kind: 'group_joined',
        subject_type: 'device',
        subject_id: 21,
        subject_label: 'Монитор #2',
        to_group_id: 5,
        to_group_label: 'АРМ-5',
      }),
    ]),
    entry('e:21', 1, [
      seg({
        kind: 'membership',
        event_kind: 'group_transferred',
        subject_type: 'device',
        subject_id: 21,
        subject_label: 'Монитор #2',
        from_group_id: 3,
        from_group_label: 'АРМ-3',
        to_group_id: 5,
        to_group_label: 'АРМ-5',
      }),
    ]),
    entry('e:20', 2, [
      seg({
        kind: 'membership',
        event_kind: 'group_nested',
        subject_type: 'group',
        subject_id: 5,
        to_group_id: 8,
        to_group_label: 'Рабочее место 8',
      }),
    ]),
    entry('m:19', 3, [moveSeg(19, 'group', { entity_type: 'group', group_id: 5 })], {
      reason_is_event: false,
      reason_source: 'group',
      actor_display: 'Петров П.П.',
    }),
  ];

  // Картридж (variant cartridge): только перемещения, без чипов.
  const cartridgeEntries: HistoryEntryDto[] = [
    entry('m:41', 0, [moveSeg(41, 'manual', { entity_type: 'cartridge' })], {
      reason_is_event: false,
    }),
    entry(
      'm:40',
      2,
      [moveSeg(40, 'act', { entity_type: 'cartridge', act_id: 42, act_number: '42' })],
      {
        reason_is_event: false,
        reason_source: 'act',
        actor_display: 'Петров П.П.',
      },
    ),
  ];

  function log(what: string) {
    return (id: number) => console.log(`[showcase] navigate to ${what}`, id);
  }
</script>

<section class="movement-timeline-section">
  <h2>MovementTimeline</h2>

  <div class="variant-block">
    <h3 class="variant-label">Устройство: все виды записей, чипы-фильтры (включая «garbage»)</h3>
    <div class="demo-anchor">
      <MovementTimeline
        entries={deviceEntries}
        loading={false}
        loadError={false}
        onNavigateToPlace={log('place')}
        onNavigateToAct={log('act')}
        onNavigateToGroup={log('group')}
        onNavigateToRequest={log('request')}
        onNavigateToDevice={log('device')}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">variant group: без чипа «Статус»</h3>
    <div class="demo-anchor">
      <MovementTimeline
        entries={groupEntries}
        loading={false}
        loadError={false}
        variant="group"
        groupId={5}
        onNavigateToPlace={log('place')}
        onNavigateToGroup={log('group')}
        onNavigateToDevice={log('device')}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Картридж (variant cartridge): только перемещения, чипов нет</h3>
    <div class="demo-anchor">
      <MovementTimeline
        entries={cartridgeEntries}
        loading={false}
        loadError={false}
        variant="cartridge"
        onNavigateToPlace={log('place')}
        onNavigateToAct={log('act')}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Пусто: устройство</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={false} />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Пусто: variant group</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={false} variant="group" />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Пусто: картридж</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={false} variant="cartridge" />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Ошибка загрузки</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={true} />
    </div>
  </div>
</section>

<style lang="scss">
  .movement-timeline-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .variant-block {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--tr-space-sm);
  }

  .variant-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .demo-anchor {
    max-width: 560px;
  }
</style>
