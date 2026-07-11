---
status: partial
phase: 18-autocomplete-dropdowns
source: [18-VERIFICATION.md]
started: 2026-07-11T00:00:00Z
updated: 2026-07-11T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. Enter в drill-in/member-режиме дропдауна пикера устройства (Акты → Позиции)
expected: Enter, когда открыт member/drill-in список, не вызывает сабмит формы акта (в group-режиме уже подавляется; WR-02 добавил подавление и в member-режиме)
result: [pending]

### 2. Удаление не-последней строки позиции при открытом/использованном пикере на нескольких строках
expected: состояние дропдаунов (открыт/закрыт, groups/members, drill-заголовок) корректно сдвигается вместе со строками, не «залипает» на старом индексе (WR-01 — removeRow теперь реиндексирует все transient-мапы)
result: [pending]

## Summary

total: 2
passed: 0
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
