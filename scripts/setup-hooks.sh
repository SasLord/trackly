#!/usr/bin/env bash
#
# One-line activation for this repository's git hooks (PRIV-02, D-10).
#
# Git does NOT enable a repo's .githooks/ directory automatically — a
# checkout with a populated .githooks/pre-commit still runs no hook at all
# until core.hooksPath is explicitly pointed at it. This is deliberately
# opt-in rather than auto-wired via a package-manager postinstall hook
# (D-10 rejected that alternative): silently changing a developer's git
# config on install is a surprise side effect, not something this project
# does without an explicit, one-command action a developer chooses to run.
#
# Run once per clone: ./scripts/setup-hooks.sh

set -euo pipefail

cd "$(dirname "$0")/.."

git config core.hooksPath .githooks

echo "core.hooksPath set to .githooks — the privacy gate now runs on every local commit."
