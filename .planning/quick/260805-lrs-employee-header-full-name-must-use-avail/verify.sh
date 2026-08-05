#!/usr/bin/env bash
set -uo pipefail

FILE="/Users/madsas/Projects/trackly/ui/src/features/layout/EmployeeLayout.svelte"

node -e "
const fs = require('fs');
const path = '$FILE';
const s = fs.readFileSync(path, 'utf8');
const styleMatch = s.match(/<style lang=\"scss\">([\s\S]*)<\/style>/);
if (!styleMatch) { console.log('FAIL_NO_STYLE_BLOCK'); process.exit(1); }
const css = styleMatch[1];
function ruleBody(selector) {
  const idx = css.indexOf(selector + ' {');
  if (idx === -1) return null;
  const end = css.indexOf('}', idx);
  return css.slice(idx, end);
}
const actions = ruleBody('.employee-header-actions');
if (!actions) { console.log('FAIL_NO_ACTIONS_RULE'); process.exit(1); }
if (!/min-width:\s*0/.test(actions)) { console.log('FAIL_ACTIONS_NO_MIN_WIDTH'); process.exit(1); }
const userName = ruleBody('.user-name');
if (!userName) { console.log('FAIL_NO_USER_NAME_RULE'); process.exit(1); }
if (/max-width:\s*200px/.test(userName)) { console.log('FAIL_USER_NAME_STILL_HAS_MAX_WIDTH'); process.exit(1); }
if (/flex-shrink:\s*0/.test(userName)) { console.log('FAIL_USER_NAME_STILL_FLEX_SHRINK_0'); process.exit(1); }
if (!/min-width:\s*0/.test(userName)) { console.log('FAIL_USER_NAME_NO_MIN_WIDTH'); process.exit(1); }
if (!/white-space:\s*nowrap/.test(userName)) { console.log('FAIL_USER_NAME_NO_NOWRAP'); process.exit(1); }
if (!/overflow:\s*hidden/.test(userName)) { console.log('FAIL_USER_NAME_NO_OVERFLOW_HIDDEN'); process.exit(1); }
if (!/text-overflow:\s*ellipsis/.test(userName)) { console.log('FAIL_USER_NAME_NO_ELLIPSIS'); process.exit(1); }
const userRole = ruleBody('.user-role');
if (!userRole) { console.log('FAIL_NO_USER_ROLE_RULE'); process.exit(1); }
if (!/flex-shrink:\s*0/.test(userRole)) { console.log('FAIL_USER_ROLE_NOT_FIXED'); process.exit(1); }
if (!/white-space:\s*nowrap/.test(userRole)) { console.log('FAIL_USER_ROLE_NO_NOWRAP'); process.exit(1); }
const otherMaxWidth200 = (css.match(/max-width:\s*200px/g) || []).length;
if (otherMaxWidth200 !== 0) { console.log('FAIL_MAX_WIDTH_200_STILL_PRESENT_' + otherMaxWidth200); process.exit(1); }
console.log('OK_EMPLOYEE_HEADER_NAME_SHRINK_GATES_PASS');
" || exit 1

UNRELATED_COUNT=$(grep -l "max-width: 200px" \
  /Users/madsas/Projects/trackly/ui/src/features/acts/ActNumberField.svelte \
  /Users/madsas/Projects/trackly/ui/src/features/devices/DeviceListRow.svelte \
  /Users/madsas/Projects/trackly/ui/src/features/devices/DeviceImportCsvModal.svelte \
  2>/dev/null | wc -l | tr -d ' ')
echo "UNRELATED_MAX_WIDTH_FILES_STILL_MATCHING=$UNRELATED_COUNT (informational — confirms those files were not this plan's target, not a pass/fail gate)"

cd /Users/madsas/Projects/trackly/ui || exit 1
echo "--- svelte-check ---"
pnpm svelte-check 2>&1 | tail -20
SC_STATUS=${PIPESTATUS[0]:-0}
echo "--- lint ---"
pnpm lint 2>&1 | tail -30
LINT_STATUS=${PIPESTATUS[0]:-0}
echo "--- build ---"
pnpm build 2>&1 | tail -20
BUILD_STATUS=${PIPESTATUS[0]:-0}

if [ "$SC_STATUS" != "0" ] || [ "$LINT_STATUS" != "0" ] || [ "$BUILD_STATUS" != "0" ]; then
  echo "FAIL_TOOLCHAIN_GATE sc=$SC_STATUS lint=$LINT_STATUS build=$BUILD_STATUS"
  exit 1
fi
echo "OK_TOOLCHAIN_GATES_PASS"
