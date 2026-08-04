// Silent AD SSO (Kerberos / SPNEGO / Negotiate) auto-login — spike-002/003.
//
// Server-mode / LAN-browser ONLY. On a domain-joined Windows machine the browser can
// present a Kerberos ticket transparently (no username/password prompt); this module
// pokes the backend `GET /api/v1/auth_ad_sso` endpoint so the browser performs that
// Negotiate handshake. Mirrors adwebapp's `auth-autoad.js`.
//
// Why it must be a real GET the browser retries (not something faked): SPNEGO is a
// two-step exchange — the server answers the first request with `401 +
// WWW-Authenticate: Negotiate`, and the domain browser SILENTLY resends it with a
// ticket. So we just fetch the endpoint and let the browser do the handshake.
//
// If there is no ticket / AD is unavailable / SSO is disabled on the server, we set a
// session-scoped `ad_skip` cookie so we don't re-poke on every page load. It has no
// Max-Age, so it clears when the browser closes and auto-login resumes next session —
// exactly adwebapp's behaviour. Explicit login always stays available.

const AD_SKIP_COOKIE = 'trackly_ad_skip';

function isBrowserServerMode(): boolean {
  // Desktop (Tauri) has no LAN Negotiate flow — SSO is server-mode only.
  return typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window);
}

function hasAdSkip(): boolean {
  return (
    typeof document !== 'undefined' &&
    document.cookie.split('; ').some((c) => c.startsWith(`${AD_SKIP_COOKIE}=`))
  );
}

function setAdSkip(): void {
  // Session cookie (no Max-Age/Expires): browser drops it on close, so a fresh session
  // retries SSO automatically.
  if (typeof document !== 'undefined') {
    document.cookie = `${AD_SKIP_COOKIE}=1; path=/; SameSite=Lax`;
  }
}

/**
 * Attempt a silent AD SSO login. Returns `true` only if the server confirmed the
 * handshake (a Trackly session cookie was issued); the caller should then re-load auth
 * status to populate the user. Returns `false` (and suppresses further attempts this
 * browser session) in every other case — no ticket, AD off, SSO disabled, or desktop.
 *
 * Never throws: any failure resolves to `false` so app startup falls through to the
 * normal login screen.
 */
export async function trySilentAdSso(): Promise<boolean> {
  if (!isBrowserServerMode() || hasAdSkip()) return false;

  try {
    const res = await fetch('/api/v1/auth_ad_sso', { credentials: 'same-origin' });
    if (res.ok) {
      const data = (await res.json().catch(() => null)) as { ok?: boolean } | null;
      if (data?.ok) return true;
    }
    // 401 (no ticket) / 503 (SSO disabled) / anything else → don't nag again this session.
    setAdSkip();
    return false;
  } catch {
    // Network error — also suppress for this session.
    setAdSkip();
    return false;
  }
}
