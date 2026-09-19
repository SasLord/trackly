<script lang="ts">
  // Phase 40.2 Plan 10 — справка по токенам маски инвентарных номеров.
  // Без пропсов: статичный контент, дословно из
  // 40.2-UI-SPEC.md «Copywriting Contract» § «Справка по токенам».
  // Переиспользуется в блоке «Настройки / Организация» (Task 2) и в
  // NumberTemplateModal.svelte (Task 1).
  const TOKENS: { token: string; description: string }[] = [
    { token: '[YYYY]', description: 'год, 4 цифры — 2026' },
    { token: '[YY]', description: 'год, 2 цифры — 26' },
    { token: '[MM]', description: 'месяц — 01…12' },
    { token: '[DD]', description: 'день — 01…31' },
    { token: '[X]', description: 'порядковый номер без нулей — 1, 2 … 125' },
    {
      token: '[XXXX]',
      description: 'порядковый номер с нулями, сколько X — столько цифр — 0001 … 9999',
    },
  ];
</script>

<div class="mask-help">
  <dl class="mask-help-defs">
    {#each TOKENS as { token, description } (token)}
      <div class="mask-help-row">
        <dt class="tr-mono">{token}</dt>
        <dd>{description}</dd>
      </div>
    {/each}
  </dl>
  <p class="mask-help-rule">
    В маске ровно один порядковый номер ([X], [XX], [XXX] …). Остальной текст, включая другие
    скобки, остаётся как есть. Токены пишутся заглавными латинскими буквами.
  </p>
  <p class="mask-help-example">
    <span class="tr-mono">ОРГ-00-[XXXXXX]</span> → <span class="tr-mono">ОРГ-00-000001</span>
  </p>
  <p class="mask-help-example">
    <span class="tr-mono">[YYYY]/[MM]-[X]</span> → <span class="tr-mono">2026/09-1</span> — нумерация
    начинается заново каждый месяц
  </p>
</div>

<style lang="scss">
  .mask-help {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }

  .mask-help-defs {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .mask-help-row {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: var(--tr-space-xs);
  }

  .mask-help-row dt {
    margin: 0;
    color: var(--tr-text-primary);
  }

  .mask-help-row dd {
    margin: 0;
    color: var(--tr-text-secondary);
  }

  .mask-help-rule,
  .mask-help-example {
    margin: 0;
  }
</style>
