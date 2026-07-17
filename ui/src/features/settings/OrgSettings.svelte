<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  // DTO from backend (settings_get_org)
  interface OrgSettingsDto {
    org_name: string;
    inn: string;
    kpp: string;
    address: string;
    has_logo: boolean;
    phone: string;
    fax: string;
    email: string;
    okpo: string;
    ogrn: string;
    address_line2: string;
  }

  let orgName = $state('');
  let inn = $state('');
  let kpp = $state('');
  let address = $state('');
  let addressLine2 = $state('');
  let hasLogo = $state(false);
  let phone = $state('');
  let fax = $state('');
  let email = $state('');
  let okpo = $state('');
  let ogrn = $state('');
  // Logo DTO from backend (settings_get_org_logo). logo_bytes/logo_mime are
  // omitted/null when no logo is stored.
  interface OrgLogoDto {
    logo_bytes?: number[] | null;
    logo_mime?: string | null;
  }

  // Preview <img> src. A `data:${mime};base64,...` URL — carries the stored MIME
  // so SVG renders (needs explicit image/svg+xml) and it is permitted by the
  // server-mode CSP `img-src 'self' data:` (blob: URLs are blocked there).
  let logoSrc = $state<string | null>(null);
  let saving = $state(false);
  let uploading = $state(false);
  let logoError = $state<string | null>(null);

  // Hidden file input for browser context
  let fileInputEl: HTMLInputElement | null = $state(null);

  async function loadOrg() {
    try {
      const dto = await apiCall<OrgSettingsDto>('settings_get_org', {});
      orgName = dto.org_name;
      inn = dto.inn;
      kpp = dto.kpp;
      address = dto.address;
      addressLine2 = dto.address_line2;
      hasLogo = dto.has_logo;
      phone = dto.phone;
      fax = dto.fax;
      email = dto.email;
      okpo = dto.okpo;
      ogrn = dto.ogrn;
      if (dto.has_logo) {
        await loadLogo();
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить настройки организации';
      pushToast('error', msg);
    }
  }

  async function loadLogo() {
    try {
      const dto = await apiCall<OrgLogoDto>('settings_get_org_logo', {});
      if (!dto.logo_bytes || dto.logo_bytes.length === 0) {
        logoSrc = null;
        return;
      }
      const mime = dto.logo_mime || 'image/png';
      // Build a data: URL. Base64-encode the bytes (chunked to avoid blowing the
      // call-stack in String.fromCharCode for larger — up to 512 KiB — logos).
      const ua = new Uint8Array(dto.logo_bytes);
      let binary = '';
      const chunk = 0x8000;
      for (let i = 0; i < ua.length; i += chunk) {
        binary += String.fromCharCode(...ua.subarray(i, i + chunk));
      }
      logoSrc = `data:${mime};base64,${btoa(binary)}`;
    } catch {
      logoSrc = null;
    }
  }

  onMount(() => {
    loadOrg();
  });

  async function saveOrg() {
    saving = true;
    try {
      await apiCall<void>('settings_save_org_fields', {
        patch: {
          org_name: orgName,
          inn,
          kpp,
          address,
          address_line2: addressLine2,
          phone,
          fax,
          email,
          okpo,
          ogrn,
        },
      });
      pushToast('success', 'Настройки организации сохранены');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить настройки организации';
      pushToast('error', msg);
    } finally {
      saving = false;
    }
  }

  function detectMime(filename: string): string {
    const lower = filename.toLowerCase();
    if (lower.endsWith('.png')) return 'image/png';
    if (lower.endsWith('.jpg') || lower.endsWith('.jpeg')) return 'image/jpeg';
    if (lower.endsWith('.svg')) return 'image/svg+xml';
    return 'image/png';
  }

  async function uploadLogo() {
    logoError = null;
    const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
    if (isTauri) {
      uploading = true;
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          filters: [{ name: 'Изображения', extensions: ['png', 'jpg', 'jpeg', 'svg'] }],
          multiple: false,
        });
        if (!selected) return;
        const filePath = typeof selected === 'string' ? selected : (selected as string[])[0];
        const { readFile } = await import('@tauri-apps/plugin-fs');
        const bytes = await readFile(filePath);
        const mime = detectMime(filePath);
        await apiCall<void>('settings_save_org_logo', {
          logoBytes: Array.from(bytes),
          logoMime: mime,
        });
        hasLogo = true;
        await loadLogo();
        pushToast('success', 'Логотип загружен');
      } catch (e: unknown) {
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить логотип';
        pushToast('error', msg);
      } finally {
        uploading = false;
      }
    } else {
      // Browser: trigger hidden file input
      fileInputEl?.click();
    }
  }

  async function handleFileInput(e: Event) {
    logoError = null;
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    // T-07-04-01: frontend size check (512 KB)
    if (file.size > 512 * 1024) {
      logoError = 'Файл слишком большой. Максимальный размер: 512 КБ.';
      input.value = '';
      return;
    }

    uploading = true;
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      await apiCall<void>('settings_save_org_logo', {
        logoBytes: bytes,
        logoMime: file.type || 'image/png',
      });
      hasLogo = true;
      await loadLogo();
      pushToast('success', 'Логотип загружен');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить логотип';
      pushToast('error', msg);
    } finally {
      uploading = false;
      input.value = '';
    }
  }

  async function removeLogo() {
    try {
      await apiCall<void>('settings_remove_org_logo', {});
      hasLogo = false;
      logoSrc = null;
      pushToast('success', 'Логотип удалён');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить логотип';
      pushToast('error', msg);
    }
  }
