// Зеркало контракта crates/link-app/src/state.rs (golden-фикстура tests/fixtures/link_state.json).
// Гейт scripts/check-link-state.mjs сверяет kind-ы и команды с фикстурой.
// Webview сети не касается: состояние приходит событием `link-event` и командой `link_get_state`.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type LoginReason = 'ip_address' | 'sso_disabled' | 'kerberos' | 'not_mapped' | 'no_answer';
export type LoginMethod = 'sso' | 'password';

export interface LinkUserView {
  display_name: string;
  login: string;
}

export type LinkState =
  | { kind: 'starting' }
  | { kind: 'needs_setup' }
  | { kind: 'pin_mismatch'; server: string; expected: string; got: string }
  | {
      kind: 'needs_login';
      reason: LoginReason | null;
      can_retry_sso: boolean;
      after_logout: boolean;
      server: string;
    }
  | { kind: 'revoked'; server: string }
  | { kind: 'connected'; user: LinkUserView; method: LoginMethod; server: string }
  | { kind: 'offline'; server: string };

export type LinkStateKind = LinkState['kind'];

export interface LinkEvent {
  state: LinkState;
  tray_available: boolean;
  secrets_persistent: boolean;
  platform: string;
}

export type CmdErrorCode =
  | 'empty_code'
  | 'invalid_code'
  | 'expired_code'
  | 'used_code'
  | 'server_unreachable'
  | 'pin_mismatch'
  | 'bad_credentials'
  | 'user_blocked'
  | 'device_banned'
  | 'rate_limited'
  | 'no_network'
  | 'auth_unavailable'
  | 'internal';

export interface CmdError {
  code: CmdErrorCode;
  message: string;
}

export type ScreenName = 'connecting' | 'setup' | 'pin_mismatch' | 'login' | 'shell';

/** Какой экран рисовать по состоянию (UI-SPEC §6.1). Exhaustive: новый kind не скомпилируется. */
export function screenFor(state: LinkState): ScreenName {
  switch (state.kind) {
    case 'starting':
      return 'connecting';
    case 'needs_setup':
      return 'setup';
    case 'pin_mismatch':
      return 'pin_mismatch';
    case 'needs_login':
    case 'revoked':
      return 'login';
    case 'connected':
    case 'offline':
      return 'shell';
    default: {
      const never: never = state;
      return never;
    }
  }
}

export const INITIAL_EVENT: LinkEvent = {
  state: { kind: 'starting' },
  tray_available: true,
  secrets_persistent: true,
  platform: '',
};

const inTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

type Subscriber = (e: LinkEvent) => void;

/** Стор по контракту Svelte (`$linkStore` в компонентах). Вне Tauri остаётся в `starting`. */
function createLinkStore() {
  let current: LinkEvent = INITIAL_EVENT;
  let started = false;
  const subs = new Set<Subscriber>();

  const set = (e: LinkEvent) => {
    current = e;
    subs.forEach((s) => s(e));
  };

  return {
    subscribe(fn: Subscriber) {
      fn(current);
      subs.add(fn);
      return () => {
        subs.delete(fn);
      };
    },
    /** Подписка на `link-event` и первичное чтение состояния. Повторный вызов — no-op. */
    async init(): Promise<void> {
      if (started || !inTauri()) return;
      started = true;
      try {
        await listen<LinkEvent>('link-event', (ev) => set(ev.payload));
        set(await invoke<LinkEvent>('link_get_state'));
      } catch {
        // остаёмся в starting; следующее событие из Rust всё равно придёт
      }
    },
  };
}

export const linkStore = createLinkStore();
