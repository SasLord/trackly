<script lang="ts">
  // Plan 06-08: ручное заведение принтера из устройства type=Принтер (PRN-04).
  // Двухшаговый submit: create device (type_id=2) → printers.create (SNMP опц.).
  // Паттерн: OperationModal.svelte / DiscoveryModal.svelte.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { devices } from '$lib/api/devices';
  import { printers } from './api';

  interface Props {
    open: boolean;
    onClose: () => void;
    onSuccess: () => void;
  }

  const { open, onClose, onSuccess }: Props = $props();

  // --- Form state ---
  let name = $state('');
  let location = $state('');
  let ipAddress = $state('');
  let community = $state('public');
  let submitting = $state(false);

  // --- Validation errors ---
  let nameError = $state('');

  // Reset on open
  $effect(() => {
    if (open) {
      name = '';
      location = '';
      ipAddress = '';
      community = 'public';
      submitting = false;
      nameError = '';
    }
  });

  function validate(): boolean {
    nameError = '';
    if (!name.trim()) {
      nameError = 'Наименование обязательно';
      return false;
    }
    return true;
  }

  async function handleSubmit() {
    if (submitting) return;
    if (!validate()) return;

    submitting = true;
    try {
      // Step 1: create device type=Принтер
      const device = await devices.create({
        type_id: 2,
        name: name.trim(),
        inventory_no: null,
        serial_no: null,
        model: null,
        specs: null,
        kit: null,
        state: null,
        location: location.trim() || null,
        location_id: null,
        status_id: 1,
      });

      // Step 2: create printer record (SNMP optional)
      const hasIp = ipAddress.trim().length > 0;
      await printers.create({
        deviceId: device.id,
        ipAddress: hasIp ? ipAddress.trim() : null,
        communityUpdate: hasIp ? community.trim() || 'public' : null,
        snmpVersion: 'v2c',
        oidProfileId: null,
        usbHostDeviceId: null,
      });

      pushToast('success', 'Принтер успешно заведён');
      onSuccess();
      onClose();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось завести принтер. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      submitting = false;
    }
  }
</script>

<Modal {open} title="Завести принтер" size="md" {onClose}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <!-- Наименование (обязательно) -->
    <div class="field">
      <label class="label" for="pc-name">Наименование</label>
      <Input
        id="pc-name"
        value={name}
        placeholder="Например: HP LaserJet Pro M404dn"
        invalid={!!nameError}
        oninput={(v) => {
          name = v;
          nameError = '';
        }}
      />
      {#if nameError}
        <span class="field-error">{nameError}</span>
      {/if}
    </div>

    <!-- Расположение (опционально) -->
    <div class="field">
      <label class="label" for="pc-location">Расположение</label>
      <LocationAutocomplete
        value={location}
        placeholder="Кабинет, склад и т.д. (необязательно)"
        id="pc-location"
        onChange={(v) => (location = v)}
      />
    </div>

    <!-- SNMP-секция (опционально) -->
    <div class="snmp-section">
      <p class="section-hint">
        SNMP / Сеть — заполните, если принтер подключён по сети (IP). Оставьте пустым для
        USB/локального принтера.
      </p>

      <div class="field">
        <label class="label" for="pc-ip">IP-адрес</label>
        <Input
          id="pc-ip"
          value={ipAddress}
          placeholder="192.168.1.100 (необязательно)"
          oninput={(v) => (ipAddress = v)}
        />
      </div>

      {#if ipAddress.trim()}
        <div class="field">
          <label class="label" for="pc-community">SNMP community</label>
          <Input
            id="pc-community"
            value={community}
            placeholder="public"
            oninput={(v) => (community = v)}
          />
        </div>
      {/if}
    </div>
  </form>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} onclick={handleSubmit}>Завести принтер</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    font-weight: var(--tr-font-weight-regular);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .snmp-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
    padding-top: var(--tr-space-md);
  }

  .section-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    margin: 0;
  }
</style>