</script>

<section class="settings-section">
  <h2 class="section-title">Организация</h2>

  <div class="form-grid">
    <div class="form-field form-field--full">
      <label class="form-label" for="org-name">Название организации</label>
      <input
        id="org-name"
        class="form-input"
        type="text"
        bind:value={orgName}
        placeholder="ООО «Название»"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-inn">ИНН</label>
      <input
        id="org-inn"
        class="form-input"
        type="text"
        bind:value={inn}
        placeholder="0000000000"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-kpp">КПП</label>
      <input id="org-kpp" class="form-input" type="text" bind:value={kpp} placeholder="000000000" />
    </div>

    <div class="form-field form-field--full">
      <label class="form-label" for="org-address">Адрес</label>
      <input
        id="org-address"
        class="form-input"
        type="text"
        bind:value={address}
        placeholder="г. Москва, ул. Примерная, д. 1"
      />
    </div>

    <div class="form-field form-field--full">
      <label class="form-label" for="org-address-line2">Адрес (2-я строка)</label>
      <input
        id="org-address-line2"
        class="form-input"
        type="text"
        bind:value={addressLine2}
        placeholder="офис 305, корпус 2"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-phone">Телефон</label>
      <input
        id="org-phone"
        class="form-input"
        type="text"
        bind:value={phone}
        placeholder="+7 (000) 000-00-00"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-fax">Факс</label>
      <input
        id="org-fax"
        class="form-input"
        type="text"
        bind:value={fax}
        placeholder="+7 (000) 000-00-00"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-email">E-mail</label>
      <input
        id="org-email"
        class="form-input"
        type="email"
        bind:value={email}
        placeholder="info@example.ru"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-okpo">ОКПО</label>
      <input
        id="org-okpo"
        class="form-input"
        type="text"
        bind:value={okpo}
        placeholder="00000000"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="org-ogrn">ОГРН</label>
      <input
        id="org-ogrn"
        class="form-input"
        type="text"
        bind:value={ogrn}
        placeholder="0000000000000"
      />
    </div>
  </div>

  <div class="save-row">
    <Button variant="primary" loading={saving} onclick={saveOrg}>
      Сохранить настройки организации
    </Button>
  </div>

  <div class="logo-section">
    <h3 class="subsection-title">Логотип организации</h3>
    <div class="logo-area">
      {#if hasLogo && logoSrc}
        <div class="logo-display">
          <!-- T-07-04-05: render as <img> — NOT raw SVG/HTML, scripts are blocked in img context -->
          <img src={logoSrc} alt="Логотип организации" class="logo-img" />
          <div class="logo-actions">
            <Button variant="ghost" size="sm" onclick={removeLogo}>Удалить логотип</Button>
            <Button variant="secondary" size="sm" loading={uploading} onclick={uploadLogo}>
              Загрузить логотип
            </Button>
          </div>
        </div>
      {:else}
        <div class="logo-placeholder">
          <span class="logo-placeholder-text">Нет логотипа</span>
          <Button variant="secondary" size="sm" loading={uploading} onclick={uploadLogo}>
            Загрузить логотип
          </Button>
        </div>
      {/if}
      {#if logoError}
        <p class="logo-error">{logoError}</p>
      {/if}
    </div>

    <!-- Hidden file input for browser context upload -->
    <input
      bind:this={fileInputEl}
      type="file"
      accept="image/png,image/jpeg,image/svg+xml"
      style="display:none"
      onchange={handleFileInput}
    />
  </div>
</section>

<style lang="scss">
  .settings-section {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    max-width: 640px;
  }

  .section-title {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .subsection-title {
    margin: 0 0 var(--tr-space-xs);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);

    &--full {
      grid-column: 1 / -1;
    }
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .form-input {
    padding: var(--tr-space-xs) var(--tr-space-md);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    background: var(--tr-bg);
    color: var(--tr-text-primary);

    &:focus {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--tr-accent) 20%, transparent);
    }
  }

  .save-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-xl);
  }

  .logo-section {
    border-top: 1px solid var(--tr-border);
    padding-top: var(--tr-space-md);
  }

  .logo-area {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .logo-display {
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
    flex-wrap: wrap;
  }

  .logo-img {
    max-height: 64px;
    max-width: 128px;
    object-fit: contain;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    padding: var(--tr-space-2xs);
    background: var(--tr-bg);
  }

  .logo-actions {
    display: flex;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .logo-placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-xs);
    border: 2px dashed var(--tr-border);
    border-radius: var(--tr-radius-xs);
    padding: var(--tr-space-xl);
    text-align: center;
    min-height: 80px;
  }

  .logo-placeholder-text {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .logo-error {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }
</style>
