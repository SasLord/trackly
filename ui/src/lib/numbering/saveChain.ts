// Phase 40.2 review fix (FE-IN-03, partial): shared, rune-free helpers for the
// D-01 save chain (occupied → mismatch → script-mix) that DeviceFormBody,
// ActFormBody and CartridgeFormBody each orchestrate. Each form still owns its
// own `$state` (popup kind, taken/mismatch/script-warning payloads) — only the
// decision logic lives here, so a fix to «which confirmation applies» or «is
// this CONFLICT really the number» lands once instead of three times.
//
// Deliberately NOT a `.svelte.ts` rune store: moving the popup state machine
// itself out of the three forms is a bigger refactor whose runtime behaviour
// static gates (svelte-check / build) cannot verify.

/** D-01/D-04: a «Продолжить» confirmation is given for ONE concrete number —
 *  never for the whole form session (FE-CR-01). `null` — nothing confirmed. */
export type PendingConfirm = {
  value: string;
  mismatch: boolean;
  scriptMix: boolean;
} | null;

function key(value: string): string {
  return value.trim();
}

/** The confirmations that apply to `value` right now. A confirmation recorded
 *  for a different number is ignored, so the server re-checks the new number
 *  and the matching popup is shown again. */
export function confirmsFor(
  pending: PendingConfirm,
  value: string,
): { mismatch: boolean; scriptMix: boolean } {
  if (pending === null || pending.value !== key(value)) {
    return { mismatch: false, scriptMix: false };
  }
  return { mismatch: pending.mismatch, scriptMix: pending.scriptMix };
}

/** Records «Продолжить» for `kind` on `value`. An earlier confirmation of the
 *  other kind is kept only if it was given for the same number. */
export function withConfirm(
  pending: PendingConfirm,
  value: string,
  kind: 'mismatch' | 'scriptMix',
): PendingConfirm {
  const current = confirmsFor(pending, value);
  return { value: key(value), ...current, [kind]: true };
}
