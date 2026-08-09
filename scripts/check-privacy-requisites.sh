#!/usr/bin/env bash
#
# Privacy gate (WR-11 / PRIV-01): organization requisites must never be
# hardcoded into the repository as real values.
#
# The repository is PUBLIC and CLAUDE.md states the constraint explicitly:
# «Всё закоммиченное остаётся в истории git даже после удаления из HEAD».
# A scrub in HEAD is therefore necessary but not sufficient — the only durable
# control is preventing requisite-shaped literals from entering in the first
# place. Phase 34 had to scrub real-looking requisites out of the template
# preview's demo context; this gate exists so that cannot silently recur.
#
# How it works: every string literal assigned to a requisite key (inn, kpp,
# okpo, ogrn, phone, fax) in Rust or HTML sources must appear in the explicit
# ALLOWED list below. That is deliberately an ALLOWLIST, not a "looks fake
# enough" heuristic — a heuristic quietly passes anything it did not anticipate,
# which is exactly the failure mode this gate exists to close. Adding a value
# here is a one-line, reviewable act that forces a human to confirm it is
# fictional.
#
# Run locally: ./scripts/check-privacy-requisites.sh

set -euo pipefail

cd "$(dirname "$0")/.."

# Fictional placeholder values approved for fixtures, tests and demo contexts.
# NEVER add a real organization's requisites here.
ALLOWED=(
  # Preview demo context (template_service.rs)
  "+7 495 123-45-67"
  "+7 495 123-45-68"
  "7700000000"
  "770000000"
  "12345678"
  "1027700123456"
  # Integration-test fixtures
  "+7 495 000-00-00"
  "+7 495 000-00-01"
  "+7 495 000-00-02"
  "7712345678"
  "771001001"
  "771201001"
  "87654321"
  "1027700654321"
  "1027700000000"
  # First-run placeholder org (organization_service.rs::placeholder)
  "0000000000"
  "000000000"
  # organization_io.rs fixtures («ООО Ромашка», path-traversal lure)
  "770001001"
  "1"
  "2"
  # pdf_render_act.rs org.json fixture
  "1234567890"
  "111222333"
  # Structurally-empty values
  ""
)

is_allowed() {
  local value="$1"
  local allowed
  for allowed in "${ALLOWED[@]}"; do
    [ "$value" = "$allowed" ] && return 0
  done
  return 1
}

# Matches both JSON-ish (`"phone": "…"`) and Rust struct-init (`phone: "…"`)
# assignment shapes. Word-boundary anchored so `telephone:` / `okpo_raw:` do not
# match by accident.
PATTERN='(^|[^A-Za-z0-9_"])"?(inn|kpp|okpo|ogrn|phone|fax)"?[[:space:]]*:[[:space:]]*"[^"]*"'

violations=0
while IFS= read -r hit; do
  [ -z "$hit" ] && continue
  location="${hit%%:*}"
  rest="${hit#*:}"
  lineno="${rest%%:*}"
  # Value = contents of the first quoted string AFTER the key's colon.
  value="$(printf '%s' "$hit" |
    sed -nE 's/.*"?(inn|kpp|okpo|ogrn|phone|fax)"?[[:space:]]*:[[:space:]]*"([^"]*)".*/\2/p' |
    head -1)"
  if ! is_allowed "$value"; then
    echo "PRIVACY GATE: unrecognized requisite literal at ${location}:${lineno}"
    echo "  value: \"${value}\""
    violations=$((violations + 1))
  fi
done < <(git grep -nE "$PATTERN" -- '*.rs' '*.html' || true)

if [ "$violations" -gt 0 ]; then
  cat <<'EOF'

The repository is PUBLIC. Real organization requisites (ИНН, КПП, ОКПО, ОГРН,
телефон, факс) must never be committed — and anything committed stays in git
history even after it is deleted from HEAD.

If the value above is FICTIONAL, add it to ALLOWED in
scripts/check-privacy-requisites.sh. If it is REAL, replace it with a
placeholder BEFORE committing.
EOF
  exit 1
fi

echo "Privacy gate OK: all requisite literals are approved placeholders."
