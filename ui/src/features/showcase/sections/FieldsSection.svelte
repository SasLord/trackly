<script lang="ts">
  import Input from '$lib/components/Input.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Radio from '$lib/components/Radio.svelte';

  let inputNormal = $state('Пример текста');
  let inputInvalid = $state('Некорректное значение');
  let inputDisabled = $state('Недоступно для правки');

  let selectNormal = $state('a');
  let selectInvalid = $state('');
  let selectDisabled = $state('a');

  let textareaNormal = $state('Многострочный пример текста для демонстрации.');
  let textareaInvalid = $state('');
  let textareaDisabled = $state('Недоступно для правки');

  let checkboxNormal = $state(true);
  let checkboxInvalid = $state(false);
  let checkboxDisabled = $state(true);

  let radioGroupNormal = $state('a');
  let radioGroupInvalid = $state('');
  let radioGroupDisabled = $state('a');
</script>

<section class="fields-section">
  <h2>Поля ввода</h2>

  <div class="field-row">
    <h3 class="field-label">Input</h3>
    <div class="state-group">
      <div class="state-cell">
        <span class="state-tag">Обычное</span>
        <Input bind:value={inputNormal} />
      </div>
      <div class="state-cell">
        <span class="state-tag">Ошибка</span>
        <Input bind:value={inputInvalid} invalid />
      </div>
      <div class="state-cell">
        <span class="state-tag">Отключено</span>
        <Input bind:value={inputDisabled} disabled />
      </div>
    </div>
  </div>

  <div class="field-row">
    <h3 class="field-label">Select</h3>
    <div class="state-group">
      <div class="state-cell">
        <span class="state-tag">Обычное</span>
        <Select bind:value={selectNormal}>
          <option value="a">Вариант A</option>
          <option value="b">Вариант B</option>
        </Select>
      </div>
      <div class="state-cell">
        <span class="state-tag">Ошибка</span>
        <Select bind:value={selectInvalid} invalid>
          <option value="">Выберите значение</option>
          <option value="a">Вариант A</option>
        </Select>
      </div>
      <div class="state-cell">
        <span class="state-tag">Отключено</span>
        <Select bind:value={selectDisabled} disabled>
          <option value="a">Вариант A</option>
        </Select>
      </div>
    </div>
  </div>

  <div class="field-row">
    <h3 class="field-label">Textarea</h3>
    <div class="state-group">
      <div class="state-cell">
        <span class="state-tag">Обычное</span>
        <Textarea bind:value={textareaNormal} />
      </div>
      <div class="state-cell">
        <span class="state-tag">Ошибка</span>
        <Textarea bind:value={textareaInvalid} invalid placeholder="Обязательное поле" />
      </div>
      <div class="state-cell">
        <span class="state-tag">Отключено</span>
        <Textarea bind:value={textareaDisabled} disabled />
      </div>
    </div>
  </div>

  <div class="field-row">
    <h3 class="field-label">Checkbox</h3>
    <div class="state-group">
      <div class="state-cell">
        <span class="state-tag">Обычное</span>
        <Checkbox bind:checked={checkboxNormal}>Согласен с условиями</Checkbox>
      </div>
      <div class="state-cell">
        <span class="state-tag">Ошибка</span>
        <Checkbox bind:checked={checkboxInvalid} invalid>Обязательный пункт</Checkbox>
      </div>
      <div class="state-cell">
        <span class="state-tag">Отключено</span>
        <Checkbox bind:checked={checkboxDisabled} disabled>Недоступный пункт</Checkbox>
      </div>
    </div>
  </div>

  <div class="field-row">
    <h3 class="field-label">Radio</h3>
    <div class="state-group">
      <div class="state-cell">
        <span class="state-tag">Обычное</span>
        <div class="radio-group">
          <Radio bind:group={radioGroupNormal} value="a">Вариант A</Radio>
          <Radio bind:group={radioGroupNormal} value="b">Вариант B</Radio>
        </div>
      </div>
      <div class="state-cell">
        <span class="state-tag">Ошибка</span>
        <Radio bind:group={radioGroupInvalid} value="a" invalid>Обязательный выбор</Radio>
      </div>
      <div class="state-cell">
        <span class="state-tag">Отключено</span>
        <Radio bind:group={radioGroupDisabled} value="a" disabled>Недоступный вариант</Radio>
      </div>
    </div>
  </div>
</section>

<style lang="scss">
  .fields-section {
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

  .field-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
  }

  .field-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .state-group {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-lg);
    flex-wrap: wrap;
  }

  .state-cell {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    min-width: 220px;
  }

  .state-tag {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }
</style>
