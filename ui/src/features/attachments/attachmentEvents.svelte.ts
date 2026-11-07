// Фаза 41.3, план 14 (D-14, D-15): события вложений родителя; перечитываются
// вместе с блоком по WebSocket-событию attachments_changed.
import { onWsEvent } from '$lib/api/ws';
import { attachments } from './api';
import type { AttachmentHistoryEventDto } from '../../bindings';

/** Вызывать при инициализации компонента (использует $effect). */
export function createAttachmentEvents(
  getKind: () => 'request' | 'device' | 'cartridge',
  getId: () => number | null | undefined,
) {
  let events = $state<AttachmentHistoryEventDto[]>([]);
  let token = 0;

  async function load() {
    const id = getId();
    const kind = getKind();
    const mine = ++token;
    if (id === null || id === undefined) {
      events = [];
      return;
    }
    try {
      const res = await attachments.history(kind, id);
      if (mine === token) events = res;
    } catch {
      if (mine === token) events = [];
    }
  }

  $effect(() => {
    void getKind();
    void getId();
    void load();
  });

  $effect(() => {
    const off = onWsEvent((ev) => {
      if (
        ev.type === 'attachments_changed' &&
        ev.parentKind === getKind() &&
        ev.parentId === getId()
      ) {
        void load();
      }
    });
    return off;
  });

  return {
    get events() {
      return events;
    },
  };
}
