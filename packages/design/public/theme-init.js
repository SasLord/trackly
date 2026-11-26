// Anti-FOUC theme bootstrap. Runs before paint to set data-theme from the
// persisted preference (or OS preference) so the first frame isn't a flash of
// the wrong theme. Kept as an external file (not inline) so the server-mode
// Content-Security-Policy `script-src 'self'` does not block it — inline
// scripts would require 'unsafe-inline' (rejected, WR-07) or a brittle hash.
(function () {
  try {
    var t = localStorage.getItem('trackly:theme');
    var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    var resolved = t === 'light' || t === 'dark' ? t : prefersDark ? 'dark' : 'light';
    document.documentElement.dataset.theme = resolved;
  } catch (e) {}
})();
