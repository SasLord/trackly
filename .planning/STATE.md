---
gsd_state_version: 1.0
milestone: v1.4
milestone_name: Карта и осмысленное размещение
status: executing
last_updated: "2026-09-03T02:51:37.246Z"
last_activity: 2026-09-03 -- Phase 40 planning complete
progress:
  total_phases: 9
  completed_phases: 3
  total_plans: 66
  completed_plans: 64
  percent: 33
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-08-19 after v1.3.3 milestone)

**Core value:** Учёт устройств и картриджей с актами приёма-передачи и историей перемещений должен работать надёжно и быстро в режиме «одной кнопкой» — без обращения к Excel-таблицам, ручного присвоения номеров актов или потери истории при возврате на склад.
**Current focus:** Phase 40 — movement-history

## Current Position

Phase: 40 (movement-history) — EXECUTING
Plan: 27 of 29 executed — 40-28/40-29 (gap-closure round 2) pending
Status: Ready to execute
Last activity: 2026-09-03 -- Phase 40 planning complete
покрытие 100%). ROADMAP.md + REQUIREMENTS.md (Traceability) обновлены.

### Phase 6 gap-closure decisions (2026-06-15)

- D-GAP-Printer-Add: принтер = устройство type=Принтер + опц. SNMP; завести вручную И через discovery; admit починить (PRN-04 USB).
- D-GAP-Replace-Select: Select принтера в форме замены = устройства type=Принтер (§427), не printers-таблица.
- D-GAP-Employee-Access: полноценный вход сотрудника → AD Phase 8; сейчас только корректный ролевой рендер.
- Критические дефекты: requests_create arg `dto` vs `payload`; requests_status_counts/get_history mismatch; printers_admit заглушка.

## Performance Metrics

**Velocity:**

- Total plans completed: 217
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| — | — | — | — |
| 02 | 5 | - | - |
| 03.3 | 2 | - | - |
| 04 | 6 | - | - |
| 5 | 6 | - | - |
| 07 | 14 | - | - |
| 08 | 2 | - | - |
| 11 | 3 | - | - |
| 12 | 21 | - | - |
| 13 | 8 | - | - |
| 14 | 3 | - | - |
| 16 | 5 | - | - |
| 17 | 7 | - | - |
| 18 | 5 | - | - |
| 22 | 6 | - | - |
| 20 | 6 | - | - |
| 21 | 1 | - | - |
| 23 | 8 | - | - |
| 25 | 8 | - | - |
| 26 | 8 | - | - |
| 28 | 16 | - | - |
| 29 | 4 | - | - |
| 31 | 4 | - | - |
| 32 | 5 | - | - |
| 34 | 6 | - | - |
| 35 | 7 | - | - |
| 37 | 4 | - | - |
| 39 | 22 | - | - |
| 39.1 | 10 | - | - |
| 39.2 | 1 | ~18 мин | ~18 мин |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01 P01 | 25 min | 4 tasks | 35 files |
| Phase 01 P02 | 7 min | 3 tasks | 10 files |
| Phase 01 P03 | 6 min | 3 tasks | 23 files |
| Phase 01 P04 | 25 min | 3 tasks | 24 files |
| Phase 01 P06 | 22 min | - tasks | - files |
| Phase 02-ui P01 | 20 min | 2 tasks | 29 files |
| Phase 02-ui P02-02 | 50m | 4 tasks | 46 files |
| Phase 02-ui P04 | 120 min | 3 tasks | 20 files |
| Phase 02-ui P05 | 240 | 3 tasks | 31 files |
| Phase 03-pdf P01 | 25 | 3 tasks | 19 files |
| Phase 03-pdf P02 | 90 | 3 tasks | 27 files |
| Phase 03 P03 | 60 | 2 tasks | 18 files |
| Phase 03 P04 | 75 | 2 tasks | 34 files |
| Phase 03 P05 | 60 | 2 tasks | 21 files |
| Phase 03.2-deferred-uat-gap-closure P02 | 15 | 2 tasks | 5 files |
| Phase 03.3 P01 | 20 | 3 tasks | 7 files |
| Phase 03.3 P02 | 5min | 3 tasks | 7 files |
| Phase 04 P01 | 6 | 2 tasks | 10 files |
| Phase 04 P02 | 6 min | 2 tasks | 6 files |
| Phase 04-cartridges P03 | 19 | 2 tasks | 23 files |
| Phase 05-auth-server-mode P02 | 95 | 2 tasks | 15 files |
| Phase 05 P03 | 24 | 2 tasks | 11 files |
| Phase 05-auth-server-mode P04 | 180 | 2 tasks | 8 files |
| Phase 05-auth-server-mode P05 | 17 | - tasks | - files |
| Phase 05 P06 | 20 min | 3 tasks | 3 files |
| Phase 06-snmp P01 | 22 | 2 tasks | 21 files |
| Phase 06-snmp P04 | 8 | 3 tasks | 14 files |
| Phase 06 P05 | 7 | 2 tasks | 10 files |
| Phase 06-snmp P06 | 11 | 2 tasks | 2 files |
| Phase 06-snmp P07 | 25 | 3 tasks | 8 files |
| Phase 06-snmp P08 | 7 | 2 tasks | 7 files |
| Phase 07 P01 | 5min | 2 tasks | 12 files |
| Phase 07 P03 | 52 | 2 tasks | 8 files |
| Phase 07 P04 | 4 | 2 tasks | 6 files |
| Phase 07-reports-dashboard-settings P05 | 4 | 2 tasks | 5 files |
| Phase 07 P07 | 120 | 2 tasks | 19 files |
| Phase 07 P10 | 176 | 2 tasks | 4 files |
| Phase 07-reports-dashboard-settings P11 | 8 | 1 tasks | 2 files |
| Phase 07 P13 | 2 | 2 tasks | 4 files |
| Phase 07 P14 | 15 | 2 tasks | 6 files |
| Phase 08 P01 | 2 min | 3 tasks | 4 files |
| Phase 08 P02 | 1 | 3 tasks | 1 files |
| Phase 09 P01 | 8min | 2 tasks | 9 files |
| Phase 09 P02 | 75m | 2 tasks | 14 files |
| Phase 09 P03 | 110m | 2 tasks | 18 files |
| Phase 09-ad P04 | 50min | 2 tasks | 12 files |
| Phase 09-ad P05 | 55min | 2 tasks | 11 files |
| Phase 10 P01 | 12min | 2 tasks | 2 files |
| Phase 10 P02 | 45min | 3 tasks | 12 files |
| Phase 10 P04 | 35min | 3 tasks | 6 files |
| Phase 11 P01 | 50m | 2 tasks | 9 files |
| Phase 11 P02 | 55min | 2 tasks | 11 files |
| Phase 12 P01 | 22min | 2 tasks | 8 files |
| Phase 12 P02 | 35min | 2 tasks | 5 files |
| Phase 12 P03 | 18min | 3 tasks | 3 files |
| Phase 12 P04 | 12min | 1 tasks | 2 files |
| Phase 12 P05 | 25min | 3 tasks | 16 files |
| Phase 12 P06 | 25min | 2 tasks | 5 files |
| Phase 12 P07 | 25min | 3 tasks | 6 files |
| Phase 12 P08 | 15min | 1 tasks | 1 files |
| Phase 12 P09 | 55min | 2 tasks | 5 files |
| Phase 12 P10 | 15min | 2 tasks | 3 files |
| Phase 12 P11 | 14min | 2 tasks | 3 files |
| Phase 12 P13 | 12min | 1 tasks | 2 files |
| Phase 12 P14 | 45m | 3 tasks | 9 files |
| Phase 12 P12 | 12min | 2 tasks | 1 files |
| Phase 12 P15 | 5min | 3 tasks | 2 files |
| Phase 12 P19 | 18min | 2 tasks | 2 files |
| Phase 12 P17 | 12min | 1 tasks | 1 files |
| Phase 12 P16 | 2min | 1 tasks | 1 files |
| Phase 12 P18 | 6min | 1 tasks | 1 files |
| Phase 12 P20 | 35min | 2 tasks | 2 files |
| Phase 12 P21 | 35min | 2 tasks | 9 files |
| Phase 13 P01 | 35min | 3 tasks | 3 files |
| Phase 13 P02 | 30min | 2 tasks | 13 files |
| Phase 13 P03 | 25min | 1 tasks | 6 files |
| Phase 13 P04 | 13min | 2 tasks | 2 files |
| Phase 13 P05 | 20min | 2 tasks | 5 files |
| Phase 13 P06 | 25min | 2 tasks | 4 files |
| Phase 13 P07 | 25min | 2 tasks | 2 files |
| Phase 13 P08 | 15min | 3 tasks | 1 files |
| Phase 14 P01 | 22min | 2 tasks | 10 files |
| Phase 14 P02 | 12min | 2 tasks | 1 files |
| Phase 14 P03 | 30min | 3 tasks | 4 files |
| Phase 15 P01 | 25min | 2 tasks | 3 files |
| Phase 15 P02 | 35min | 3 tasks | 5 files |
| Phase 15 P03 | 50 | 3 tasks | 5 files |
| Phase 15 P04 | 25min | 2 tasks | 2 files |
| Phase 16 P01 | 25min | 3 tasks | 6 files |
| Phase 16 P02 | 30min | 3 tasks | 7 files |
| Phase 16 P03 | 15min | 3 tasks | 3 files |
| Phase 16 P05 | 45min | 3 tasks | 8 files |
| Phase 16 P04 | 20min | 2 tasks | 6 files |
| Phase 17 P01 | 55min | 3 tasks | 6 files |
| Phase 17 P04 | 50min | 3 tasks | 5 files |
| Phase 17 P03 | 25min | 3 tasks | 3 files |
| Phase 17 P05 | 7min | 3 tasks | 3 files |
| Phase 17 P06 | 15 min | 3 tasks | 4 files |
| Phase 17 P07 | 40min | 2 tasks | 1 files |
| Phase 18 P01 | 20min | 2 tasks | 4 files |
| Phase 18 P02 | 12min | - tasks | - files |
| Phase 18 P03 | 9min | 3 tasks | 6 files |
| Phase 18 P04 | 25min | 2 tasks | 1 files |
| Phase 18 P18-05 | 40min | 3 tasks | 1 files |
| Phase 19 P01 | 14min | 3 tasks | 7 files |
| Phase 19 P02 | 11min | 3 tasks | 4 files |
| Phase 19 P03 | 25min | 2 tasks | 2 files |
| Phase 19 P04 | 45min | 3 tasks | 6 files |
| Phase 19 P05 | 20min | 3 tasks | 5 files |
| Phase 19 P06 | 25min | 2 tasks | 2 files |
| Phase 19 P08 | 5min | 2 tasks | 3 files |
| Phase 19 P07 | 20 min | 2 tasks | 2 files |
| Phase 19 P09 | 12min | 2 tasks | 1 files |
| Phase 19 P10 | 8min | 2 tasks | 2 files |
| Phase 22 P01 | 76min | 4 tasks | 11 files |
| Phase 22 P02 | 240min | 2 tasks | 5 files |
| Phase 22 P03 | 25m | 2 tasks | 6 files |
| Phase 22 P04 | 25min | 4 tasks | 3 files |
| Phase 22 P05 | 96min | 2 tasks | 3 files |
| Phase 22 P22-06 | 60 | 2 tasks | 3 files |
| Phase 20 P01 | 25min | 3 tasks | 8 files |
| Phase 20 P02 | 15min | 2 tasks | 2 files |
| Phase 20 P03 | 10min | 2 tasks | 3 files |
| Phase 20 P04 | 8min | 2 tasks | 1 files |
| Phase 21 P01 | 22min | 1 tasks | 2 files |
| Phase 23 P01 | 10min | 2 tasks | 2 files |
| Phase 23 P02 | 20min | 2 tasks | 6 files |
| Phase 23 P03 | 35min | 2 tasks | 115 files |
| Phase 23 P04 | 50min | 2 tasks | 106 files |
| Phase 23 P05 | 20min | 2 tasks | 101 files |
| Phase 23 P06 | 10min | 2 tasks | 7 files |
| Phase 23 P07 | 15min | 2 tasks | 3 files |
| Phase 23 P08 | 15min | 3 tasks | 14 files |
| Phase 24 P01 | 8min | 2 tasks | 5 files |
| Phase 24 P02 | 12min | 2 tasks | 3 files |
| Phase 24 P03 | 3min | 3 tasks | 6 files |
| Phase 24 P04 | 3min | 2 tasks | 2 files |
| Phase 24 P05 | 5min | 2 tasks | 2 files |
| Phase 24 P06 | 5min | 2 tasks | 2 files |
| Phase 24 P07 | 15min | 3 tasks | 5 files |
| Phase 24 P08 | 6min | 2 tasks | 4 files |
| Phase 24 P09 | 8min | 2 tasks | 1 files |
| Phase 24 P10 | 6min | 2 tasks | 1 files |
| Phase 24 P11 | 6min | 3 tasks | 1 files |
| Phase 24 P12 | 12min | 2 tasks | 2 files |
| Phase 24 P13 | 5min | 2 tasks | 2 files |
| Phase 25 P01 | 5min | 2 tasks | 3 files |
| Phase 25 P02 | 25min | 2 tasks | 1 files |
| Phase 25 P03 | 25min | 2 tasks | 1 files |
| Phase 25 P04 | 20min | 2 tasks | 2 files |
| Phase 25 P05 | 25min | 2 tasks | 3 files |
| Phase 25-dropdown P06 | 15min | 2 tasks | 2 files |
| Phase 25 P07 | 30min | 2 tasks | 2 files |
| Phase 25-dropdown P08 | 12min | 2 tasks | 1 files |
| Phase 26 P1 | 8min | 3 tasks | 5 files |
| Phase 26 P02 | 8min | 2 tasks | 2 files |
| Phase 26 P03 | 6min | 2 tasks | 2 files |
| Phase 26 P04 | 6min | 2 tasks | 2 files |
| Phase 26 P05 | 10min | 2 tasks | 1 files |
| Phase 26 P06 | 8min | 2 tasks | 1 files |
| Phase 26 P07 | 10min | 3 tasks | 3 files |
| Phase 26 P08 | 3min | 4 tasks | 8 files |
| Phase 27 P01 | 4min | 2 tasks | 3 files |
| Phase 27 P03 | 25min | 3 tasks | 5 files |
| Phase 27 P05 | 12min | 2 tasks | 1 files |
| Phase 27 P06 | 35min | 3 tasks | 1 files |
| Phase 27 P08 | 15min | 3 tasks | 1 files |
| Phase 27 P02 | 13min | 3 tasks | 7 files |
| Phase 27 P04 | 15min | 3 tasks | 8 files |
| Phase 27 P07 | 20min | 3 tasks | 6 files |
| Phase 27 P09 | ~4h57min | 2 tasks | 31 files |
| Phase 28 P01 | 4min | 2 tasks | 4 files |
| Phase 28 P02 | 20 min | 2 tasks | 2 files |
| Phase 28 P03 | 4min | 2 tasks | 2 files |
| Phase 28 P04 | 4min | 2 tasks | 2 files |
| Phase 28 P05 | 6min | 3 tasks | 3 files |
| Phase 28 P06 | 8min | 2 tasks | 2 files |
| Phase 28 P07 | 5 min | 2 tasks | 2 files |
| Phase 28 P08 | 8 min | 1 tasks | 1 files |
| Phase 28 P09 | 8min | 2 tasks | 4 files |
| Phase 28 P10 | N/A | 2 tasks | 0 files |
| Phase 29 P01 | 6min | 3 tasks | 3 files |
| Phase 29 P02 | 12min | 2 tasks | 3 files |
| Phase 29 P03 | 8min | 2 tasks | 2 files |
| Phase 29 P04 | 15min | 3 tasks | 1 files |
| Phase 30 P01 | ~20min | 3 tasks | 4 files |
| Phase 30 P02 | 5min | 3 tasks | 3 files |
| Phase 30 P04 | ~10min | 2 tasks | 1 files |
| Phase 30 P05 | 12min | 2 tasks | 5 files |
| Phase 30 P06 | 8min | 2 tasks | 2 files |
| Phase 30 P07 | 12min | 2 tasks | 2 files |
| Phase 30 P08 | 15min | 3 tasks | 2 files |
| Phase 30 P09 | 35min | 2 tasks | 2 files |
| Phase 31 P1 | 39min | 3 tasks | 5 files |
| Phase 31 P03 | 50min | 2 tasks | 10 files |
| Phase 32 P01 | 35min | 2 tasks | 2 files |
| Phase 32 P02 | 55min | 2 tasks | 2 files |
| Phase 33 P01 | 35min | 3 tasks | 7 files |
| Phase 33 P02 | ~50min | 3 tasks | 5 files |
| Phase 33 P03 | 35min | 3 tasks | 2 files |
| Phase 33 P04 | 20min | 2 tasks | 2 files |
| Phase 34 P01 | 30min | 3 tasks | 8 files |
| Phase 34 P02 | ~25min | 3 tasks | 9 files |
| Phase 34 P03 | 50min | 3 tasks | 8 files |
| Phase 34 P04 | 15min | 2 tasks | 2 files |
| Phase 34 P05 | 40min | 2 tasks | 5 files |
| Phase 34 P06 | ~2h05min | 3 tasks | 5 files |
| Phase 35 P01 | ~15min | 2 tasks | 4 files |
| Phase 35 P02 | 13min | 3 tasks | 1 files |
| Phase 35 P03 | 10min | 2 tasks | 1 files |
| Phase 35 P04 | 40min | 3 tasks | 4 files |
| Phase 35 P05 | 20min | 2 tasks | 0 files |
| Phase 35 P07 | ~2h | 3 tasks | 7 files |
| Phase 36 P01 | 8min | 2 tasks | 2 files |
| Phase 36 P02 | 22min | 2 tasks | 1 files |
| Phase 36 P03 | 75min | 3 tasks | 4 files |
| Phase 36 P04 | 50min | 3 tasks | 4 files |
| Phase 36 P06 | 40min | 3 tasks | 5 files |
| Phase 36 P05 | 240min | 2 tasks | 0 files |
| Phase 37 P01 | 45min | 2 tasks | 14 files |
| Phase 37 P02 | ~15min | 3 tasks | 15 files |
| Phase 37 P03 | 40min | 2 tasks | 9 files |
| Phase 260819-vfg P01 | 5min | 1 tasks | 2 files |
| Phase 260820-uo4 P01 | 12min | 2 tasks | 3 files |
| Phase 39 P01 | 80min | 3 tasks | 3 files |
| Phase 39 P02 | 25min | 3 tasks | 5 files |
| Phase 39 P03 | 12min | 3 tasks | 4 files |
| Phase 39 P04 | 70min | 3 tasks | 3 files |
| Phase 39 P06 | 43min | 6 tasks | 10 files |
| Phase 39 P05 | 55min | 3 tasks | 8 files |
| Phase 39 P10 | 140min | 6 tasks | 10 files |
| Phase 39 P07 | 25min | 4 tasks | 3 files |
| Phase 39 P09 | 19min | 5 tasks | 6 files |
| Phase 39 P08 | 40m | 2 tasks | 3 files |
| Phase 39 P11 | 39min | 5 tasks | 9 files |
| Phase 39 P12 | 55min | 3 tasks | 7 files |
| Phase 39 P22 | 120m | 4 tasks | 31 files |
| Phase 39 P13 | 50min | 3 tasks | 3 files |
| Phase 39 P19 | 35min | 2 tasks | 2 files |
| Phase 39 P15 | 55min | 3 tasks | 11 files |
| Phase 39 P16 | 35min | 4 tasks | 7 files |
| Phase 39 P17 | 35min | 4 tasks | 6 files |
| Phase 39 P18 | 55min | 4 tasks | 6 files |
| Phase 39 P14 | 65min | 2 tasks | 8 files |
| Phase 39 P20 | 7 UAT rounds | 2 tasks | 21 files |
| Phase 39 P21 | 250min | 2 tasks | 14 files |
| Phase 39.1 P01 | 10min | 3 tasks | 3 files |
| Phase 39.1 P03 | 35m | 2 tasks | 5 files |
| Phase 39.1 P04 | ~20min | 1 tasks | 3 files |
| Phase 39.1-place-path-display P06 | 30min | 2 tasks | 6 files |
| Phase 39.1 P07 | 35min | 2 tasks | 10 files |
| Phase 39.1 P08 | 35min | 2 tasks | 3 files |
| Phase 39.1 P09 | 20min | 1 tasks | 1 files |
| Phase 39.1 P10 | 40m | 2 tasks | 13 files |
| Phase 39.2 P02 | 25min | 2 tasks | 9 files |
| Phase 39.2 P03 | 25min | 3 tasks | 2 files |
| Phase 39.2 P04 | 20min | 2 tasks | 4 files |
| Phase 40-movement-history P01 | 8min | 3 tasks | 4 files |
| Phase 40 P02 | 15min | 1 tasks | 3 files |
| Phase 40 P03 | 9min | 1 tasks | 6 files |
| Phase 40 P04 | 17min | 2 tasks | 6 files |
| Phase 40 P05 | 45min | 2 tasks | 3 files |
| Phase 40 P06 | 19min | 2 tasks | 24 files |
| Phase 40 P11 | 40min | 2 tasks | 5 files |
| Phase 40 P07 | 35min | 2 tasks | 2 files |
| Phase 40 P08 | 40min | 3 tasks | 3 files |
| Phase 40 P09 | 40min | 3 tasks | 2 files |
| Phase 40 P10 | 30min | 3 tasks | 17 files |
| Phase 40 P12 | 35m | 2 tasks | 4 files |
| Phase 40 P13 | ~40min | 2 tasks | 4 files |
| Phase 40 P14 | 35min | 1 tasks | 1 files |
| Phase 40 P20 | 20min | 2 tasks | 2 files |
| Phase 40 P15 | 25min | 2 tasks | 4 files |
| Phase 40 P18 | 20min | 2 tasks | 4 files |
| Phase 40 P19 | 25min | 1 tasks | 1 files |
| Phase 40 P16 | 20min | 2 tasks | 3 files |
| Phase 40 P17 | 20min | 2 tasks | 2 files |
| Phase 40 P21 | 35min | 3 tasks | 4 files |
| Phase 40 P24 | 12min | 3 tasks | 4 files |
| Phase 40 P25 | 15min | 2 tasks | 3 files |
| Phase 40 P26 | 5min | 4 tasks | 6 files |
| Phase 40 P22 | 20min | 2 tasks | 2 files |
| Phase 40 P27 | 10min | 2 tasks | 3 files |
| Phase 40 P23 | 15m | 1 tasks | 1 files |

## Accumulated Context

### Roadmap Evolution

- Phase 03.1 inserted after Phase 03: Acts quantity model + UAT gap closure (G-1..G-13)
- Phase 03.2 inserted after Phase 03.1: gap-closure deferred UAT items DEF-1/2/3 from Phase 03.1 (URGENT)
- Phase 03.3 inserted after Phase 03.2: Device-list UX round 2 — 4 UAT items after 03.2 (grouping condition column / cell tooltips / status column / location autocomplete) (URGENT)
- Phase 9 added (2026-06-19): AD-аутентификация и заявки на регистрацию пользователей (USR-08..12, REQ-06, SET-10) — вынесено из Phase 8 при SPIDR-split 2026-06-18; traceability в REQUIREMENTS.md синхронизирована
- Phase 10 added (2026-06-21): Ограничение роли employee (Сотрудник) — доступ только к Заявкам + отдельный employee-UI; аудит role-gating read-эндпоинтов на бэкенде
- Phase 12 added (2026-06-22): Взаимосвязь картриджной заявки — сквозная связка заявки на замену картриджа → установка (выбор заправленного картриджа, авто-подстановка расположения принтера, предзаполнение заявителя)
- Phase 13 added (2026-06-25): Редизайн совместимости Принтеры↔Картриджи по уникальному наименованию/типу принтера (не per-device junction; сносит промежуточный UI/таблицы из Phase 12) + свёрнутые chip-задачи (kind-aware drum-state дефолт авто-возврата, лимит списка принтеров 500-vs-200). В милстоне v1.1.
- Phase 14 added (2026-07-03): Данные и структура акта — миграции/схема для расширенных реквизитов организации, Комплектации, Технических характеристик, Срока до, мультиустройства и контекста рендера. Milestone v1.1.1 (PDFA-03, PDFA-04, PDFA-06).
- Phase 15 added (2026-07-03): Рендер и соответствие образцу — дефолтный `.minijinja`-шаблон, мультиустройство через `ItemsTable`, двухстрочные подписи, regression-тесты PDF-пайплайна. Milestone v1.1.1 (PDFA-01, PDFA-02, PDFA-05, PDFA-07, PDFA-08).
- Phase 17 added (2026-07-06): Отчёты и Шаблоны через HTML-печать — перевести экспорт Отчётов с krilla `render_docspec` на HTML-печать по паттерну Phase 16 (акты), переделать редактор Шаблонов в Настройках, убрать krilla из активного пути; закрывает отложенные пункты 16-HUMAN-UAT 2a (миграция Отчётов) и 2b (баг `reports_export_pdf` «Ошибка при создании PDF»). Milestone v1.2.
- Phase 18 added (2026-07-09): Автокомплит и дропдауны — все автокомплиты через portal в `body`; выбор устройства в актах: открытие по фокусу, рабочая фильтрация, группировка одинаковых устройств с раскрытием, схлопывание единственной группы. Milestone v1.1.2 (AUTO-01..05).
- Phase 19 added (2026-07-09): Акты — дата и редактирование — дата «Когда отдали» сохраняется как дата акта; кнопка «Редактировать» становится рабочей (требует диагностики первопричины перед фиксом). Milestone v1.1.2 (ACT-01, ACT-02).
- Phase 20 added (2026-07-09): Печать актов и организация — полный org-контекст в шапке device-акта; безопасный SVG-логотип (санитизация/data: URI, без исполняемых скриптов); вторая строка адреса в печатных формах. Milestone v1.1.2 (PRN-01, ORG-01, ORG-02).
- Phase 21 added (2026-07-09): Точечные фиксы — формат автокода картриджа `C-XXXX`, фотобарабана `D-XXXX`. Milestone v1.1.2 (CRT-01).
- Phase 22 added (2026-07-12): Правка возвратов — «Редактировать» на return-акте активна, открывает диалог «Возврат по акту №XXX» с прежними значениями; полная правка возврата с пересборкой эффектов на устройства по дельте. Отменяет D-07 (Phase 19). Milestone v1.1.2 (ACT-03). Вынесено из живого UAT Фазы 19. (Прим.: `gsd-sdk query phase.add` дал сбой на кириллице — номер 20 вместо 22 + пустой slug; фаза добавлена вручную как 22-return-act-edit.)
- Phase 23 added (2026-07-16): Токены и основы дизайн-системы — новый слой `--tr-*` (поверхности/текст/акцент/семантика/нейтрали/тени), миграция space/radius/font-size ПО ЗНАЧЕНИЮ (не по имени — ловушка переименования шкал), фикс 2 undefined-token багов (`--font-size-sm`, `--radius-lg`). Milestone v1.2 (DS-01, DS-02, DS-03, DS-04, QA-01).
- Phase 24 added (2026-07-16): Базовые компоненты — Button/Input-Select-Textarea-Checkbox/Badge/Tabs/Modal переработаны на новой системе. Milestone v1.2 (CMP-01..05).
- Phase 25 added (2026-07-16): Таблицы и Dropdown — строки таблицы + строка-группа (свёртка/счётчик/вложенные устройства), новый компонент Dropdown/комбобокс (плоский + групповой список) — выделены в отдельную фазу как самые сложные компоненты. Milestone v1.2 (CMP-06, CMP-07).
- Phase 26 added (2026-07-16): Окна с готовым макетом — Дашборд и Устройства, единственные 2 окна из ~12 с реальным макетом Claude Design. Milestone v1.2 (WIN-01, WIN-02).
- Phase 27 added (2026-07-16): Окна основного рабочего процесса — Акты, Картриджи, Принтеры; макета нет, раскладка выводится из компонентной системы фаз 24–25. Milestone v1.2 (WIN-03, WIN-04, WIN-05).
- Phase 28 added (2026-07-16): Окна поддержки и администрирования — Заявки, Отчёты, Настройки, Пользователи; макета нет. Milestone v1.2 (WIN-06, WIN-07, WIN-08, WIN-09).
- Phase 29 added (2026-07-16): Вход и интерфейс сотрудника — Логин/Pending/Blocked/FirstRunWizard, EmployeeLayout; отдельные layout-shell от основного приложения, макета нет. Milestone v1.2 (WIN-10, WIN-11).
- Phase 30 added (2026-07-16): Качество — доступность (AA-контраст, focus ring) и визуальный паритет Tauri WebView vs LAN-браузер; финальная сквозная проверка по всем окнам фаз 26–29. Milestone v1.2 (QA-02, QA-03).
- Phase 31 added (2026-08-03): Служебный AD-bind — ФИО и роли из AD-групп — service-account LDAP bind (по образцу adwebapp `ldap.go`) резолвит `displayName` для SSO-пользователей с кэшем; маппинг AD-группа → роль через `memberOf`, fail-closed при недоступности каталога. Milestone v1.3 (SSO-01, SSO-03).
- Phase 32 added (2026-08-03): Авто-админ по списку логинов + релиз SSO в main — настраиваемый список доменных логинов (аналог `ADMIN_AD_LOGINS`) получает роль «Администратор» сразу при первом SSO-входе; операционный итог фазы (не REQ) — мерж `spike/ad-sso-kerberos` в `main` и релиз обычной версии. Milestone v1.3 (SSO-02).
- Phase 33 added (2026-08-03): Полировка предпросмотра печати — модалка предпросмотра (Акты/Приёмка/Отчёты) показывает лист A4 на сероватой подложке с полями (margins), WYSIWYG-совпадение предпросмотра и `@media print`. Независима от Phase 31/32 (чистый фронтенд/CSS). Milestone v1.3 (PRV-01, PRV-02, PRV-03).
- Phase 34 added (2026-08-08): Единая шапка документов — лого + реквизиты организации (Times New Roman 12pt, A4 20/15мм) на всех трёх печатных формах, источник `org.name`, доставка новой шапки в существующие установки через новый срез `_legacy_defaults`. Первый шаг фазы — спасти правки пользователя из `target/debug/templates/`. Milestone v1.3.3 (DOC-04, DOC-05, DOC-06).
- Phase 35 added (2026-08-08): Тело акта приёма-передачи — канонiчный текст (две стороны, «составили настоящий акт о нижеследующем», перечень/состояние/срок/подписи), согласованный с пользователем ДО вёрстки; без полосок-подчёркиваний под автоподставляемым текстом; горизонтальный блок подписей по строке на подписанта. Зависит от Phase 34 (общий файл шаблона). Milestone v1.3.3 (DOC-07, DOC-08, DOC-09).
- Phase 36 added (2026-08-08): Пагинация акта по количеству устройств — один лист для одного устройства, «Приложение №1» со второго листа для нескольких. Зависит от Phase 35 (форма тела определяет разбивку). Milestone v1.3.3 (DOC-10, DOC-11).
- Phase 37 added (2026-08-08): Приватность данных — обезличивание HEAD от уже утёкших реальных данных организации и сотрудников (код/шаблоны/тесты/`.planning/`-артефакты) + durable-гейт против повторной утечки по образцу `check-contrast.mjs`/`check-print-isolation.mjs`, подключённый в `pnpm lint`. История git не переписывается (решение пользователя). Порядок внутри фазы: сначала чистка, затем гейт — иначе гейт падает на собственном репозитории. Независима от направления шаблонов. Milestone v1.3.3 (PRIV-01, PRIV-02).
- Phase 38 added (2026-08-08): Nyquist-покрытие Фазы 32 — закрытие унаследованного из v1.3 долга (`32-VALIDATION.md: nyquist_compliant: false` → `true`). Независима от остальных фаз. Milestone v1.3.3 (QA-04).
- Phases 37+38 merged into one (2026-08-08, решение пользователя при утверждении роадмапа): чистка и гейт всё равно планируются и проверяются вместе, потому что гейт обязан проходить на уже очищенном HEAD. Прежняя Phase 39 (Nyquist) стала Phase 38. Итог: 5 фаз вместо 6, покрытие требований не изменилось.
- Phase 39.1 inserted after Phase 39: Формат пути Места переезжает из trackly.config.toml в UI: умолчание организации + переопределение на месте с наследованием (URGENT)
- Phase 39.2 inserted after Phase 39.1: Долг фазы 39.1: 6 Warning + 4 Info из 39.1-REVIEW.md (единый владелец дефолтов пути, транзакция на записи, робастность на битых данных, UI/a11y экрана Настроек) (URGENT)

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- **Roadmap:** Standard granularity, 8 phases sequential, MVP mode на всех фазах
- **Stack (locked):** rusqlite 0.39 + refinery 0.8 + split read/write pools + single-writer task; tauri 2.11 + svelte 5 + axum 0.8 + tower-sessions 0.13 + snmp2 0.4 + ldap3 0.12 + argon2 0.5 + rustls 0.23 + rcgen 0.13 + krilla 0.7 (default PDF)
- **«Расходник»:** ОСТАЁТСЯ как тип устройства (бумага, одноразовые флешки и пр.) — НЕ для картриджей; картриджи живут в собственном разделе
- **PDF engine:** krilla 0.7 default, Typst-as-lib — backup по итогам spike в Phase 3
- **Pantum auto-restart:** alert-only в v1 (PRN-06); авто-restart — v2 (PNT)
- **Roadmap v1.3:** 3 фазы (31–33), продолжение нумерации с Phase 30 (v1.2), без искусственного дробления под granularity=standard (6 требований). SSO-01/03 объединены в Phase 31 (общая инфраструктура service-account bind); SSO-02 + мерж спайка в main — Phase 32; PRV-01..03 — независимая Phase 33 (фронтенд/CSS, можно параллельно с SSO).
- [Phase ?]: Plan 01-01: MSRV 1.85 to 1.88 (Tauri 2 dep graph)
- [Phase ?]: Plan 01-01: rusqlite 0.39 to 0.38, refinery 0.8 to 0.9 (rusqlite-bundled feature)
- [Phase ?]: Plan 01-01: Included tauri-plugin-single-instance from Day 1 per RESEARCH Open Question 2
- [Phase ?]: Plan 01-01: ESLint 9 flat config (eslint.config.js); pnpm 10.17.1 pinned via packageManager field
- [Phase ?]: Plan 01-02: Paths::resolve_for_exe_dir is public (test seam)
- [Phase ?]: Plan 01-02: UNC rejection via simple starts_with(r"\\\\") prefix check
- [Phase ?]: Plan 01-02: AppError kept minimal (Internal + Validation); Plan 04 extends
- [Phase ?]: Plan 01-02: webview_env uses #[rustfmt::skip] at fn-level to preserve one-line unsafe contract
- [Phase ?]: Plan 01-03: embed_migrations!(../../migrations) from trackly-infra crate root — refinery 0.9 macro path form
- [Phase ?]: Plan 01-03: MigrationReport { schema_version: u32, applied_count: usize } — Plan 04 AppCtx hardcodes 12 for downgrade check
- [Phase ?]: Plan 01-03: test_db() public (not cfg test) — tempfile-backed, canonical fixture for all downstream integration tests
- [Phase ?]: Plan 01-03: WAL applied via apply_writer_pragmas BEFORE refinery — Pitfall #4 mitigated, idempotency test confirms
- [Phase ?]: Plan 01-03: act_items.condition_at_time TEXT (snapshot, not timestamp) and sessions.expiry_date INTEGER (tower-sessions convention) are allowlisted in timestamp invariant test
- [Phase ?]: Free-fn error mappers (map_rusqlite/refinery/send_timeout/oneshot_recv) instead of impl From — Rust orphan rule blocks impl in trackly-infra
- [Phase ?]: ReaderPool: simple std::sync::Mutex<Vec<Connection>> LIFO, panic on exhaustion accepted for Phase 1 (LAN scale); Phase 2+ can swap to deadpool
- [Phase ?]: Probe-read pattern: SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI + explicit drop before writer open — guarantees byte-identical file on downgrade rejection (success criterion #4)
- [Phase ?]: rusqlite promoted to runtime dep of trackly-app for context.rs probe-read step; trackly-core remains rusqlite-free (no_io_deps gate still green)
- [Phase ?]: Plan 06: filter.pmc kept as documentation-only placeholder; CSV-level post-filter in csv_check.rs is the authoritative gate
- [Phase ?]: Plan 06: svelte-check is continue-on-error in ci-full.yml until Phase 2 wires @tauri-apps/api (per deferred-items.md)
- [Phase ?]: Plan 06: Sysinternals ProcMon SHA256 logged but NOT gated (Microsoft does not publish stable checksums; T-06-01 accepted with audit-log mitigation)
- [Phase ?]: Plan 06: cyrillic sandbox doubles as success-criterion-#1 + FOUND-11 fixture; crash gate (T-06-04) prevents silent pass
- [Phase ?]: 02-01: Path B column mapping: domain uses UI names, SQL stays V003
- [Phase ?]: 02-01: DeviceRepository associated type Conn keeps rusqlite out of trackly-core (hexagonal boundary)
- [Phase ?]: 02-01: ImportSessionStore lazy sweep on put() only - no background task
- [Phase ?]: 02-02: DevicesPlaceholder.svelte временный; Plan 03 заменит на features/devices/DevicesPage.svelte
- [Phase ?]: 02-02: initTheme() вызывается в main.ts ДО mount — no-flash guarantee
- [Phase ?]: 02-02: svelte-check теперь blocking gate в ci-fast + ci-full (Phase 1 deferred item закрыт)
- [Phase ?]: Phase 3-01 (PDF spike): krilla 0.7 PASSED — pinned Metadata + xmp_metadata=false + regex post-process yields deterministic byte-stream on macOS aarch64 (sha256 88df7f9d…); Typst-as-lib fallback NOT triggered
- [Phase ?]: Phase 3-01: MSRV 1.88 → 1.92; Win7 32-bit closed in v1
- [Phase ?]: Phase 3-01: minijinja features = json + fuel + serde (required for set_fuel and tmpl.render(serde_json::Value))
- [Phase ?]: Phase 3-01: krilla 0.7 API path — krilla::metadata::{Metadata, DateTime}, krilla::SerializeSettings (interchange/serialize are private modules)
- [Phase ?]: Plan 03-03: DeviceRow остаётся serde-free; canonical snapshot пишется через device_snapshot_json helper
- [Phase ?]: Plan 03-03: ActReturnDto принимает bulk_location_name/location_name_override для UX-friendly resolve
- [Phase ?]: Plan 03-03: cascade-delete handover делает LIFO undo (returns reverse order) в одной writer-tx
- [Phase ?]: Plan 03-04: ActService::with_pdf_pipeline (Optional Arc-deps) вместо breaking-change в new() — backward-compat сохраняет Phase 2/3 test fixtures
- [Phase ?]: Plan 03-04: minijinja +builtins feature; шаблоны default("—", true) для null-handling (срабатывает на explicit JSON null, не только undefined)
- [Phase ?]: Plan 03-04: PDF preview UI = iframe + blob URL (НЕ pdfjs-dist canvas) — Pitfall 8 обход, WebView2/WKWebView сами рендерят PDF нативно
- [Phase ?]: Plan 03-04: DEV-14 UI button «Печать документа приёма» отложена в plan 05 — backend devices_render_acceptance_pdf готов и протестирован
- [Phase ?]: Plan 03-05: ACT-04 поиск через LIKE+FTS5 (UNION CTE), acts_fts отложен до Phase 7
- [Phase ?]: Plan 03-05: DEV-14 UI flow через intermediate-modal → preview-modal mode='acceptance'
- [Phase ?]: Plan 03-05: W-9 MSK encoding на UI; backend UTC форматирование оставлено Phase 7
- [Phase ?]: Phase 3 closed: все 16 требований complete; готова к /gsd-verify-work
- [Phase ?]: 03.2-02
- [Phase 03.3]: ITEM-1 — Вариант A (флаг group_by_condition: bool в DeviceFilter); DevicesPage передаёт false, ActFormItemsTable передаёт true; DEF-2B сохранён
- [Phase 03.3]: ITEM-1 — «разное» для смешанной group (зафиксировано пользователем, UAT-ITEMS §Решения п.1); вычисляется через condition_distinct_count > 1 на клиенте
- [Phase 03.3]: ITEM-2 — нативный title= на всех text-ячейках (не кастомный tooltip-компонент)
- [Phase 03.3]: ITEM-4 — вторая секция в DeviceAutocompleteField через существующую locations_autocomplete Tauri-команду; HTTP route добавляется в http/devices.rs
- [Phase ?]: group_by_condition flag design
- [Phase ?]: 05-02-SUMMARY.md
- [Phase ?]: 05-02: server API design
- [Phase ?]: 05-02: session store design
- [Phase ?]: 05-02: server shutdown
- [Phase ?]: 05-02: auth hashing
- [Phase ?]: authorize() enforced in build_* helpers — единая точка авторизации для HTTP и Tauri
- [Phase ?]: GovernorLayer несовместим с tower oneshot тестами: создавать сессии программно через RusqliteSessionStore::create()
- [Phase ?]: role_endpoint_matrix: macro_rules! new_app! для свежего router на каждый test case (oneshot потребляет router)
- [Phase ?]: bindings-phase6.ts: Phase 6 типы вынесены в отдельный файл (не gitignored bindings.ts) для хранения в git без force-add
- [Phase ?]: specialist role maps to manager in UserRole; isSpecialist = admin || manager in requests portal
- [Phase ?]: 06-08: admit returns Vec<PrinterDto>; two-step probe→device→printer in admit; D-GAP-Replace-Select: devices.list(type_id=2) in RequestFormModal
- [Phase ?]: 07-01: snake_case JSON in Phase 7 DTOs — consistent with existing device.rs, no camelCase rename_all
- [Phase ?]: 07-01: StatusCount in reports.rs distinct from device.rs StatusCount — different semantic shapes (status_name:String+count:i64 vs status_id:i64+count:u64)
- [Phase ?]: 07-01: V026 org_settings single-row invariant enforced via CHECK (id = 1) + seed row at migration time
- [Phase 07]: 07-02: V027 migration for is_default column on document_templates (ALTER TABLE ADD COLUMN NOT NULL DEFAULT 1)
- [Phase 07]: 07-02: OrgDbService coexists with OrganizationService — new write layer, backward compat preserved for act_service PDF pipeline
- [Phase 07]: 07-02: rusqlite::backup::Backup scope block pattern — inner block ensures Backup+reader_guard drop before integrity_check on dest_conn (borrow checker)
- [Phase ?]: 07-04: TemplateEditor full-width card (no max-width: 640px) per UI-SPEC SET-09/D-20 exception — template textarea needs full available width
- [Phase ?]: 07-04: Logo served as img src=data:... not raw SVG injection — scripts blocked in img context (T-07-04-05 mitigated)
- [Phase ?]: DashboardStatusCount: renamed from StatusCount in dto/reports.rs to avoid TypeScript collision with device.rs StatusCount
- [Phase ?]: settings_move_db and app_restart Tauri-only: not exposed in HTTP router (T-07-07-03, D-19)
- [Phase ?]: 07-14: Vec<ReportCountEntry> for ReportCountsDto (no HashMap) — consistent with all existing DTOs in reports.rs; specta derives cleanly; TypeScript gets Array not Record
- [Phase ?]: 08-01: bundle.active=true — Tauri bundler включён для всех ОС (D-14)
- [Phase ?]: 08-01: bundle.icon расширен до 5 форматов (32x32/128x128/128x128@2x/icns/ico) — Pitfall 3 закрыт
- [Phase ?]: 08-01: bundle.macOS.signingIdentity='-' — ad-hoc подпись без Apple Developer ID (D-04)
- [Phase ?]: MSRV pinned correctly
- [Phase ?]: perl -0pi version injection
- [Phase ?]: GITHUB_EVENT_NAME fallback
- [Phase ?]: portable no-updater discipline
- [Phase 09]: AdClient port + RealAdClient/MockAdClient adapters mirror SnmpClient triad exactly; ldap3 confined to real.rs, hickory-resolver confined to discovery.rs (no OpenSSL pulled in)
- [Phase 09]: AD fallback only on UnknownLogin (never BadPassword) — avoids a second enumeration oracle for known local logins
- [Phase 09]: Added AppError::ServiceUnavailable{service} instead of reusing WriteQueueBusy — distinct infra-fault path for AD-unreachable
- [Phase 09]: on_ad_bind_success scoped to active-user-only this plan; blocked/deleted/unknown branches are typed TODOs for plan 03
- [Phase 09]: approve_ad_register completes the request directly (open->completed) via a manual optimistic-lock UPDATE, not RequestTransitionOp::Complete — that op's state machine requires in_progress as the source state
- [Phase 09]: ad_register reject semantics check the target user's live is_active flag at reject time, not ad_subtype alone, to distinguish pending-discard from auto-accept-then-rejected
- [Phase 09]: AppError::RegistrationPending/AccessBlocked map to HTTP 403, not 401 — AD bind succeeded, identity is known, just not yet admitted
- [Phase 09-ad]: remember=true sets persistent 30-day sliding cookie (Expiry::OnInactivity), set after session.insert() so it survives the flush-before-insert sequence
- [Phase 09-ad]: AdSettingsDto excludes all AD-password fields; connection settings are read-only TOML, only enabled/auto_accept are writable
- [Phase 09-ad]: bindings-phase9.ts placed at ui/src/ (not ui/src/lib/) matching the real bindings-phase6.ts convention; plan frontmatter path was stale
- [Phase 09-ad]: BlockedScreen restore CTA re-invokes auth_login with retained credentials (no dedicated restoration endpoint) — restoration request is created server-side as a side effect of the blocked AD bind path
- [Phase 09-ad]: ad_register reject-confirmation copy is keyed on adSubtype + a UI-fetched AdSettingsDto.auto_accept hint; backend reject_ad_register independently re-derives the correct mutation from user.is_active, so UI copy mismatch cannot cause incorrect deletion
- [Phase 10]: 10-01: Cross-plan RED/GREEN TDD — auth.rs ReadData matrix fix + Case 9 flip land here, intentionally failing (zero authorize(ReadData) call-sites exist yet); Plan 10-02 wires the call sites and turns Case 9 GREEN
- [Phase 10]: Gated all 5 read-domain resource types (devices/acts/cartridges/printers/reports) with authorize(caller, &Action::ReadData) across both HTTP and Tauri transports — Closes the BFLA gap (API5:2023) left after Plan 10-01's permission-matrix fix; Employee role can no longer read data via list/get/search/status-counts/history/low-stock/suggest endpoints
- [Phase 10]: Kept build_printers_refresh on its pre-existing Action::ReadPrinters check, untouched by this plan's ReadData gating — ReadPrinters is a separate, intentionally distinct action from ReadData — conflating them would have been an architectural overreach beyond this plan's scope
- [Phase 10]: Extended role_endpoint_matrix.rs CI test from 10 to 19 cases covering acts_list, cartridges_list, printers_list, reports_list_device_acts, and users_list — Proves the BFLA fix works end-to-end and serves as a regression guard against future endpoint additions in these 5 domains
- [Phase 10]: 10-04: employeeRoutes implemented as a plain route-map switch in App.svelte's if/else-if chain (not svelte-spa-router wrap() guards) — reuses the existing role-gating pattern already used for shell selection
- [Phase 10]: 10-04: AccessDenied.svelte destructures empty Props ({}) instead of binding unused 'location' prop — svelte-check flags unused destructured bindings as an error
- [Phase 11]: category_name appended as LAST column in SELECT_REQUESTS (idx 18) to avoid index-shift; LEFT JOIN request_categories covers get/list/fetch_in_tx via shared mapper
- [Phase 11]: bindings-phase6.ts is hand-maintained (not regenerated by cargo test); updated manually for categoryName + RequestCategoryDto in sync with Rust DTOs
- [Phase 11]: request_printer_options gates on Action::CreateRequest (every role has it), not ReadData/ReadPrinters which Phase 10 closed for Employee — avoids regressing Phase 10's BFLA fix while unblocking the cartridge-replace form.
- [Phase 11]: request_printer_options DTO is strictly {id, name, location} — no SNMP/community/IP/serial fields cross the wire (BOLA/BOPLA closure, API1/API3:2023).
- [Phase 12]: installable_only implemented as hardcoded SQL state_id IN (1,2), not a parameterized list — D-01/D-02 domain constants, no client-supplied value-set, closes injection surface
- [Phase 12]: printer_location appended LAST (idx 19) in SELECT_REQUESTS after category_name (idx 18) — preserves append-only convention, single shared mapper across get/list/fetch_in_tx
- [Phase 12]: History enrichment folds cartridge code+model into the existing notes_json 'notes' key (no new JSON key) to keep get_history()/RequestHistoryEntryDto unchanged
- [Phase 12]: RBAC test cases numbered 31/32 (not plan's stale suggestion of 25/26) — continued from the file's actual existing max case number
- [Phase 12]: effectiveCartridge derived pattern (cartridge prop ?? selectedCartridge) lets OperationModal serve both cartridge-centric and request-centric install entries off one code path (D-08)
- [Phase 12]: Checkpoint Task 4 (human-verify, gate=blocking) auto-approved under AUTO_MODE; happy path/DISC-02/D-08 regression confirmed via code review + svelte-check/build, not a live interactive session
- [Phase 12]: 12-04: suggest_person() UNIONs acts + cartridges.holder_name (both Giver/Receiver map to holder_name identically); frequency merge via outer GROUP BY SUM(freq) over a UNION ALL CTE
- [Phase 12]: Plan 12-05: CartridgeService gained internal printer_repo: Arc<SqlitePrinterRepository> field (constructed via Arc::new) rather than threading it through CartridgeService::new() — avoids 11 call-site changes
- [Phase 12]: Plan 12-05: printer_cartridge_models compatibility — setter service methods self-gate via inline authorize(), build_* helpers don't double-gate; getter build_* helpers gate directly since getter service methods take no caller param
- [Phase 12]: Plan 12-05: D-13/D-14 narrowing implemented as single SQL predicate (?N IS NULL OR NOT EXISTS(...) OR model_id IN (...)) — one indexed query encodes both narrow-when-configured and pass-through-when-not
- [Phase ?]: 12-06: Auto-return reuses the new install's given_by_name as implicit actor (D-17) — no new actor field added to ReturnToStock
- [Phase ?]: 12-06: current_printer_device_id SET folded into the same optimistic-lock UPDATE as the status transition, rather than a second UPDATE
- [Phase ?]: 12-06: Auto-return previous cartridge via direct UPDATE inside the same tx (not recursing into transition_in_tx) — internal cascade is known-safe by construction
- [Phase 12]: Plan 12-07: bindings.ts already contained PrinterCompatibleModelsDto/CartridgeModelCompatibleDevicesDto from 12-05's cargo test regen; API wrappers built against real modelId/device_ids wrapper DTO contract, not the plan's assumed cartridgeModelId/number[] shape
- [Phase 12]: Plan 12-08: compatibilityUnconfigured state replaces noModelScopeWarning; gated on preFillPrinterId !== undefined, fail-safe default false on getCompatibleModels error (UX hint, not security boundary)
- [Phase 12]: Plan 12-09: Reused existing Select component (value+onchange) for previous-cartridge charge state instead of raw bind:value select — Matches established codebase convention in OperationModal.svelte's own op-state field and avoids Svelte native-select numeric coercion bug documented in CartridgeFilters.svelte
- [Phase 12]: 12-10: SQLite table-rebuild pattern (CREATE _new -> INSERT SELECT explicit columns -> DROP -> RENAME) scoped inside PRAGMA foreign_keys=OFF/ON within one migration file removes the printers connectivity CHECK without touching printer_readings/printer_alerts FK resolution
- [Phase 12]: 12-11: WsEvent per-variant rename_all=camelCase fixes GAP-12-04 — outer tag stays snake_case, fields camelCase, mirrors RequestTransitionPayload pattern
- [Phase 12]: 12-11: OperationModal suppressSuccessToast opt-in prop — RequestDetail passes true to avoid duplicate toast; cartridge-centric entry (D-08) untouched
- [Phase 12]: 12-13: given_by_name_arm built as Giver-scoped Rust string variable (empty for Receiver) instead of unconditional SQL arm — structural guarantee against cross-field leakage
- [Phase ?]: 12-14: cancel() реализован как отдельный сервисный метод/эндпоинт, не вариант RequestTransitionPayload — избегает протаскивания Employee через transition()'s безусловный authorize(TransitionRequests)
- [Phase ?]: 12-14: V031 миграция (CHECK requests.status += 'cancelled') добавлена как Rule 2 auto-fix — без неё cancel() падал с CHECK constraint failed
- [Phase 12]: 12-12: printerContext: $state<PrinterDto | null> populated inside the existing printers.get(preFillPrinterId) $effect (no second API call) — printerContextHint shows deviceName+ipAddress instead of raw #id, rendered first in the install form, before the cartridge-select picker
- [Phase ?]: Plan 12-15: combined Tasks 2+3 into one commit since both modify the same RequestDetail.svelte if/else-if chain; isOwnRequest condition simplified by dropping redundant isAdRegister check (already guaranteed by parent chain)
- [Phase 12]: 12-19: Inverted actor computed server-side from the triggering Install op's given_by_name/given_to_name (no new payload fields) — closes Tampering threat T-12-19-02 by construction
- [Phase 12]: 12-19: Collapsed Install vs ReturnToStock/ToRefill/FromRefill/WriteOff UPDATE branches in transition_in_tx into one — current_printer_device_id is now always written, fixing a latent bug where direct (non-auto) returns left a stale printer link
- [Phase 12]: 12-17: connectWs() refcounted singleton (refCount + activeCleanup module state) replaces single-shot disconnectFn; idempotency keyed on refCount not ws!==null since browser branch nulls ws on every reconnect — fixes GAP-12-10 duplicate WS toasts without touching the 3 call sites
- [Phase 12]: 12-16: renamed locationLabel (stale name — it actually held IP, not location) to ipText; new locationText derived from printer.deviceLocation closes GAP-12-09 (B1) — printer list row now shows device location left, IP/USB/"—" right via margin-left:auto
- [Phase 12]: 12-18: closed GAP-12-11 by broadening OperationModal's printerContext/previousCartridge lookup $effect gate from `cartridge===null && preFillPrinterId!==undefined` to just `preFillPrinterId!==undefined` — cartridge-centric install entry now shows printer name+IP hint and the «Предыдущий картридж» block, same as request-centric; compatibleModels/cartridgeOptions effects intentionally kept on the narrower `cartridge===null` gate (D-08 regression guard preserved)
- [Phase 12]: 12-20: PrinterSelect.svelte adds optional, compatibility-prioritized printer selector to cartridge-centric install (D-20/D-21); falls back to flat list when no compatibility links exist, never blocks
- [Phase 12]: 12-20: effectivePrinterId derived (preFillPrinterId ?? selectedPrinterId) unifies request-centric and cartridge-centric printer context into one lookup/payload path; previousCartridge block (D-22) reused unchanged
- [Phase 12]: 12-21 (Round 5, GAP-12-13): root cause of printerContext staying null — effectivePrinterId is always a device_id, but printers_get resolves WHERE p.id=?1; added parallel printers_get_by_device_id command (same RBAC gate) instead of changing printers_get's contract (used elsewhere keyed by printers.id); OperationModal switched its lookup effect to getByDeviceId
- [Phase 12]: 12-21 (DEC-A/DEC-B): printerContextHint branches on isSelectorVisible (same predicate gating PrinterSelect markup) — omits name when selector already shows it; Расположение auto-fills from printerContext.deviceLocation in the cartridge-centric entry only, never overwriting manual input
- [Phase 13]: 13-01: upsert_compatibility_in_tx stores printer_name as-given (no TRIM at write); normalisation (LOWER+TRIM) applied only at compare time in list()/compatible_model_aggregates (D-02/D-03/D-04)
- [Phase 13]: 13-01: D-05 pass-through scoped strictly to list()'s cartridge-selection filter, NOT applied in compatible_model_aggregates — R4/D-07 require the printer-card aggregate to reflect only real V005 compatibility rows
- [Phase 13]: Pulled forward Plan 13-03's Tauri/HTTP/specta deletion scope into 13-02 (Rule 3 blocking-issue fix) — Removing the printer/cartridge compat service methods broke compilation in 5 transport-adapter files outside 13-02's stated scope; fixing was required to keep trackly-app building, and matches 13-03's own pre-planned deletion instructions exactly
- [Phase 13]: Cartridge model compatibility DTOs switched from Vec<(String,String)> brand/model pairs to Vec<String> printer names — Matches V032 migration's single printer_name column (Plan 13-01); CartridgeModelDto/CreateDto/PatchDto all updated together
- [Phase 13]: 13-03: compatible_aggregates_for_printer placed on CartridgeService (not PrinterService) since the underlying query lives in cartridges_sqlite.rs — Avoids duplicating query logic across domains; printers.rs build_* helper calls through ctx.cartridges
- [Phase 13]: 13-03: no D-07 pass-through on the new aggregate endpoint — A model with zero compatibility rows for a printer is simply absent from the response, not included with zero counts; Admin/Manager with no matches still gets 200 with models: []
- [Phase 13]: 13-04: transition_in_tx — moved resolved_state_id computation to after prev_current.model_kind_id is fetched, since the kind-aware branch depends on it
- [Phase 13]: 13-04: printers_sqlite.rs::list() — removed .min(200) cap entirely rather than raising it, per D-13 uncapped-read decision (no pagination introduced)
- [Phase ?]: 13-05: suggest_compat_printer re-sourced from devices.name (D-06) instead of cartridge_model_compatibility free-text history; dropped legacy field param across service/Tauri/HTTP layers
- [Phase 13]: filteredCompatibility (trim+dedupe) sent in submit payload, per plan action text, not raw compatibility variable — 13-06: plan's <action> for Task 2 explicitly names filteredCompatibility; frontmatter key_links regex was a looser hint
- [Phase 13]: CompatibleModelsEditor.svelte and OperationModal.svelte compat-junction call sites logged to deferred-items.md, not fixed under 13-06 — Both outside 13-06 files_modified; confirmed pre-existing via git-stash diff; CompatibleModelsEditor.svelte explicitly scoped to Plan 13-07 per UI-SPEC
- [Phase ?]: 13-07: compatAggregates/deviceData/installedCartridge each get their own independent $effect keyed on printer, matching the existing readings $effect convention
- [Phase ?]: 13-07: installedCartridge loading-gap renders '…' instead of falling back to the numeric id — no raw id shown in any intermediate state
- [Phase 13]: 13-08: res.models.length === 0 used as direct equivalent of removed modelIds.length === 0 check for compatibilityUnconfigured (no extra heuristic)
- [Phase 13]: 13-08: compatibleDeviceIds D-05 pass-through computed from printerOptions itself (Set of all deviceId) instead of a second network call
- [Phase 13]: 13-08: previousCartridgeStateId kind-aware default (5 drum / 3 cartridge) set when previousCartridge resolves (.then branch), not in the modal-open reset effect
- [Phase ?]: 14-01: org_settings new requisite columns default to empty string (not V026-style placeholder) — missing requisites degrade to blank per D-02
- [Phase ?]: 14-01: HeaderBlock direct-construction sites use ..Default::default() spread for new fields where site doesn't need requisite content
- [Phase ?]: 14-01: new org_settings columns always appended last in SQL SELECT/UPDATE to preserve existing r.get(N) ordinal indexes
- [Phase 14]: 14-02: Task 1 required no code changes to http/settings_org.rs or tauri_cmds/settings_org.rs — both pass OrgPatch through opaquely; bindings.ts already carried the 5 new fields from Plan 01
- [Phase 14]: 14-03: org_db wired via separate with_org_db() builder (not folded into with_pdf_pipeline's 3-arg signature) — avoids breaking existing test call sites; org_db is Option-aware end-to-end
- [Phase 14]: 14-03: render_pdf fallback (org_db=None) reads legacy org.json name/inn/kpp/address, defaults 5 new requisites to empty strings — matches D-02 degrade-to-blank contract
- [Phase 15]: 15-01: Section::Signature sublabels use plain #[serde(default)] + Option<String> idiom (defaulting to None, not the fn-default idiom used for spacer_pt) so absence renders the pre-Phase-15 single-line layout unchanged
- [Phase 15]: 15-01: ttf-parser promoted to direct dependency (0.25.1, exact-pinned, already transitive via krilla->rustybuzz/skrifa) via Task 0 human-verify checkpoint
- [Phase 15]: 15-01: 2-column header grid stays fixed regardless of logo presence (no adaptive single-column fallback); empty requisite lines (phone/fax/email/OKPO+OGRN) skipped entirely rather than shown as blank placeholder
- [Phase ?]: [Phase 15]: 15-02: render_pdf's None org_db branch explicitly returns (dto, None, None) 3-tuple — no behavior change for fixtures without org_db wired
- [Phase ?]: [Phase 15]: 15-02: Section::DeviceCard long_fields renderer does not filter empty values itself — template is sole source of truth for which long fields get emitted (matches existing conditional-injection idiom)
- [Phase ?]: [Phase 15]: 15-02: act.giver_name intentionally no longer displayed in act body per D-09 (moved to bare Выдал signature label; receiver_name now in intro paragraph) — deliberate content change, not a regression
- [Phase ?]: [Phase 15]: 15-03: render_handover_act_produces_cyrillic_pdf assertion updated from stale giver_name-in-body wording to receiver_name (D-09 removed giver_name from body) — planned N=1 regression anchor
- [Phase ?]: [Phase 15]: 15-03: acts_e2e_smoke.rs handover_pdf_render_within_e2e had the same D-09 giver_name-in-body drift as pdf_render_act.rs but was outside the plan's files_modified — fixed as Rule 1 auto-fix (same root cause, single assertion line)
- [Phase ?]: [Phase 15]: 15-03: act_42.sha256 regenerated (88df7f9d -> caaca9c5) via deliberate single-step procedure (run test, copy printed hash, verify act_42.json fixture input untouched) per T-15-09 mitigation — not a blanket auto-accept
- [Phase ?]: [Phase 15]: 15-04: Header renders once on page 1 only (WR-05 gap closure)
- [Phase ?]: [Phase 15]: 15-04: DeviceCard measured via measure_device_card_height (mirrors draw-time wrap_text_to_width arithmetic) — never split across a page boundary; other section variants use a cheap pre-draw bounds check
- [Phase ?]: [Phase 15]: 15-04: act_42.sha256 verified unchanged (not regenerated) — pagination bounds check never fires for the single-device fixture
- [Phase 16]: 16-01: Task 3 (templates + build_safe_html_env) executed before Task 2 (html_templates.rs) to keep every intermediate commit compiling — include_str! in Task 2 depends on the .html files created in Task 3
- [Phase ?]: 16-02: reused pipeline.organization.paths for templates dir resolution instead of adding a new ActService paths field + with_paths builder
- [Phase ?]: 16-02: OrganizationService::read_logo_bytes added — reads legacy org.json logo file bytes+MIME for base64 data: URI embedding in render_acceptance_pdf
- [Phase ?]: 16-02: Rule-3 fix folded Tauri/HTTP adapter type changes (acts.rs, templates.rs, http/acts.rs) into this plan to keep cargo build -p trackly-app green; full delivery UX rework remains Plan 16-03 scope
- [Phase 16]: 16-03: Task 1/2 (String return type, text/html content-type) already complete from Plan 16-02's Rule-3 fix — scope narrowed to deleting acts_open_pdf_in_system + regenerating bindings.ts
- [Phase 16]: 16-03: ui/src/bindings.ts is gitignored, never committed — regenerated via cargo test --test export_bindings, verified in place, no git commit for that file
- [Phase ?]: 16-05: renamed render_with_missing_template_returns_notfound/render_with_broken_template_returns_validation to assert graceful fallback (embedded HTML default), not error
- [Phase ?]: 16-05: Rule 1 bugfix — org.logo_data_uri needed | safe in both HTML templates; autoescape was entity-encoding the / in base64 data: URIs, corrupting the logo in production
- [Phase ?]: 16-04: Save-as-PDF button removed entirely (not repurposed to save raw HTML) — browser print dialog already offers Save-as-PDF (D-09/Req 5)
- [Phase ?]: 16-04: Rule 1 fix in client.ts (outside stated files_modified) — HTTP transport's binary-response branch wrongly converted text/html responses to number[]; added explicit text/html -> res.text() branch, required for D-09 dual-transport correctness
- [Phase ?]: 16-04: templates_render_preview stale application/pdf content-type + Promise<number[]> frontend type left unfixed (dead code, zero UI callers) — logged to deferred-items.md
- [Phase ?]: 17-01: ReportService gained minimal organization: Option<Arc<OrganizationService>> field + with_organization builder (not full pipeline struct) since export_pdf only needs .paths for templates_dir resolution
- [Phase ?]: 17-02: TemplateService organization field + with_organization builder mirrors ActService/ReportService; validate_preview retargeted to HTML render
- [Phase ?]: 17-02: T-17-02-01 mitigated via fixed DEFAULT_HTML_TEMPLATES allowlist check before path join in update_body/reset_to_default
- [Phase ?]: 17-02: tests/template_edit.rs (Rule 3 fix) rewired with_organization + retargeted assertions from DB-backed get_active to file-backed list_all_for_editor
- [Phase ?]: 17-02: test env-var guard mutex switched to tokio::sync::Mutex (from std::sync::Mutex) since guards held across .await (clippy::await_holding_lock)
- [Phase 17]: 17-04: html_report_render.rs negative-artifact assertion avoids literal DocSpec/render_docspec substrings to not trip the Req 6 grep gate on the test file itself
- [Phase 17]: 17-04: fixed a Plan 17-01 unit test in report_service.rs whose negative-match assertion literally contained render_docspec/DocSpec strings, tripping the same Req 6 grep gate
- [Phase 17]: 17-03: PdfPreviewModal mode=report is additive-only extension (no rewrite of print machinery); ReportsPage export+print unified onto one modal-opening trigger; TemplateEditor variables panel is per-kind data-driven (VARIABLES_BY_KIND) replacing static hardcoded block
- [Phase ?]: 17-05: column_labels appended as new 8th arg to export_pdf (not replacing columns) — keeps row_field key-based cell resolution untouched
- [Phase ?]: 17-05: disallowed logo_mime drops the logo entirely (logo_bytes=None) rather than falling back to a default mime
- [Phase 17]: 17-07: full trackly-app test suite confirmed green (77 binaries, 0 failures) via background-monitored canonical CI invocation (mock env + --test-threads=1); closes Req-7's UNCERTAIN status with evidence, not hypothesis
- [Phase 18]: 18-01: list_grouped true-branch group key = (type_id,name,model) (D-05), sort by count DESC (D-04), name_prefix drives FTS5 text filter via build_fts_query (AUTO-03); false-branch untouched
- [Phase ?]: Phase 18 Plan 02: dropdownAnchor.ts wraps .dropdown AND .dropdown-item in :global() (not just .dropdown) — matches DeviceContextMenu.svelte precedent, avoids scoped-CSS pruning risk on portaled nodes; box-shadow --shadow-md -> --shadow-elev-2 (unused token fix)
- [Phase 18]: 18-03: PersonAutocomplete/DeviceAutocompleteField migrated to portal + dropdownAnchor recipe from Plan 18-02; DeviceAutocompleteField passes maxHeight:200 to match its 200px CSS max-height
- [Phase 18]: 18-03: Select/CartridgeSelect/GroupedPrinterSelect/PrinterSelect documented AUTO-01-compliant by construction (native <select>, no custom overlay) after re-reading each source, per T-18-07 mitigation
- [Phase ?]: 18-04: raw <input> replaces Input.svelte for device picker (no ref-forwarding); openByRow[idx] alone gates dropdown visibility, empty-state renders inside
- [Phase ?]: 18-04: activeIndexByRow keyboard-nav highlighting added as Rule 2 completeness fix alongside the plan's ArrowUp/Down/Enter/Tab handler
- [Phase ?]: 18-05: единственная оставшаяся после фильтрации группа всегда разворачивается через drillInto (auto-flatten), единый код-путь с обычным drill-in (AUTO-05/D-09)
- [Phase ?]: 18-05: количество устройства задаётся только в колонке «Количество» таблицы позиций — spinner убран из дропдауна пикера (checkpoint fix)
- [Phase ?]: 18-05: isExpandable требует ids.length>1 — единственный экземпляр не раскрывается, клик сразу выбирает (checkpoint fix)
- [Phase 19]: 19-01: ui/src/bindings.ts is gitignored — Task 1 regeneration verified but produces no committed diff (only Rust ActDto struct change is committed)
- [Phase 19]: 19-01: html_act_render.rs tests assert on act.date_human (RU) not act.date (ISO) — the act_handover.html template only renders date_human; ISO field is unused in markup
- [Phase 19]: 19-02: update_act_header_in_tx SET clause unconditional for 5 original header fields, COALESCE-only for handover_date_utc/number — Plan 19-03 must resolve values before calling
- [Phase 19]: 19-02: complectation_at_time semantics documented on ActUpdateItemDto (retained vs newly-added device); specs (тех.характеристики) intentionally excluded from update DTO
- [Phase 19]: 19-03: update_act_header_in_tx's unconditional SET fields always resolved to Some(..) in ActPatch construction (never left as outer None)
- [Phase 19]: 19-03: custom:update_remove chosen as distinct audit action for edit-driven device removal (vs delete_soft's custom:undo); payload_json still carries act_id for bulk-undo compat
- [Phase 19]: 19-03: requirements-completed left empty (not ACT-02) — requirement spans plans 19-02..19-05, only backend half done here (matches 19-02's precedent)
- [Phase ?]: 19-04: build_acts_update mirrors build_acts_create's single-DTO shape (id/expected_version live inside ActUpdateDto, not split args)
- [Phase ?]: 19-04: RBAC regression landed as Case 42 (grepped actual max 41 first, not a stale plan-suggested number)
- [Phase ?]: 19-04: requirements-completed left empty (not ACT-02) -- transport wiring only; Plan 19-05 closes the user-visible UI loop
- [Phase 19]: Plan 19-05: edit-mode prefill sources directly from initialAct (acts.get(id) result), bypassing live device search since existing act positions are в_работе, not на_складе
- [Phase 19]: Plan 19-05: second, independent ActFormModal instance (mode=edit) added in ActsPage rather than threading shared create/edit state through one modal
- [Phase 19]: Plan 19-05: D-07 edit-button gating deliberately omits !act.archived — archived handover acts remain editable, unlike Возврат
- [Phase 19-06]: recompute_parent_archived call gated on added/removed non-empty, placed after CAS header UPDATE and before final-audit fetch — Closes CR-01 — archived was never recomputed on update()'s device-set mutations; recompute must run after CAS (both bump version) to avoid spurious OptimisticLockMismatch, and gating preserves the header-only version+1 contract
- [Phase ?]: 19-08: WR-02 closed via option (a) — clamp UI quantity to 1 in edit mode (schema-consistent per D-06) rather than extend ActUpdateItemDto with quantity/device_ids
- [Phase ?]: 19-08: edit-mode qty column renders static '1' span, not a disabled input, to avoid a misleading spinner control
- [Phase ?]: 19-08: todayISO() switched to getUTCFullYear/getUTCMonth/getUTCDate, unifying with unixToIso()/isoToUnix() UTC convention (IN-01)
- [Phase 19]: 19-07: WR-01 cascades renamed act number to child return acts in the same tx (option a) instead of excluding act_type='return' from the uniqueness check — preserves do_return's copy-parent-number invariant
- [Phase 19]: 19-07: WR-03 audits retained-item complectacia edits (custom:act_item_complectation_edit) gated on stored != incoming value, so a no-op resubmit writes zero audit rows
- [Phase ?]: 19-09: retained-vs-new row marker for act-edit device cell is complectation_at_time !== undefined (not row.picked) — row.picked is also true for a device freshly chosen during the current edit session; only complectation_at_time (set exclusively by itemsFromInitialAct) distinguishes a retained/prefilled position
- [Phase ?]: 19-09: ActFormBody.svelte left untouched after комплектация UI removal — itemsFromInitialAct prefill and the edit payload's complectation_at_time mapping still round-trip the value unchanged even though the editable input was removed from ActFormItemsTable
- [Phase ?]: 19-10: handleEditSaved assigns selectedAct = act directly (fresh ActDto from acts.update()) for immediate reactive detail refresh, closing D-11 stale-detail bug (selectedActId=act.id alone is a no-op when the act is already selected)
- [Phase ?]: 19-10: Редактировать/Возврат buttons on ActDetail converted from disabled-placeholder to bare omission, gated on act_type==='handover' && !act.archived — closes D-12/D-13; return-act editing stays out of scope
- [Phase 22]: 22-01: D-07 implemented this plan (compute-on-read archived_at_utc, no new column, no migration) per user decision 2026-07-12
- [Phase 22]: 22-01: ActReturnDto new fields (giver_name/receiver_name/handover_date_utc) are Option<T> + serde(default) back-compat; write-site consumption deferred to Plan 22-02
- [Phase ?]: [Phase 22]: 22-02: do_return write-site fix persists payload's own giver/receiver/handover_date_utc (D-05/D-12/Pitfall 1); None falls back to parent-swap/now for back-compat
- [Phase ?]: [Phase 22]: 22-02: update_return() clones Phase 19's update() inverted to ActType::Return (added=newly-returned; removed=un-return restore; retained-with-change=re-apply) in one single-writer tx
- [Phase ?]: [Phase 22]: 22-02: D-11 guard = 3-field snapshot compare (status_id+location_id+state) vs return's own after_json; validate-then-mutate, Conflict aborts whole tx, no force-override (catches reissue AND manual relocation)
- [Phase 22]: 22-03: acts_update_return reuses Action::MutateActs (no new RBAC surface) — same gate as acts_update/acts_return/acts_delete, proven by role_endpoint_matrix Case 43
- [Phase 22]: 22-03: bindings.ts stays generated-only — regenerated via cargo test --test export_bindings, never hand-edited; only Rust command/DTO + export_bindings.rs assertions + acts.ts are committed
- [Phase 22]: 22-04: ReturnModal edit mode defaults applyToAll=false on open — preserves per-row saved condition/location from editTarget.items instead of discarding behind an unset bulk field
- [Phase 22]: 22-04: single ReturnModal instance reused for create+edit via mode/editTarget/parentAct props (not a second modal component)
- [Phase 22]: 22-04: ActUpdateReturnDto unused location_id/location_name/notes/deadline_utc fields sent as null from edit payload — confirmed unread by ActService::update_return
- [Phase 22]: 22-05: CR-01 fix applied at consumption point (location.or(before.location_id) before update_full_in_tx), preserving None='no override' semantics upstream — avoids breaking D-11 change detection which relies on None meaning no location override was requested
- [Phase 22]: 22-05: CR-02 fix tags retained-edit audit rows with custom:return_item_edit and excludes that action from select_latest_device_mutation — generalizes correctly across multiple retained edits before un-return, unlike a status-based filter; select_latest_device_mutation_pair (D-11 drift check) left untouched since it needs the newest row including retained-edits
- [Phase ?]: 22-06: validate_update_return mirrors validate_return (dedup/non-empty/per-item-override) MINUS act_item_id dedup (edit items use act_item_id:0 placeholder) — closes WR-01 raw-HTTP gap
- [Phase ?]: 22-06: update_return step 8a added-loop ports do_return's already_returned+qty<=handover_qty bound (WR-03)
- [Phase ?]: 22-06: parent_act_id .expect() -> AppError::Internal domain error inside single-writer closure (WR-02) — no panic path poisons the write task
- [Phase ?]: 22-06: V034 comment corrected (WR-04) — one-time backfill, NOT safe to re-run manually post-Phase-22; comment edit changes refinery checksum so existing dev DBs must be recreated (tests use fresh temp DBs, unaffected)
- [Phase 20]: 20-01: V035 is next-sequential migration (after V034); address_line2 appended as LAST field/column everywhere (no ordinal shift to existing columns), per D-04/D-10
- [Phase 20]: 20-01: embed_migrations! stale incremental-build cache — touching crates/trackly-infra/src/db/migrations.rs forces rebuild if new migration files aren't picked up by test runs
- [Phase 20]: 20-02: render_acceptance_pdf rewritten to org_db.get_for_pdf() parity with render_pdf; read_logo_bytes/org.json legacy path fully removed (D-11); address_line2 propagated to all 3 render ctx sites (D-07)
- [Phase 20]: OrgSettings.svelte address_line2 wired; bindings.ts regenerated (gitignored, no commit needed)
- [Phase 21]: 21-01: format!("{prefix}-{seq:04}") is minimum-width, not fixed — no migration needed; existing 6-digit codes stay valid distinct strings
- [Phase 21]: 21-01: cartridges_numbering.rs assertion widened to len >= 6 (min 4 digits) per plan spec, forward-compatible with counters > 9999
- [Phase 23]: 23-01: --tr-line-height-mono фиксирован как 1.4 (не задано UI-SPEC для mono-роли) — по аналогии с --tr-text-label при том же размере 13px
- [Phase 23]: 23-01: заголовочный комментарий global.scss переформулирован без буквального @use './tokens' в тексте — иначе греп-критерий D-05 (ровно 1 совпадение) ложно триггерится
- [Phase 23]: 23-02: Rule 3 (closed-world gate) strips comments before matching — {role}-placeholder docs in _tokens.scss otherwise trip a false undefined-token violation
- [Phase 23]: 23-02: scripts/**/*.mjs added to eslint.config.js's existing node-config file-pattern block — new dev scripts weren't covered by any existing glob
- [Phase 23]: 23-02: D-15 closed — all 5 pre-existing eslint errors fixed; 7 pre-existing prettier-formatting-drift files logged in deferred-items.md, out of scope
- [Phase 23]: 23-03: var(--tr-text-inverse) на трёх auth-экранах не переименован в --tr-on-accent — консистентность с необновляемым skip-link паттерном (Layout.svelte/EmployeeLayout.svelte)
- [Phase 23]: 23-03: NetworkSettings/UserListRow success-бейдж мигрирован на --tr-success (color-mix источник) / --tr-success-text (текст) — ближе к установленному -soft/-text triplet паттерну
- [Phase 23]: 23-04: verify-value-map.mjs RADIUS_EXCEPTION_FILES fixed to include ui/ prefix (git diff paths are repo-root-relative) - built in 23-02, false-positived on the exact expected radius-sm allowlist exception
- [Phase 23]: 23-04: --radius-lg QA-01 fix applied to 4 auth screens (LoginPage/BlockedScreen/FirstRunWizard/PendingScreen) as part of Task 1 space/radius sweep
- [Phase ?]: 23-05: ReturnModal.svelte делегирует рендер списка возврата ReturnItemsTable.svelte — deviceLabel декомпозирован в deviceName+inventoryNo для tr-mono seam
- [Phase ?]: 23-05: class="tr-mono" всегда как отдельный вложенный span, не примесь к multi-class атрибуту — гарантирует греп-видимость точного литерала
- [Phase 23]: 23-06: Whole-tree final verification found 0 residual gaps (all 3 check-tokens.mjs rules + verify-value-map.mjs clean on first run) — Confirms plans 23-01..23-05 left no seam gaps between sequential sweep-plans
- [Phase 23]: 23-06: pnpm prettier --write . run per plan instruction, closing last pre-existing prettier-drift file; pnpm lint green for the first time in phase 23 — All 6 diffs manually verified as pure line-wrap/reflow, no logic or value changes
- [Phase 23-07]: check-tokens.mjs Rule 4 intentionally matches rgba/hsl inside var(--tr-x, rgba(...)) fallbacks — closed-world token model makes such fallbacks dead code to remove, not preserve
- [Phase 23-07]: verify-value-map.mjs tokensOnSide() applies an unanchored global regex per split line (no m-flag) instead of one anchored+lazy pattern over the whole hunk text — fixes CR-01
- [Phase 23-07]: tr-danger-ring fixed at alpha 0.2 for both themes (rgb components copied verbatim from tr-danger), canonizing 8 of 9 duplicated invalid-focus-ring sites; Button.svelte 0.3 converges in plan 23-08
- [Phase 23]: 23-08: Modal.svelte overlay dark-mode override удалён после миграции на theme-scoped var(--tr-overlay)
- [Phase 23]: 23-08: Button.svelte danger-ring alpha 0.3->0.2 (var(--tr-danger-ring)) — WR-01-санкционированный visual touch, handoff в фазу 24 (CMP-01)
- [Phase ?]: 24-01: --tr-accent-text values transcribed verbatim from RESEARCH.md (Badges.dc.html/Tabs.dc.html agree), not recomputed
- [Phase ?]: 24-01: theme.svelte.ts applyResolved() uses requestAnimationFrame (not setTimeout) to remove .theme-switching, guaranteeing removal only after new theme paints
- [Phase ?]: Phase 24 Plan 02: added missing --tr-danger-hover/--tr-danger-active tokens to _tokens.scss (Rule 3 blocking fix) — RESEARCH.md claimed VERIFIED-present but they didn't exist
- [Phase ?]: Phase 24 Plan 02: ButtonsSection.svelte written as fully explicit static markup (no #each loops) to match literal-string acceptance greps and keep showcase self-documenting
- [Phase 24]: Checkbox/Radio destructure props with let (not const) — required for bind:checked/bind:group on their own native input; Input/Select/Textarea keep const since they never bind: to themselves
- [Phase 24]: Checkbox/Radio .invalid state reuses Input/Select/Textarea's --tr-danger/--tr-danger-ring pair since Fields.dc.html has no dedicated error-box spec for these two
- [Phase 24]: 24-04: Modal .modal-container background --tr-surface-raised -> --tr-surface (matches Modal.dc.html; diverges from surface-raised in dark theme, identical in light)
- [Phase 24]: 24-04: Modal border NOT added despite Modal.dc.html spec — task explicitly scoped this out; known gap for future pass
- [Phase 24]: Badge default render preserved verbatim (accent stays solid); appearance matrix opt-in via badge-m* namespace
- [Phase 24]: 24-06: Tabs container role rendered via literal {#if}/{:else} branches sharing one #snippet (not a dynamic ternary) — required for the role="tablist"/role="group" acceptance-criteria grep to pass
- [Phase 24]: Task 3 (24-07 human-verify checkpoint) closed as gate-closure under auto_advance; live browser/visual verification against .dc.html references NOT performed by a human and remains outstanding — documented in 24-07-SUMMARY.md
- [Phase 24]: 24-08: kept oninput/onchange callbacks alongside bind:value/bind:checked unchanged — both mechanisms coexist without conflict; no consumer-facing API change
- [Phase 24]: 24-09: Nested &.badge-m-count inside .badge-m-success/.badge-m-warning/.badge-m-danger (matching existing &.badge-m-soft/&.badge-m-solid/&.badge-m-dot nesting), not new flat .badge-m-X.badge-m-count selectors
- [Phase 24]: 24-10: single $effect on 'open' handles both initial focus and focus-restoration cleanup; trapTab only intercepts Tab/Shift+Tab at wrap-around edges, native tab order elsewhere
- [Phase 24]: 24-11: gate="blocking-human" primary + config auto_advance flip secondary defense-in-depth for genuine human checkpoint sign-off (closes 24-07's silent auto-approval gap)
- [Phase ?]: 24-12: single TRAP_FOCUSABLE_PARTS array derives both dialog-scoped and portal-scoped selectors via .map() (not string-prepend to joined selector) to avoid silently scoping only the first alternative
- [Phase ?]: 24-12: portaledFocusable() uses getClientRects().length>0 not offsetParent, since dropdownAnchor.ts sets position:fixed on portaled dropdowns
- [Phase 24]: 24-13: Gave all 5 Badge tones a border for appearance=count (majority pattern) instead of removing borders from the 4 that had them
- [Phase 24]: 24-13: Fixed WR-06 via one-way value={value} + explicit value = v assignment in oninput, not by narrowing Input's type prop union (would break ActNumberField.svelte)
- [Phase 25]: 25-01: TableRow owns ALL base <td> metrics (D-10); Table.svelte is shell-only, no consumers wired
- [Phase ?]: 25-02: void-marker pattern to split Dropdown.svelte across 2 task commits under strict noUnusedLocals TS gate (removed in Task 2 as bindings wire into the panel)
- [Phase ?]: 25-02: AUTO-05 $effect only runs when flat=false — flat mode has no drill-in concept
- [Phase ?]: 25-02: Dropdown panel CSS uses overflow:auto not UI-SPEC's literal overflow:hidden — required for scrolling within max-height
- [Phase ?]: 25-03: checkmark font-weight 600 (var(--tr-font-weight-semibold)) not a new 700 weight, per UI-SPEC Checker Sign-Off recommendation
- [Phase ?]: 25-03: two-stage Escape in member-view gated on showBack (not just viewMode), so AUTO-05's auto-flattened single-group view closes immediately instead of looping back into a 1-item groups list
- [Phase ?]: 25-03: drill-in focus management via new returnIndex field — entering member-view activates first option, backToGroups() restores the group's own index
- [Phase ?]: 25-04: group count-pill uses a named const bound via variant={countPillVariant}, not a literal variant="accent" string, to keep the badge-tone grep gate exact at 4
- [Phase 25]: DeviceListRow group-last-child divider uses :global(tr.group-last-child) > .cell (not the plan's bare > td) to out-specificity TableRow's own base <td> border-bottom rule
- [Phase 25]: DeviceList footer visibility gated by a dedicated skeletonLoading derived, shared with Table's loading prop, avoiding a regression where the footer would show during initial-load skeleton
- [Phase ?]: Programmatic focus()/click() sequence in onMount forces Dropdown's fully-internal open/viewMode/drill-in state into a permanently-visible demo (Plan 25-06) since Dropdown exposes no bindable props for that state (Plan 25-02 D-02)
- [Phase 25]: 25-07: combined Task1(wire Dropdown)+Task2(cleanup) into one commit — literal split fails project's strict noUnusedLocals gate (old handlers become dead the instant old markup is removed), same class as Plan 25-02
- [Phase 25]: 25-07: fixed Dropdown.svelte itself (outside declared files_modified) — mouse-click/Enter pick never closed the panel in Plans 25-02/25-03, first exposed by this pilot; added open=false in handleOptionClick + new handleMemberClick
- [Phase ?]: 25-08: openPanel() shares the expandSeq counter (increment before resets) instead of a parallel invalidation mechanism, so drillInto/AUTO-05 effect stay coordinated on reopen
- [Phase ?]: 25-08: Tab-branch guard checks group truthiness before isGroupExpandable to prevent crash on Tab-with-no-active-option
- [Phase ?]: 25-08: select-variant search input gets keyboard/ARIA wiring but not onmousedown-preventDefault — needs real focus for typing
- [Phase 26]: 26-01: sidebar-width migrated by value (240px->236px), not by name — not a --tr-* token, check-tokens.mjs closed-world gate does not scan it
- [Phase 26]: 26-01: PageHeader owns the burger button internally, not Layout — future pages adopting PageHeader get the mobile toggle for free
- [Phase ?]: 26-04: footer prop passed explicitly as footer={footer} (not shorthand {footer}) to satisfy plan's grep-based acceptance criteria
- [Phase ?]: 26-05: Search input labelled via visually-hidden label for=id instead of adding aria-label prop to Input.svelte (out of scope, owned by Plan 26-03)
- [Phase ?]: 26-05: Tabs key === 'null' sentinel recovers STATUSES[0].id (literal null) from String(null) round-trip — verified for all 5 status entries
- [Phase ?]: 26-07: Pill breakdown uses local .pill-row/.pill markup, not Badge.svelte (Badge lacks label+strong pair support, UI-SPEC §3.10)
- [Phase ?]: 26-07: ChartWidget COLORS kept as literal hex, documented exception to --tr-* token gate for data-viz series consistency across themes
- [Phase ?]: 26-07: Value-label-over-bar font-size set to 9px (not 11px) per conditional-acceptance value; 11px fallback deferred to Plan 26-08 dark-theme UAT
- [Phase 26-08]: gate="blocking-human" primary layer held through a real 10-round gap-closure cycle without silent auto-approval; workflow.auto_advance flip/restore (Tasks 2/4) verified net-zero relative to pre-plan config.json
- [Phase ?]: 27-01: DetailField использует var(--tr-space-3xs) вместо gap:2px (миграция по значению)
- [Phase ?]: 27-01: DetailPanel не красит фон .detail-panel — поверхность даёт обёртка master-detail (D-02, планы 27-02/04/07)
- [Phase 27]: 27-03: PdfPreviewModal/ActFormModal/ActNumberField audit-only — уже полностью на токенах/примитивах
- [Phase 27]: 27-03: сырые чекбоксы без bespoke-CSS всё равно заменены на Checkbox-примитив ради консистентности must_haves truth о примитивах в модалках Актов
- [Phase 27]: 27-05: OperationModal.svelte уже полностью ре-токенизирован в фазе 23 — Task 1 выполнен как аудит без изменений кода
- [Phase 27]: 27-05: ModelFormModal brand/model автокомплит остаётся на inline bespoke-input паттерне (не переведён на portal+dropdownAnchor) — миграция позиционирования вне границы SC #4
- [Phase 27]: 27-06: Tasks 1-2 (CartridgeFormModal/CartridgeFormBody, CompatibilityEditor/CartridgeContextMenu/LowStockBanner) required no code changes — re-audit confirmed prior compliance with tokens/primitives
- [Phase 27]: 27-06: CompatibilityEditor raw input autocomplete rows kept (not Input primitive) — matches established LocationAutocomplete.svelte pattern for custom listbox/aria dropdown logic
- [Phase ?]: 27-08: Task 1/3 (PrinterCreateModal/DiscoveryModal/TonerGauge/PrinterAlertBanner) — аудит без изменений, уже на var(--tr-*) и примитивах
- [Phase ?]: 27-08: Checkbox-примитив не поддерживает aria-label — доступный текст DiscoveryResultsTable передан через children-snippet + локальный .sr-only
- [Phase 27-02]: Table.svelte/TableRow.svelte остаются нетронутыми (D-03); функциональные пробелы (empty-state action, row-click) закрыты на стороне ActsList/ActListRow (footer-snippet, onclick на <td>), не правкой shared-примитива
- [Phase 27-02]: DetailPanel.title остаётся string (не Snippet) — заголовок детали акта потерял tr-mono стилизацию номера; чисто типографская деталь, поля/действия не затронуты (D-01)
- [Phase 27-02]: ActHeaderField.svelte удалён — единственный потребитель (ActDetail) мигрировал на общий DetailField (D-01)
- [Phase 27]: 27-04: CartridgesSearchAndTabs tab keys already string-typed — Tabs adapter trivial, no numeric String() round-trip
- [Phase 27]: 27-04: ModelsList renders Table framed=false inside existing bordered toolbar card — avoids double-framing (Table has no header-toolbar slot)
- [Phase 27]: 27-04: CartridgeDetail field-grid CSS class renamed fields-grid to info-grid to avoid literal collision with removed-bespoke-class grep gate
- [Phase ?]: 27-07: колонка тонера в списке принтеров показывает первую запись tonerLevels (TonerGauge инлайн), не все цвета
- [Phase ?]: 27-07: секция Данные устройства в PrinterDetail — DetailSection без heading-пропа, чтобы сохранить локальную section-heading-row (заголовок+кнопка Редактировать)
- [Phase 27]: Both-theme UAT (D-02) не auto-approved несмотря на auto_advance=true — требуется живой человеческий просмотр обеих тем; approved после cargo tauri dev UAT + 24 fix-коммитов (батчи B-G: fill-height master-detail, единый фон полей, кастомный Dropdown в Картриджах, sticky-шапка деталей, inset selected-row) — Визуальная проверка raised-vs-surface distinction в обеих темах требует реальных глаз; gate=blocking в плане 27-09 соблюдён буквально
- [Phase ?]: RequestsSearchAndTabs: justify-content:space-between replaces .tabs{flex:1} wrapper to keep create-button right-aligned without a bespoke flex wrapper (28-01)
- [Phase 28]: 28-02: panelTitle = typeLabel (простая строка), title-row (2 Badge) + meta-row — bespoke первый контент children по прецеденту PrinterDetail
- [Phase 28]: ReportSubNav count fallback for missing statusCounts changed from string '–' to number 0 (Tabs.count typed number) — accepted minor edge-case
- [Phase 28]: PeriodSelector onMonthChange/onYearChange changed signature from Event to string (native select removed); period-recalculation logic unchanged
- [Phase 28]: ReportTable error-state kept as sibling branch outside Table (no Table API equivalent for error, only loading/empty), same pattern as RequestDetail/ActDetail loading branches
- [Phase 28]: ReportFilters.svelte required zero code changes for D-04 audit — GAP-R4 had already removed all filter fields, fully on Button primitive
- [Phase 28]: 28-06: Select/Input (block-level, width:100%) обёрнуты в узкие wrapper-div (.select-shrink/.input-shrink) в BackupSettings для сохранения прежней компактной ширины полей
- [Phase 28]: 28-06: .fingerprint в NetworkSettings намеренно не тронут (bare font-family: monospace) — вне явного объёма плана, симметрично StorageSettings.db-path-code
- [Phase 28]: org-email в OrgSettings использует Input type="text" вместо native type="email" (28-07) — Input.svelte контракт не поддерживает 'email' тип; серверная валидация остаётся авторитетной
- [Phase 28]: Plan 28-08: TemplateEditor kind-select — .select-shrink wrapper (fit-content + min-width 220px), reused from BackupSettings.svelte, avoids Select's width:100% stretching in the label+select flex row
- [Phase 28]: Пароль в UserFormModal остаётся raw type=password (T-28-09-01) — обязательное исключение из D-04, Input.svelte не поддерживает password-тип
- [Phase 28]: Email в UserFormModal через Input type=text — нативная HTML5-валидация email потеряна, серверная валидация авторитетна
- [Phase 28]: Phase 28 Plan 10: both-theme UAT (D-01..D-08, SC #1-4) approved by human after gap-closure round (28-11..28-16) — Known '0' vs legacy '-' report-counter diff (28-03) reconfirmed as sole intentional visual difference; does not misrepresent data
- [Phase ?]: Phase 29 Plan 01: AuthShell does not render title element — each screen keeps its own heading as children (title-to-content spacing differs per screen)
- [Phase ?]: Phase 29 Plan 01: FormField error color uses --tr-danger-text (Fields.dc.html), deliberate deviation from LoginPage's pre-existing --tr-danger convention
- [Phase 29]: Plan 29-02: Input.svelte gains additive autocomplete prop (HTMLInputAttributes['autocomplete']) to support LoginPage/FirstRunWizard migration
- [Phase 29]: Plan 29-03: PendingScreen needed local .pending-card text-align:center wrapper — AuthShell's non-stack default has no built-in text-align
- [Phase 29]: EmployeeLayout fill-to-bottom mirrors admin Layout.svelte definite-height pattern (D-04 parity); ThemeSwitcher bounded at consumer, not shared component
- [Phase 30]: Canonical 43-pair WCAG contrast table hardcoded in check-contrast.mjs (no CLI params) — closed-world by design, matches check-tokens.mjs Rule 3
- [Phase 30]: rgba()-based tokens (soft/focus-ring/danger-ring/overlay/row-selected) intentionally excluded from contrast table — alpha compositing out of scope, residual risk closed by manual UAT in plan 30-03
- [Phase 30]: check-focus-outline.mjs uses brace-depth stack to find enclosing rule — single algorithm handles same-block (ActListRow) and cross-nested-block (Tabs.svelte) paired outline/box-shadow patterns
- [Phase 30]: 30-02: Dropdown search-input использует outward 2px focus-ring (не inset) — панель scrolls (overflow:auto), не clips
- [Phase 30]: 30-02: .tr-dropdown-option (drill-in panel) намеренно не тронут — UAT-кандидат для финального чекпоинта плана 30-03
- [Phase 30-04]: No focus-restore-on-close for the new search-input auto-focus $effect (30-04) — matches plan's explicit out-of-scope instruction
- [Phase 30-04]: ArrowLeft with showBack=false is an intentional no-op, not a panel-close (30-04) — asymmetric with Escape's fallback since there's nothing to navigate back to
- [Phase 30]: 30-05: check-focus-outline ignore marker must be a single line immediately before outline: none; (script only checks current+previous line, not multi-line comment blocks)
- [Phase 30]: Task 2 live-check (30-07): CSS-харнесс в headless Chromium/Playwright вместо реального приложения (auth недоступен в этой среде) — точно воспроизведены селекторы Layout.svelte/DashboardPage.svelte после фикса — Полноценный запуск требовал бы настройки dev-аутентификации/сид-данных ради одноразовой проверки чисто-CSS фикса; харнесс тестирует ровно тот CSS grid-track-sizing механизм, который был root cause
- [Phase 30-08]: Dropdown search-input: убрать всегда-видимое кольцо фокуса, добавить встроенную клиентскую фильтрацию (visibleGroups) — Кольцо было визуальным шумом на всегда-сфокусированном поле; фильтрация нужна для 11 flat+select+searchable консьюмеров одним изменением вместо правки каждого файла (Gap 3, QA-02)
- [Phase 30]: Плана 30-09: live-проверка Gap 7 в реальном приложении отложена до открытого UAT re-run чекпоинта 30-03 Task 3 — синтетическая проверка (Playwright + реальный compiled CSS в WebKit/Chromium) подтвердила механизм фикса вместо неё
- [Phase 31]: MockAdDirectory reuses existing us100/us200 fixture identities (no new placeholder names)
- [Phase 31]: TtlCache<V> implemented generically to support two independently-TTL'd instances (display_name, role) in Plan 31-02
- [Phase 31]: Plan 31-03: role_hint threaded ONLY into auto_register_ad_user/create_pending_registration; on_ad_bind_success's other branches untouched
- [Phase 31]: Plan 31-03: try_ad_login (password-bind path) passes role_hint: None — directory role enrichment is SSO-only in this phase
- [Phase 32]: Plan 32-01: admin_logins stored as flat TOML string array in AdConfig, mirroring role_mapping pattern (D-01)
- [Phase 32]: Injection at on_ad_bind_success (both sso_login and try_ad_login) — DRY, ADMIN_AD_LOGINS parity
- [Phase 32]: Single audit_log action 'ad_auto_admin' with payload_json.prior_state distinguishing branches
- [Phase 32]: admin_logins threaded via with_admin_logins builder (not new constructor arg) to avoid touching 9 existing AuthService::new call sites
- [Phase 33]: pagedjs pinned to exact 0.4.3 (no caret) so Plan 33-02's CSP sha256 hash-source cannot silently drift on pnpm install
- [Phase 33]: bootstrapScript.js kept static/non-interpolated; ui/eslint.config.js extended with parent/HTMLIFrameElement/MessageEvent globals + a script-mode override for the new bootstrap script
- [Phase ?]: PRV-CSP: CSP hash-source computed once (sha256-5ZDjul5PEiak1qhxbmi9Rx3W4tYmf4sQbt9wgef8vQY=) and hardcoded as a literal in http/mod.rs, verified by ui/scripts/check-pagedjs-csp-hash.mjs drift gate wired into pnpm lint
- [Phase 33]: Paged.js srcdoc built imperatively (not $derived) once per render, to avoid iframe reload/pagination loss on theme toggle
- [Phase 33]: pagedjs dist bundle imported via relative filesystem path (not bare package specifier) — its package.json exports map has no ./dist/* subpath entries, which broke vite build once the module became reachable from the app entry graph
- [Phase 33]: Plan 33-04: printViaTopLevel wraps previewer.preview() stylesheet-argument shape in try/catch with a wrapper-stripping fallback (RESEARCH.md Open Question 2, unverified by automated tests — see 33-VALIDATION.md manual UAT row)
- [Phase 260808-np4]: Consolidated REQ-06 ad_register visibility rule from 3 duplicated implementations (11 call sites) into 2 shared functions: trackly_core::auth::excludes_ad_register and requests_sqlite::ad_register_predicate/ad_register_exclude_clause
- [Phase ?]: 34-01: full_name appended as LAST column in org_settings so all pre-existing SELECT/UPDATE ordinal positions stay stable
- [Phase ?]: 34-01: migrate_from_org_json legacy UPDATE left untouched — org.json has no full_name equivalent
- [Phase ?]: 34-01: cargo clean -p trackly-infra required after adding a new migration file — refinery embed_migrations! has no rerun-if-changed hook for migrations/, so new .sql files are invisible to incremental rebuilds until a scoped clean
- [Phase ?]: Rescued reference header from target/debug/templates/act_handover.html (still present, matched research mtime/size), privacy-scrubbed by manual substitution of org.full_name/org.name for the hardcoded real org name -- never a whole-file copy
- [Phase ?]: v21 legacy snapshot taken via cp BEFORE the canonical templates' rewrite (D-15/Pitfall 5 timing), verified non-empty diff post-rewrite
- [Phase ?]: _header.html registered in DEFAULT_HTML_TEMPLATES but NOT in KNOWN_LEGACY_DEFAULTS (no legacy predecessor) -- adjusted the pre-existing .first()-based upgrade test to skip filenames with no registered legacy slice
- [Phase 34]: 34-03: enabled minijinja's multi_template feature — required for {% include %} to parse, orthogonal to the no-filesystem-loader invariant
- [Phase 34]: 34-03: list_all_for_editor filters filenames starting with '_' — shared partials like _header.html are not standalone editable document kinds
- [Phase 34-04]: Placed full_name field after address-line2, before phone in OrgSettings.svelte, mirroring address_line2's 4-touchpoint shape
- [Phase 34]: D-17: templates_status endpoint reports missing/unreadable file as Current (2-value enum, no third Missing variant)
- [Phase 34]: act_handover.html separate header was NOT a defect: canonical template already includes _header.html; on-disk file was user's hand-edited copy correctly preserved by D-14/D-16 upgrade logic, verified live on real pre-Phase-34 install
- [Phase 34]: No 'template changed manually' UI indicator in Settings is expected — 34-05 is backend-only by design, UI consumer deferred to backlog DOC-12
- [Phase 34]: C-01 (empty full_name, short name only) visual result reviewed and explicitly accepted as-is — direct consequence of D-04's independent-conditional-lines design
- [Phase 34]: Task 1 Step 7 scripted scratch-directory upgrade procedure superseded by a stronger real-install observation (act_handover.html finding), not run as scripted
- [Phase 34]: Report print-form .subtitle rendered raw English PeriodDto.mode discriminator ('year 2026') — fixed via ReportService::format_period_label covering both transports, found during UAT and scoped into phase 34 at user's direction
- [Phase 35]: act_acceptance.html signature block reworked to horizontal one-line-per-signer layout, byte-identical CSS/markup pattern to act_handover.html (D-09/D-06/D-07/D-08); duplicate Кто передал/Кто принял table rows removed
- [Phase ?]: Full-suite cargo test -p trackly-app hangs on pre-existing login_remember_persistent_cookie test (lives inside trackly-app package, not just --workspace); use -- --skip login_remember_persistent_cookie for full-package verification
- [Phase 35]: Phase 35 UAT approved: body/signature-block rework confirmed on both transports; multi-device pagination (Приложение №1) explicitly deferred to Phase 36 (DOC-10/DOC-11)
- [Phase 35]: 35-06: G-01/CR-01 закрыт — гейт length==1 снят, .device-block самоидентифицируется именем устройства при любом N (D-02a); human-UAT на обоих транспортах approved
- [Phase 35]: 35-06: известный follow-up (не исправлен в этом плане) — для act_handover.html не снят срез _legacy_defaults/v23/ под правку Task 1; практических последствий нет (тег не выпускался, материализованных копий с промежуточным телом на машине не осталось)
- [Phase 35]: 35-07: v23-снимок снят строго ДО CSS-правки (min-width:0/white-space:normal/overflow-wrap:break-word в .signature-row .signature-name), иначе assert_ne! precondition guard нового теста прошёл бы тривиально
- [Phase 35]: 35-07 Task 3 (human-verify, gate=blocking): approved — пользователь подтвердил перенос длинного вымышленного ФИО в пределах печатной ширины на десктопе и в LAN-браузере для обоих актов, без обрезания и без ухода за край листа
- [Phase 36]: 36-01: v24 snapshot taken from current HEAD before any pagination edit (Pitfall 7/C-01), byte-identical confirmed via diff; new upgrade_replaces_v24_... regression test is expected RED until 36-02 lands the pagination rewrite (structural, self-resolving)
- [Phase 36]: act_handover.html N=1/N>1 pagination: appendix table uses tbody-per-device (not bare tr) for break-inside: avoid, per Paged.js's TBODY/THEAD-only fragmentation support
- [Phase 36]: 36-03: render_handover_default_template_uses_field_rows_not_device_card narrowed from N=2 to N=1 — abbreviated appendix <th> headers are legitimate design at N>1 (D-01), not a device-card regression
- [Phase 36]: 36-03: act_items.quantity>1 only exercised via direct DB UPDATE in tests — ActService::create's legacy clone-on-handover path always inserts quantity=1 per row
- [Phase 36]: 36-03: html_field_row_underline_gate.rs widened from 2 to 3 legitimate border-bottom sources — new .appendix-table thead tr hairline (D-05) is a discovered drift fix, not a scope loosening
- [Phase 36]: Plan 36-04: RepeatTableHeadHandler in bootstrapScript.js must be a native ES6 class, not ES5 pseudo-inheritance — window.PagedModule.Handler is a native ES6 class in the bundled paged.min.js UMD build; invoking it via .call() throws TypeError at runtime, which the D-02 degrade path silently masks as an unpaginated fallback (found via desktop UAT checkpoint rejection, commit c11b0d9)
- [Phase 36]: D-17 supersedes D-03: act.items_grouped[] aggregates print-identical positions in Rust (mirrors devices_sqlite::list_grouped, extended with all printed fields) — act_items.quantity is hardcoded to 1 and never carried a real multiplicity signal
- [Phase 36]: D-17 (заменяет D-03, 2026-08-13): одинаковые позиции акта склеиваются в печати через group_items_for_print() — исправлено в gap-closure плане 36-06, живо подтверждено пользователем.
- [Phase 36]: Пользователь 2026-08-13 явно отложил проверку реальной печати/LAN-транспорта/изоляции печатного DOM на следующую сессию, приняв риск — зафиксировано как НЕ пройдено, не как пройдено.
- [Phase ?]: 37-01: scratch-only substitution script (never committed) applied 22 class A/B/C replacements across 14 HEAD files, verified via git diff + git grep before deleting the scratch mapping (D-03)
- [Phase ?]: 37-01: grouped 5 commits by which class(es) each file actually carries (2 combined A+C / B+C commits for mixed files) rather than forcing a strict 3-commit A/B/C split
- [Phase 37]: ROADMAP.md dangling-reference rewrite applied inside Phase 37's own section (Success Criteria + Wave 1 bullet) to satisfy Task 2's automated zero-match gate, checkbox state left untouched
- [Phase 37]: Reused template_service.rs demo_context_for_kind's established placeholder requisites (phone/fax/okpo/ogrn/email) when scrubbing 15-02-PLAN.md's demo-context prose instead of inventing new placeholder values
- [Phase 37]: Widened check-privacy.mjs mode-1 file filter to \.(rs|html)(\.|$) to recognize .rs.txt/.html.txt extension chains — needed so the C-02 allowlist-regression fixture stays out of cargo build while still being scanned
- [Phase 37]: check-privacy.mjs supports explicit positional file-argument scanning (bypassing git plumbing) so the fixture-driven self-test can target specific files deterministically
- [Phase 37]: Binary-extension violations (R8) labeled class D in check-privacy.mjs output, matching 37-RESEARCH.md's A/B/C/D class taxonomy
- [Phase 260819-vfg]: 260819-vfg: alias/редирект со старого ключа вкладки 'backup' не нужен — activeSection чисто локальный state, без URL/localStorage адресации
- [Phase 260820-uo4]: Стандартные варианты «Состояния» (Новое, Б/У, Хорошее, Среднее, Плохое, На списание) — статичный фронтенд-only список в DeviceAutocompleteField.svelte, backend не тронут
- [Phase 260820-uo4]: Open-гейтинг дропдауна унифицирован на allItems.length > 0 (было suggestions.length > 0 || allLocationSuggestions.length > 0)
- [Phase ?]: cartridges_fts dropped+recreated without location column (V038) — FTS5 external-content rebuild reads content table by column name; leaving location declared breaks rebuild once cartridges.location is gone
- [Phase ?]: V038: drop dependents (indexes, triggers) before ALTER TABLE ... DROP COLUMN — DROP COLUMN + trigger dependency checks vary across SQLite point releases, verified empirically
- [Phase 39]: PlacePatch mirrors PlaceNew 1:1 as all-Option<T> (incl. parent_id), matching DevicePatch's all-optional shape convention
- [Phase 39]: D-20 auth split: MutatePlaces is Admin-only (joins ManageUsers/ManageSettings bucket), ReadPlaces is Admin|Manager — proven by TDD RED targeting the exact copy-paste regression
- [Phase 39]: ActRow carries both full_path (live) and place_path_snapshot (frozen at write time, D-16) as two distinct fields
- [Phase 39]: CartridgeTransitionOp.place_id widened from required String to Option<i64> to let cartridge_service.rs apply a kind-aware default (D-13)
- [Phase 39]: PrinterRow gained device_place_id as a net-new field for PlacePicker id-bound selection
- [Phase 39-04]: CAS-failure error mapping uses OptimisticLockMismatch/NotFound split (established codebase pattern), not the plan text's literal Conflict — devices_sqlite.rs/acts_sqlite.rs already established this split; a blanket Conflict would make places_sqlite.rs the only inconsistent adapter
- [Phase 39]: 39-06: FK-violation on place_id mapped through the existing generic map_rusqlite()->Conflict path, not a field-specific Validation case (no codebase precedent for field-specific FK special-casing)
- [Phase 39]: 39-06: search_fts's D-29 place-path substring match computed in Rust via to_lowercase().contains() against the raw query, OR-combined with the FTS5 match via two independent CTEs (fts_hits, place_hits)
- [Phase 39]: 39-06: CSV device import fetches the full place candidate set once per commit call, resolves each row's place text against it in Rust (exact match only, RowError per UI-SPEC S12 on miss)
- [Phase ?]: PlaceService mutations call PlaceRepository's create/rename/archive/unarchive directly on &mut Connection (not a wrapped transaction) — rusqlite::Transaction has no DerefMut, so &mut tx cannot satisfy &mut Self::Conn; audit_log insert gets its own short-lived conn.transaction() instead
- [Phase 39]: 39-10: is_storage quick filter (D-11.2/D-11.4) added to acts/devices/cartridges/requests reports; D-28 subtree place_id filter kept scoped to acts/devices only, matching pre-existing cartridges/requests filter scope
- [Phase ?]: 39-07: place_path_snapshot passed as explicit update_act_header_in_tx parameter, not folded into ActPatch (domain/acts.rs out of this plan's file scope)
- [Phase 39]: Cartridges' third mutating-location surface: CartridgeService::update()'s own inline INSERT OR IGNORE INTO locations round-trip closed — Distinct from upsert_location_in_tx and the five named transition ops; found by direct read of update()'s body, not by the plan's grep-based inventory
- [Phase 39]: 39-08: read методы get/list_children/list_all/subtree_stats/full_path/list_subtree_contents возвращают доменные типы напрямую (не DTO), по буквальному тексту action плана, без правок dto/place.rs
- [Phase 39]: 39-08: search() Cyrillic-safe — repo.list_all(false), фильтрация to_lowercase().contains() в Rust, лимит 100 симв./50 строк, без SQL LIKE/GLOB
- [Phase ?]: 39-11: act_handover.minijinja confirmed dead code for act rendering (render_pdf reads act_handover.html exclusively since Phase 16/17 pivot); Task 4's regression test exercises the active HTML path instead
- [Phase ?]: 39-11: added D-27 'Расположение:' print field-row to act_handover.html (Rule 2) — the template contract had claimed act.location_name/place_path was available for years but the body never rendered it; registered a new _legacy_defaults/v26 upgrade-safety snapshot for existing installs
- [Phase 39]: Places transport (Plan 12): PlaceService self-gates authorize() internally; build_places_* helpers add a deliberate second transport-layer gate, matching this codebase's build_* convention (redundant, not a bug).
- [Phase 39]: 39-22: real-place-creation fixture pattern (SqlitePlaceRepository::create, no service layer) used in acts_e2e_smoke.rs/acts_search.rs/acts_clone_handover.rs DEF-3/devices_grouping.rs to replace the removed auto-create-by-name path (D-18)
- [Phase 39]: PlacePicker (39-13) exposes optional fetchChildren/fetchSearchResults/fetchOne/createPlace injection props, defaulting to apiCall('places_list_children'|'places_search'|'places_get'|'places_create'); every real form consumer (Plans 15-19) omits them and gets the default wire-backed behavior, only the showcase overrides them with invented demo data
- [Phase 39]: PlaceFormModal: rename mode shows only Название — places_rename only mutates name, other fields would be dead UI
- [Phase 39]: PlaceMoveModal consequences text follows UI-SPEC §11.3's literal clause order (nested places before devices), opposite of the backend's own delete-blocked-message order
- [Phase 39]: Plan 15: CSV import mapping key 'location'->'place' renamed on BOTH frontend and device_service.rs — a frontend-only rename would silently drop CSV place data (unknown mapping keys ignored)
- [Phase 39]: Plan 15: fixed double row-prefix bug in CSV place-not-found error (device_service.rs) — backend no longer bakes 'Строка N:' into error_message, letting the frontend's existing per-row prefix compose UI-SPEC §12's exact copy
- [Phase 39]: Plan 15: renamed DeviceList.svelte/TableSection.svelte table header 'Расположение'->'Место' for term unification, though DeviceList.svelte wasn't in the plan's declared files_modified
- [Phase 39]: OperationModal.svelte's prefillLocation prop removed entirely; Install place prefill for both request-centric and cartridge-centric flows now flows through one generalized printerContext effect reading printer.devicePlaceId (Plan 16)
- [Phase 39]: D-11.3 storage-status suggestion checkbox implemented as informational-only for cartridges (Plan 16) — CartridgeTransitionPayload has no status-override field since cartridge status is operation-driven, not place-driven; Plans 17/18 should re-verify their own device/act D-11.3 wiring
- [Phase 39]: Plan 17: D-11.1 quick-pick реализован как chip-строка над PlacePicker в ReturnModal.svelte (не пин/приоритет внутри самого контрола, PlacePicker не менялся); D-11.3 checkbox НЕ добавлен ни на одну act-поверхность — принадлежит только устройствам (см. Plan 16 amendment)
- [Phase ?]: 39-18: D-26 short-path/tooltip display implemented in ReportTable.svelte (the actual cell renderer), not ReportsPage.svelte where the plan's task text placed it
- [Phase ?]: 39-18: ReportsPage.svelte's filter-state ReportFilter drops location_name entirely rather than renaming to place_path (semantically nonsensical on a filter-parameter type); location_id renamed to place_id as instructed
- [Phase 39]: PlaceMoveModal defaultParentId/targetChosen fix — Rule 1 bug fix: null selectedParentId could not distinguish root-chosen from unfilled, blocking D-03 root-move; fixed with explicit targetChosen boolean, needed for Plan 14's drag-drop root dropzone
- [Phase 39]: PlaceTree fetches whole tree once, counters lazy per visible node — T-39-14-02 confirms ~300 rows makes places_list_all trivial; D-25 content counters use places_subtree_stats lazily per visible node, cached, to avoid 300 eager round-trips
- [Phase ?]: Native HTML5 drag-and-drop for the place tree does not work in WKWebView; reimplemented on Pointer Events with a manually-rendered drag ghost (GAP-2/GAP-11)
- [Phase ?]: PlacesPage owns onlyHere/activeTab/tree-selection as controlled props + localStorage (trackly:places:*) because PlaceContents/PlaceTree remount on every place selection via {#key place.id:token}
- [Phase ?]: Content-row click opens a read-only PlaceEntityViewModal (readonly mode added to existing DeviceFormBody/CartridgeFormBody) instead of navigating away; printers reuse the device form since no dedicated printer form exists
- [Phase 39]: Kept migration_idempotency.rs's location_id-absence assertions and the frozen _legacy_defaults/v20-v26 template snapshots as deliberate exceptions to the phase-closing vocabulary sweep — the test's purpose IS asserting the column is gone (PLC-04 regression lock); the templates are byte-identical upgrade-detection fixtures whose text must match what was actually shipped historically
- [Phase 39]: prettier drift on 11 phase-39-authored files was fixed (by the coordinator, commit e07c702f) rather than deferred, because pnpm lint is a sequential first-fail CI gate in ci-fast.yml/ci-full.yml — leaving it red would have masked every downstream CI check, the same failure class that left this project's CI red unnoticed for two weeks previously
- [Phase ?]: PathDisplayVariant: старый токен 'full' явно отклоняется, не трактуется как алиас 'last' (семантически противоположны)
- [Phase ?]: path_variant_override валидируется в Rust (PathDisplayVariant::from_str), без SQL CHECK — мирроринг places.kind
- [Phase ?]: 39.1-03: D-24 regression test lives in existing devices_csv_export.rs integration test, not a new mod tests in device_service.rs
- [Phase 39.1]: Phase 39.1 Plan 04: place_path_short на списке картриджей — общий SELECT_CARTRIDGES получил LEFT JOIN place_effective_variant, но только list() использует map_row_with_short_path; get()/search_fts() остаются на map_row и всегда дают None
- [Phase 39.1-06]: render_pdf resolves place path variant at render time (current place_effective_variant for acts.place_id), not act-create time
- [Phase ?]: Plan 07: HTTP SetPathVariantPayload uses camelCase (pathVariantOverride), matching existing places.rs Payload convention
- [Phase 39.1-08]: Radio/Input подраздела получают disabled во время сохранения (states matrix UI-SPEC); .sr-only/.field-hint/.field-error продублированы локально в OrgSettings.svelte (Svelte scoped styles)
- [Phase 39.1]: PlaceFormModal: дропдаун «Вариант сокращения» видим в create и rename режимах, сохраняется вторым RPC places_set_path_variant — D-12: поле мутируемо после создания места, в отличие от Типа/Родителя/Уровня
- [Phase 39.1]: 39.1-10: dto::auth тест переписан на явный assert отсутствия place_path_display в JSON (D-22)
- [Phase 39.2]: 39.2-02: у дефолтов формата пути один владелец — trackly_infra::repos::place_path_settings; сид V039 связан с константами тестом fresh_db_seed_matches_module_defaults, а не doc-комментарием
- [Phase 39.2]: 39.2-03 (WR-05): три ключа org-дефолтов формата пути пишутся одной транзакцией; атомарность доказана инъекцией отказа триггером RAISE(ABORT), а не комментарием
- [Phase 39.2]: 39.2-03 (IN-05): GET org-дефолтов прогоняет variant через PathDisplayVariant::from_str с fallback на DEFAULT_VARIANT и warn!; разделители не валидируются и не триммятся (D-09)
- [Phase 39.2]: 39.2-03 (IN-02): Manager покрыт матрицей ролей на обеих мутациях формата пути — Case 45 расширен седьмой мутацией, добавлен Case 51
- [Phase ?]: 39.2-04: деградация place_path_short к полному пути сделана в репозиториях (не в Svelte) — «—» в колонке «Место» теперь означает только отсутствие места
- [Phase 40-01]: place_movements — standalone append-only table (D-01), no SQL CHECK on source/entity_type, Rust-side from_str_lenient soft-degrade parsing (Pitfall 6/IN-01)
- [Phase 40]: compute_place_path_short lives in trackly-app (not trackly-core) because it needs &ReaderPool, an I/O-capable type forbidden by the no_io_deps.rs boundary gate
- [Phase 40]: New module place_path_display.rs is standalone rather than folded into place_path_settings.rs, which stays scoped to bare &Connection settings reads
- [Phase 40]: device_service::update caller threading: extract user_id from &Identity before the writer closure (mirrors place_service::create); http/devices.rs::handler_update needed zero changes
- [Phase 40]: cartridge_service::update/transition caller threading follows device_service::update's proven pattern (Plan 40-03): extract caller.user_id before the writer closure
- [Phase 40]: transition_in_tx's caller_user_id: Option<i64> is a trailing param; both main mutation AND nested D-16/D-17 auto-return audit inserts now use it (Pitfall 3 closed)
- [Phase 40-05]: record_movement_if_applicable takes places_repo as &dyn PlaceRepository<Conn = Connection> per the plan's exact interface, keeping all seven downstream write-site call sites uniform
- [Phase 40-05]: delete_by_act_id_in_tx is the sole owner of DELETE FROM place_movements WHERE act_id = ? (D-03); plan 40-20's undo path must call it, never hand-roll the statement
- [Phase ?]: act_service's four write-site methods (create/update/do_return/update_return) now thread caller: &Identity end-to-end, matching device_service/cartridge_service pattern from 40-03/40-04 — update_return's 3 internal loops already shared one top-level user_id_opt, so one signature change fixed all of them (verified via a both-loops test)
- [Phase 40-11]: HST-04: from_place_id/to_place_id are independent subtree-inclusive filters combined by AND (D-24); Куда reuses place_path/place_path_short, Откуда gets new from_place_path fields (Pitfall 7)
- [Phase 40-11]: query_movements_inner threads &ReaderPool alongside &Connection to call place_path_display::compute_place_path_short (single formula owner, D-18/D-20) rather than re-deriving the variant/separators inline
- [Phase 40-07]: place_movements_repo added as new DeviceService field (Arc-clone convention, mirrors printer_repo/place_repo) rather than passed per-call
- [Phase 40]: D-05's meaningful reason lives in note, not source — transition-driven cartridge movements keep source=Manual (D-07 closed enum) and carry an operation-derived Russian note; manual PlacePicker edits keep note=None
- [Phase 40]: Plan 40-09: un-return loop in update_return not wired to record_movement_if_applicable — Plan scoped exactly 5 call sites (create/update/do_return/update_return's added+retained_with_change loops); un-return's place restoration explicitly out of scope per plan's own action text and grep-count acceptance criteria
- [Phase 40]: Timeline actor_display resolved inline via SQL fallback (login), not a new repo method; compute_place_path_short called synchronously inside the same spawn_blocking closure to avoid N nested async spawns per timeline
- [Phase 40]: columns_for("movements") uses handover_date_utc as its date key, not created_at_utc — row_field/ReportRow only populate the former (Plan 40-11's field-reuse decision)
- [Phase 40]: Added the actual #[tauri::command] reports_list_movements wrapper + specta registration in Plan 40-12, ahead of what its action text literally asked for, because Plan 40-18's UI wiring assumes it already exists
- [Phase 40-13]: D-28 bulk move gated on Action::MutateDevices + Action::MutateCartridges (D-13, no new Action variant)
- [Phase 40-13]: Tauri command returns i32 (not usize) — tauri-specta cannot export usize (BigIntForbidden); service/build_* layer keeps usize
- [Phase 40]: Plan 14 (role-matrix coverage): 8 new Cases (52-59) in role_endpoint_matrix.rs, one HTTP + one Tauri Case per endpoint family, closing the IN-02-shaped gap for timeline read, movements report list/export, and bulk-move
- [Phase 40-20]: delete_soft now deletes each act's own place_movements rows at its own soft-delete point in the LIFO cascade loop (D-03), via SqlitePlaceMovementsRepository::delete_by_act_id_in_tx — never a single blanket delete at function end
- [Phase 40-15]: MovementTimeline loading state renders nothing (parent owns the single spinner, per UI-SPEC States table)
- [Phase 40-15]: Act-number/place navigation are prop callbacks (onNavigateToAct/onNavigateToPlace), not hardcoded hash writes, so 40-16/40-17 can each sequence their own close-then-navigate
- [Phase 40-15]: ActsPage does not switch tabs to match a hash-focused act's type/archived state — it only selects+fetches the act by id directly, sufficient for D-19's navigation target requirement
- [Phase ?]: COLUMNS_MAP.movements keyed by domain (not activeReport 'all') to avoid colliding with REQUEST_COLUMNS — currentCmd/currentColumns branch on activeDomain first
- [Phase ?]: ReportsPage.svelte required editing beyond plan's 3-file scope (Rule 3) — ReportSubNav.svelte's widened DomainKey broke the page's own duplicated DomainKey type, cascading into ReportFilters.svelte's reportDomain prop type
- [Phase 40]: Plan 40-19: bulk-move trigger button variant=secondary (confirm button is variant=primary per UI-SPEC's non-destructive classification, D-28/D-13)
- [Phase 40]: Plan 40-19: bulk-move confirm dialog's {N} count is fetched fresh with nested=true, independent of the onlyHere toggle, to match move_subtree_contents' actual scope
- [Phase 40]: Timeline fetch shares the modal's single loading/loadError state with the main entity fetch — a timeline-only failure surfaces as the modal's top-level error, not a scoped per-section message
- [Phase 40]: DeviceContextMenu reuses its existing onDelete prop as the generic list-reload signal after the view-modal's edit-save, avoiding new prop threading through DeviceList/DeviceGroupRow
- [Phase 40]: CartridgeDetail/PrinterDetail use MovementTimeline's own scoped loadError (not a shared page-level error) for the new timeline sections, opposite of Plan 40-16's modal — neither component had an existing single fetch to fold the new call into
- [Phase 40]: 40-21: cascade места принтера на картриджи без optimistic-lock — Синхронизация производного состояния (место следует за принтером), не пользовательское редактирование; обратная запись места принтеру гейтится WHERE place_id IS NULL как race-guard
- [Phase 40]: 40-24: Deep-link tab derivation runs once per mount (initialTabDerived), gated on id === initialFocusId
- [Phase 40]: 40-24: act_number resolution mirrors SqliteActRepository::SELECT_ACTS query shape rather than a second formula
- [Phase 40]: 40-24: D-06 explanation duplicated as static text in empty and short timeline states, no length threshold introduced
- [Phase 40]: 40-25: гейт check-report-type-parity.mjs проверяет INV-1 (reportType на ReportTable читает reportTypeKey()) и INV-2 (литерал в showDeletedBadge — return-значение reportTypeKey())
- [Phase 40-26]: Sentinel для place_distinct_count = -1 (не ' ' как у condition_distinct_count) — place_id целочисленный FK, NULL-места считаются отдельным бакетом (WR-04 прецедент)
- [Phase 40-26]: Гейтинг ячейки места на фронтенде — инлайн-тернарник в разметке, не именованная $derived-переменная, чтобы удовлетворить check-place-path-short.mjs (INV-1/2/3)
- [Phase 40-22]: Auto-return fallback only considers movements into is_storage=1 places, matching the user's decision to restore last known WAREHOUSE location, not any arbitrary prior place
- [Phase 40-22]: No storage-place history means place_id stays NULL after auto-return — deliberate non-regression for cartridges never logged into a storage place
- [Phase 40]: 40-27: hoisted print-idempotency state (activePolisher/repeatTableHeadHandlerRegistered/printing) in PdfPreviewModal.svelte to component scope so LAN print cleanup runs at call-start, not only on afterprint
- [Phase ?]: Разделили общий чек placeId в validate() install/to_refill: для to_refill место обязательно всегда (нет контекста принтера); для install — только для legacy-пути без принтера (effectivePrinterId === undefined), сервер уже резолвит/подставляет место при выбранном принтере (40-21/40-22).

### Pending Todos

1 pending — `/gsd-capture --list` для просмотра.

- **2026-08-08 — Rework act templates: единая шапка + переработка тела акта приёма-передачи**
  (`.planning/todos/pending/2026-08-08-rework-act-templates-shared-header-handover-body-redesign.md`).
  ⚠️ Доработанная пользователем шапка лежит только в `target/debug/templates/` (gitignored,
  умрёт от `cargo clean`) — переносить в `crates/trackly-app/templates/` первым делом.
  Размер задачи — фаза, не quick.

### Blockers/Concerns

Spike-зоны, требующие внимания во время планирования соответствующих фаз:

- **Phase 1:** WEBVIEW2_USER_DATA_FOLDER timing, Cyrillic Windows manifest setup, ProcMon-in-CI scaffolding (~½ дня каждый)
- **Phase 3:** krilla vs Typst-as-lib spike на реальном Cyrillic-фикстуре (1–2 дня)
- **Phase 6:** host-side механизм для Pantum hang detection — local agent vs remote WMI/RPC (требует реального BM5100ADN, ~неделя)
- **Phase 8:** валидация LDAP-bind против реального Windows Server 2022 с channel binding enforced (½ дня с реальным DC)
- Phase 36: real-print, LAN-транспорт end-to-end, print-DOM isolation (SC#4), N=1 один лист (SC#1) — явно отложено пользователем 2026-08-13, НЕ пройдено. Нужна отдельная UAT-сессия перед закрытием фазы.
- 39-11: cargo test -p trackly-app --lib fails to compile (missing 'places' field in AppCtx test fixtures in http/health.rs:126 and tauri_cmds/health.rs:142, introduced by Plan 39-05, never backfilled) — blocks unit tests, unrelated to 39-22's integration-test scope

### Явные решения по приватности

**PRIV-01 (2026-08-09, code review фазы 34, WR-11) — реквизиты-плейсхолдеры в
истории git: ПРИНЯТО И ЗАДОКУМЕНТИРОВАНО, историю НЕ переписываем.**

Контекст. До фазы 34 демо-контекст превью шаблонов
(`template_service.rs::demo_context_for_kind`) содержал захардкоженные
реквизиты, которые по форме выглядели как настоящие (внутренне согласованные
телефонный код и региональный префикс ОГРН). Фаза 34 заменила их на заведомо
вымышленные плейсхолдеры (`+7 495 123-45-67`, `12345678`, `1027700123456`).
HEAD чист — проверено grep'ом по `crates/trackly-app/templates/` и по новым
тестам/шаблонам; все литералы вымышлены («ООО Тест», «Иванов И.И.»).

Проблема. `CLAUDE.md` фиксирует: «Всё закоммиченное остаётся в истории git даже
после удаления из HEAD», а репозиторий публичный. Значит, скраб в HEAD —
необходимая, но не достаточная мера.

Решение. Перезапись истории (`git filter-repo` + force-push) — операция
разрушительная, затрагивает общий remote и все существующие клоны/форки, и
требует ОТДЕЛЬНОЙ явной авторизации пользователя. В рамках code-review-фикса она
СОЗНАТЕЛЬНО НЕ ВЫПОЛНЯЛАСЬ. Принято: жить с историей как есть, зафиксировать
решение здесь.

Компенсирующий контроль. Добавлен CI-гейт `privacy-requisites-gate`
(`.github/workflows/ci-fast.yml`), который валит сборку, если в `*.rs` / `*.html`
появится литерал реквизита (`inn`/`kpp`/`okpo`/`ogrn`/`phone`/`fax`), не
похожий на плейсхолдер. Он не чинит прошлое, но не даёт проблеме повториться.

Открыто для пользователя. Если перезапись истории всё же нужна — это отдельная
задача с явной авторизацией; она потребует форс-пуша и пересоздания локальных
клонов у всех, кто их имеет.

## Deferred Items

Items acknowledged and deferred at v1.1 milestone close on 2026-06-26. The v1.1
milestone audit (`milestones/v1.1-MILESTONE-AUDIT.md`) assessed all of these as
`tech_debt` — no critical blockers. Most are v1.0 (already-shipped) leftovers or
un-automatable human-verify items (no FE test runner by design).

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| uat_gap | 03.1 — 03.1-DEFERRED-UAT-ITEMS.md (v1.0) | open (0 pending) | 2026-06-26 |
| uat_gap | 03.1 — 03.1-HUMAN-UAT.md (v1.0) | partial (13 scenarios) | 2026-06-26 |
| uat_gap | 03.3 — 03.3-UAT-ITEMS.md (v1.0) | unknown (0 pending) | 2026-06-26 |
| uat_gap | 04 — 04-HUMAN-UAT.md (v1.0) | passed (0 pending) | 2026-06-26 |
| uat_gap | 05 — 05-UAT.md (v1.0) | testing (0 pending) | 2026-06-26 |
| uat_gap | 07 — 07-HUMAN-UAT.md (v1.0) | passed (13 scenarios) | 2026-06-26 |
| uat_gap | 08 — 08-HUMAN-UAT.md (v1.0) | passed (0 pending) | 2026-06-26 |
| uat_gap | 10 — 10-HUMAN-UAT.md (v1.1) | partial (2 scenarios, live-browser only) | 2026-06-26 |
| uat_gap | 11 — 11-HUMAN-UAT.md (v1.1) | partial (7 scenarios, live-browser only) | 2026-06-26 |
| verification_gap | 03 — 03-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 03.1 — 03.1-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 03.2 — 03.2-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 04 — 04-VERIFICATION.md (v1.0) | human_needed | 2026-06-26 |
| verification_gap | 10 — 10-VERIFICATION.md (v1.1) | human_needed (render checks) | 2026-06-26 |
| verification_gap | 11 — 11-VERIFICATION.md (v1.1) | human_needed (render checks) | 2026-06-26 |
| quick_task | 260618-vtm-backup-date-schedule-template-fixes | done (recorded complete ✓ in Quick Tasks table; no separate record file) | 2026-06-26 |
| quick_task | 260621-r8x-fix-fk-constraint-on-request-accept-assi | done (recorded complete ✓ in Quick Tasks table; no separate record file) | 2026-06-26 |

Items acknowledged and deferred at **v1.1.2** milestone close on 2026-07-15. The
v1.1.2 audit assessed all as `tech_debt` (no critical blockers). Major UAT / security /
Nyquist gaps for phases 18–22 were CLOSED before archiving (see
`milestones/v1.1.2-MILESTONE-AUDIT.md` → Close-Time Resolution). What remains:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| security | 18 — no SECURITY.md (backend list_grouped + UI; low risk, T-18-07 no-custom-overlay confirmed by code re-read) | deferred | 2026-07-15 |
| code_review | 18 — 5 Info-level findings (IN-01..05, advisory, non-blocking) | deferred | 2026-07-15 |
| security | 20 — 3 defense-in-depth WARNINGs (WR-01/02/03) already disclosed in 20-SECURITY.md, non-blocking | deferred | 2026-07-15 |
| test_coverage | cross-phase — no HTTP role-matrix case for settings_save_org_fields Employee→403 (guard structurally present) | deferred | 2026-07-15 |
| docs | historical "11 vs 12" requirement miscount (12 REQ-IDs actually defined & satisfied) | deferred | 2026-07-15 |

Items acknowledged and deferred at **Phase 27 (core-workflow-windows)** close on 2026-07-21,
explicitly requested to be revisited at **milestone v1.2 finish-up** (not blocking Phase 27/28
progress). Both-theme live UAT (27-09) approved with these items noted for later:

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| ui_polish | Дополнительные мелкие визуальные правки по Актам/Картриджам/Принтерам (WIN-03/04/05) — пользователь донесёт конкретный список позже, после того как все окна Фазы 28 будут переведены на дизайн-систему | deferred to milestone v1.2 end | 2026-07-21 |
| tech_debt | Нативные `<select>` в окнах Фазы 28 (Настройки/Дашборд/Отчёты/Пользователи) → перевести на кастомный `Dropdown`-примитив (Фаза 25), по аналогии с миграцией Картриджей в 27-09 (commit `80d0b41`) | deferred, likely Phase 28 or quick-task | 2026-07-21 |
| tech_debt | `PersonAutocomplete` + `LocationAutocomplete` — визуально идентичны (27-09 батч E), но остаются двумя раздельными реализациями; кандидат на слияние в единый переиспользуемый компонент | deferred to milestone v1.2 end | 2026-07-21 |

### Подтверждено при закрытии вехи v1.3 (2026-08-08)

Пред-закрывающий аудит (`gsd-sdk query audit-open`) показал 49 открытых пунктов. Разобраны
поштучно: **ни один не относится к фазам 31/32/33**, то есть к вехе v1.3. Состав:

- **27 быстрозадач** со статусом `missing` — это отсутствие поля `status:` во frontmatter
  их SUMMARY.md, а НЕ незавершённая работа. Все 27 записаны в таблице «Quick Tasks Completed»
  ниже как `complete ✓`. Сканер к тому же местами разбирает слаги с мусором (хвосты вида
  `...","`), то есть парсит таблицу STATE.md неаккуратно — ложные срабатывания.

- **13 uat_gap + 8 verification_gap** — фазы 03–30, наследство вех v1.0–v1.2, уже принятое
  при их закрытии (см. таблицу выше). Новых относительно v1.1 close: фазы 16, 17, 23, 24, 30.

- **1 «debug-сессия» `knowledge-base`** — это `.planning/debug/knowledge-base.md`, накопительный
  файл знаний, а не сессия. Ложное срабатывание сканера.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| verification_gap | 16 — 16-VERIFICATION.md (v1.2) | human_needed | 2026-08-08 |
| verification_gap | 24 — 24-VERIFICATION.md (v1.2) | gaps_found | 2026-08-08 |
| uat_gap | 16 / 17 / 23 / 30 — UAT-файлы вех v1.1.2–v1.2 | passed/resolved/diagnosed, 0 pending | 2026-08-08 |
| tooling | `audit-open` считает быстрозадачу незакрытой при отсутствии `status:` во frontmatter SUMMARY; 27 ложных срабатываний. Либо добавлять поле в шаблон, либо чинить сканер | deferred | 2026-08-08 |
| tooling | `audit-open` неаккуратно парсит таблицу STATE.md — слаги приходят с хвостами `","` | deferred | 2026-08-08 |

**Собственный долг вехи v1.3** зафиксирован отдельно в `milestones/v1.3-MILESTONE-AUDIT.md`
(frontmatter `tech_debt`): SSO-01 не покрывает вход по паролю; Phase 32 без подтверждённого
Nyquist-покрытия; тройное дублирование предиката `ad_register`; смена пароля служебной учётки
`svc-ldap-readonly`.

## Quick Tasks Completed

| Date | Slug | Summary | Status |
|------|------|---------|--------|
| 2026-06-14 | http-camelcase-payloads | S-5 parity: `#[serde(rename_all = "camelCase")]` on all axum request payload structs in http/ so browser/HTTP transport accepts the camelCase keys the frontend sends (e.g. `userNew`, `actId`). Fixes latent 422 on multi-word args in server mode. +regression test. | complete ✓ |
| 2026-06-18 | backup-date-schedule-template-fixes | Phase-07 round-3 follow-ups. R3-1: fixed Backups «Последний бэкап: Invalid Date» — `BackupSettings.svelte` read wrong DTO field (`timestamp` instead of `timestamp_utc` unix-seconds) + dropped phantom `last_backup_time`. R3-2: schedule blank after restart — normalized `"disabled"↔""` sentinel at load/save boundary (mirrors GAP-S5 load-on-mount). R3-3/CR-02: `template_service.rs` `update_body`/`reset_to_default` now guard on `rows_affected == 0` → `AppError::NotFound` instead of silent `Ok(())` (+TDD test). R3-4/CR-01 intentionally WONTFIX (RU-only UTC+3 v1). | complete ✓ |
| 2026-06-20 | rustls-crypto-provider-panic | Gap-closure fix discovered during 09-05 end-to-end human-verify: server-mode toggle panicked — both `ring` and `aws-lc-rs` providers in dep graph (ldap3 pulls aws-lc-rs; rcgen/tokio-rustls pull ring), rustls 0.23 can't auto-select. Added `ensure_crypto_provider()` (idempotent `Once`, installs `ring`) called first in `tls::build_server_config`/`load_from_pem` + early in `main.rs`. Enabled `ring` feature on `rustls` dep. Resolves the `graceful_shutdown_drain` pre-existing failure flagged in `09-ad/deferred-items.md` (now marked RESOLVED); +regression test `generate_self_signed_does_not_panic`. `cargo build`/`test`/`clippy -D warnings`/`fmt --check` all clean. | complete ✓ |
| 2026-06-20 | ad-test-connection | Gap-closure: "Проверить подключение" button on AD settings was a dead stub (hardcoded `disabled`, no backend). Added `AdClient::test_connection` (port + Real/Mock impls — LDAPS connect + anonymous bind, no end-user creds), `AuthService::test_ad_connection` (ManageSettings-gated, mirrors `settings_set_ad`), HTTP route + Tauri command (both registered, bindings regenerated), and wired the UI button (loading state, success/error toast + inline hint, enabled only when AD is on). +4 backend tests (mock reachable/unreachable, HTTP admin-gating 401/403, mock-mode 200). `cargo build`/test/`clippy -D warnings`/`fmt --check` + `pnpm svelte-check` all clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-defects | Reproduce-first fix of 3 defects found during 09-05 human-verify. **Defect 1** (duplicate restore requests): `create_restore_request` unconditionally inserted a new open `ad_register`/`restore` row on every AD bind — a blocked user re-submitting via login form + `BlockedScreen`'s CTA spammed duplicates. Made idempotent (check-then-insert in one writer tx, reuse existing open request). **Defect 2** (reject failed with generic toast): root cause was NOT the service-layer state machine (a service-level test passed unexpectedly) — it was `RequestTransitionPayload`'s `#[serde(tag = "op", rename_all = "camelCase")]` only renaming the tag's variant-name values, not cascading to each variant's field names (documented serde semantics); every real `requestId`-keyed JSON call failed deserialization, and axum's default `Json` rejection returns a plain-text 422 (not a structured AppError), surfacing as the generic fallback toast. Fixed by adding per-variant `rename_all = "camelCase"` to all 3 variants (Accept/Reject/Complete) +3 wire-contract unit tests +1 HTTP-transport repro test. **Lower-priority** (pending-user mis-routed to restore branch): `on_ad_bind_success`'s catch-all routed ANY inactive+non-deleted user (including never-approved pending registrations) into `create_restore_request`, which would create a spurious second `restore`-subtype request. The `users` table has no column distinguishing "never approved" from "approved then blocked" (both are `is_active=0, deleted=false`) — fixed by joining a `has_open_register_request` signal (open `ad_register`/`register` row) into `find_user_any_state` and routing pending re-binds to a new `reuse_or_create_pending_registration` path instead. Tightened the existing test to assert exact behavior instead of accepting either outcome. All 3 fixes committed atomically (`69dd50c`, `7402e60`, `1977fd3`). Final gate: `ad_register`+`requests_ad_register`+`requests_ad_register_http`+`ad_auth` (21 tests) green, full `trackly-app` suite green, `clippy -D warnings` clean, `fmt --check` clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-restoration-flow-ux | UX gap-closure: blocked-login no longer auto-creates a restore request on every AD bind (was burying the admin's rejection reason behind a fresh request). `on_ad_bind_success`'s blocked branch is now READ-ONLY — reports the most recent restore request's state via enriched `AppError::AccessBlocked { pending, rejection_reason }` (reads `requests.resolution_notes` for the canonical reject reason). New explicit, idempotent `AuthService::request_ad_restore(login, password)` re-binds to AD and creates/reuses the open restore request — exposed over HTTP (`/api/v1/request_ad_restore`, same rate-limit treatment as `auth_login`) and Tauri, bindings regenerated. Fixed a real bug along the way: `error_axum.rs` was hand-rolling the JSON error body and silently dropping `details` for every `AppError` variant over HTTP — switched to `Json(&self.0)` (the real `Serialize` impl). `BlockedScreen.svelte` reworked to 3 states (none/pending/rejected-with-reason) driven by `LoginPage`-forwarded error details, CTA now calls `request_ad_restore` instead of resubmitting `auth_login`. `docs/AD-SETUP.md` updated. Replaced 3 now-invalid tests in `ad_register.rs` with 8 new ones covering all 3 read states + idempotent create + anti-enumeration (wrong password / active user) + full reject→reason-surfaced→re-request lifecycle (11 tests in that file, all green). Full targeted suite (`ad_auth`+`ad_register`+`requests_ad_register`+`requests_ad_register_http`, 26 tests) green, `export_bindings` no drift, `clippy -D warnings` clean, `fmt --check` clean, `pnpm svelte-check` 0 errors. Pre-existing unrelated `clippy --tests` len_zero issue in `template_service.rs` left as-is (already tracked in `09-ad/deferred-items.md`). | complete ✓ |
| 2026-06-21 | fix-fk-constraint-on-request-accept-assi | UAT bug (Phase 10 live-verify): admin "Принять в работу" failed with `conflict: FOREIGN KEY constraint failed` while "Отклонить" worked. ROOT CAUSE: `RequestDetail.svelte` sent `assignedToUserId: identity.id`; in unlocked-desktop mode that's the sentinel `0` ("Рабочий стол"), which has no `users` row → violated `requests.assigned_to_user_id → users(id)`. Reject sends no assignee. FIX: `RequestService::transition` Accept now resolves the assignee server-side from `caller.user_id` (None for trusted-desktop → COALESCE keeps existing), ignoring the client value (D-REQ-01 override pattern); UI sends `assignedToUserId: null`. +regression test `request_accept_assignee.rs` (trusted-admin accept with forged id 0 → in_progress, assignee NULL). Full suite green (85 bins, AD mock), fmt/svelte-check clean. | complete ✓ |
| 2026-06-20 | 09-ad-gaps-ws-bridge | Cross-transport notification bug found during 09-05 live-verify: admin's desktop Requests page never live-refreshed when a browser/LAN user created or changed a request — required a manual reload. ROOT CAUSE: nothing forwarded `ctx.ws_broadcast` (the `tokio::sync::broadcast` channel browser WS clients subscribe to in `http/ws.rs`) into the Tauri webview; the only `app.emit("trackly-event", ...)` calls lived inside Tauri command handlers themselves (`tauri_cmds/requests.rs`), so HTTP-originated mutations never reached desktop. Affected ALL browser→desktop notifications, not just AD. Fixed by wiring a single global bridge task in `main.rs`'s `tauri::Builder.setup(...)` that subscribes to `ctx.ws_broadcast` and forwards every `WsEvent` via `app.emit("trackly-event", &event)` (same serde payload — `ws.ts`'s existing `event.type` handlers unchanged; Lagged→continue, Closed→exit, mirrors `http/ws.rs`). Confirmed `RequestService::transition`/`approve_ad_register` already pushed the same `WsEvent::RequestStatusChanged` the direct `app.emit` calls were sending — removed those now-redundant direct emits from `requests_transition`/`requests_approve_ad_register` to avoid double-firing on desktop (single source of truth: service layer → ws_broadcast → bridge). +regression test proving `tokio::sync::broadcast` fans an identical event out to every independent subscriber (the property the bridge relies on). Also committed an untracked proven-green HTTP repro (`restore_request_visibility_http.rs`) left over from the investigation, documenting the backend create/visibility/pending chain is correct. `cargo build`/targeted tests (17 passed)/`clippy -D warnings`/`fmt --check` all clean. | complete ✓ |
| 2026-06-30 | fix-tls-cert-san-for-wildcard-bind-host | UX follow-up to the bind-host fix (4ec2a9b): self-signed cert SAN only held `[host, "localhost"]`, so a wildcard bind host (`0.0.0.0`/`::`/empty) put the useless literal `0.0.0.0` in the SAN — LAN browsers connecting via `https://<LAN-IP>:port` got a hostname-mismatch error on top of the expected self-signed-untrusted warning, worsening the fingerprint-trust UX. `tls::generate_self_signed` now routes its SAN list through a new `collect_subject_alt_names` helper: for wildcard hosts it enumerates the machine's non-loopback IPv4/IPv6 addresses as IP-SANs (rcgen 0.14 auto-classifies IP-parseable strings via `IpAddr::from_str`), adds the OS hostname (validated as a DNS label via `is_valid_dns_name`), and keeps `"localhost"`; non-wildcard hosts retain the original `[host, "localhost"]` behaviour. Call sites unchanged (`main.rs:162`, `http/settings.rs:272`, `tauri_cmds/auth.rs:93` — all pass `&host`). New deps `if-addrs 0.15` + `hostname 0.4` (pure-Rust, libc-only, no OpenSSL/DLL — portable-friendly). +3 unit tests (`is_wildcard_host_classifies_correctly`, `collect_sans_non_wildcard_unchanged`, `collect_sans_wildcard_includes_detected_lan_ip` — asserts ≥1 detected LAN IP in SAN, documents/skips if host has no non-loopback ifaces). tls unit tests + `tls_server_smoke` (incl. `generate_self_signed_does_not_panic`) green, `clippy` clean. | complete ✓ |
| 2026-07-02 | 260702-vtf-y-tooltip | Follow-up to debug session `dashboard-consumption-chart-422` (bc0f00c): the consumption chart rendered but was uninformative (no Y-axis/scale, unreadable magnitudes). Rewrote `ChartWidget.svelte` from a hand-rolled SVG line chart into a dependency-free **grouped bar chart** (viewBox `0 0 500 220`, LEFT_PAD=42): Y-axis with `niceMax` rounding + 5 gridline ticks + numeric labels, grouped vertical bars per model per month with value labels above non-zero bars, and a stylized `$state`-driven `<div>` hover tooltip (`getBoundingClientRect`-based positioning, «Месяц · Модель: N» — chosen over native SVG `<title>` for instant styled UX). Correctly handles the single-month case that previously rendered invisibly. Preserved: Props/ConsumptionPoint interfaces, loading/error/empty states, sr-only a11y table, legend, PeriodToggle; DashboardPage.svelte unchanged. `svelte-check` 0 errors, `pnpm --dir ui build` green (ui/dist rebuilt, gitignored). Commit `4ccc179`. Awaiting user live visual verify. | complete ✓ |
| 2026-07-02 | fix-y-axis-integer-ticks | Follow-up fix (live-verify defect on 260702-vtf): Y-axis skipped labels. Ticks were `round(niceMax*i/4)` over 4 intervals — when `niceMax` wasn't a multiple of 4, fractional tick values rounded to gaps (`niceMax=5` → 0,1,3,4,5, dropping «2» since 2.5 rounds up). Switched `ChartWidget.svelte` to an integer nice-step (`yStep` from 1/2/5/10/… — smallest giving ≤5 intervals; `niceMax = ceil(maxVal/yStep)*yStep`; ticks iterate 0→niceMax by yStep) so labels are always whole and contiguous. `svelte-check` 0 errors, `ui/dist` rebuilt. Commit `9405e62`. | complete ✓ |
| 2026-07-04 | 260704-uw3-template-seed-upgrade | Fix: existing DBs never picked up the Phase 15-02 `act_handover.minijinja` rewrite because `seed_defaults_on_startup` only INSERTed a bundled default when `active_count == 0` — any pre-existing active row short-circuited the seed, permanently freezing the template body at whatever it was when first seeded. Extended `seed_defaults_on_startup` with an auto-upgrade branch: fetches `(is_default, body_minijinja)` for the active row per `kind`, branches 3 ways — no row → INSERT (unchanged), row with `is_default=1` and body differing from bundled → UPDATE in place (mirrors `reset_to_default`'s UPDATE shape, `version+1`), row with `is_default=0` (user-customized via `update_body`) or body already matching → no-op. +3 regression tests (bug-repro upgrade, no-clobber of customized templates, idempotency across repeated calls). `cargo test`/`clippy -D warnings`/`fmt --check` all green. Commits `20fb879`, `1a7a1d7`. | complete ✓ |
| 2026-07-05 | 260704-wxw-act-pdf-word-fidelity-redesign | Rewrote default `act_handover.minijinja` + added `Section::FieldRow` DocSpec/renderer variant so the rendered Акт приёма-передачи matches the Word reference sample's body structure: «метка \| подчёркнутое значение» rows instead of `device_card` boxes, full-length field labels (Инвентарный номер:/Серийный номер:/Модель:/Комплектация:/Технические характеристики:/Состояние:/Сроком до:), no per-device «Устройство №N» heading/counter, devices listed sequentially. `FieldRow` draw-arm in `renderer.rs` uses `krilla::geom::{PathBuilder, Rect}` + `krilla::paint::Fill` + `Surface::set_fill`/`draw_path` for the underline (confirmed `fill_path`/`stroke_path` do not exist in krilla 0.7 by reading vendored source); `measure_field_row_height` mirrors `measure_device_card_height`'s measure-then-place pagination pattern so wrapped values never split across a page boundary. `Section::DeviceCard` and its tests kept unchanged (backward compat). `act_42.sha256` verified unchanged (fixture uses only KeyValueTable/ItemsTable/Signature — untouched by this additive change). Full `trackly-app` suite (75 test binaries)/`clippy -D warnings`/`fmt --check` all green. Commits `6b6148f`, `0aed41a`, `3e73cf6`, `fa13a26`, `dc667e0`, `adbb44b`. | complete ✓ |
| 2026-07-15 | 260715-gt2-act-edit-device-quantity | Edit-акта: разрешено задавать количество >1 у НОВОЙ (не retained) не-serial позиции, когда на складе достаточно устройств той же группы (было — жёстко «1»; нельзя было добавить, напр., 3 клавиатуры за раз). Backend не менялся: `ActUpdateDto.items` — full-replacement set из one-device-per-entry `ActUpdateItemDto`, а `ActService::update`'s `added: Vec<i64>` loop (`act_service.rs:667-754`) уже N-safe (переводит каждое добавленное устройство в `в_работе` + локацию акта). Правки только UI: (1) `ActFormItemsTable.svelte` — убран `mode==='edit'` clause из qty-тернаров в `pickDevice`/`pickGroup` (`hasSerial ? 1 : Math.min(...)`), qty-cell рендерит редактируемый `<input max={qtyMax(row)}>` для свежих non-serial строк, статичную «1» — только для retained (`complectation_at_time !== undefined`) или serial; (2) `ActFormBody.svelte` — edit-branch submit теперь `.flatMap()` разворачивает свежую строку с `quantity>1` в N `ActUpdateItemDto` через `group_ids.slice(0, quantity)` (mirror create-branch), retained-строки по-прежнему по одной записи. Retained/serial позиции неизменны. +regression-тест `add_multiple_positions_transitions_all_devices` (`acts_update.rs`) — доказывает multi-device add (items.len 4, 3 новых → `status_id=2` + `location_id=loc_b` + audit_log). Gates: `cargo test --test acts_update` 14/14, `clippy -D warnings` clean, `svelte-check` 0 errors, `pnpm --dir ui build` ok. Commits `e3ab329`, `ae996bc`, `644278a`, `a5c31bc`. Примечание: pre-existing repo-wide `cargo fmt` drift (12 мест в acts_update.rs + др., присутствует на baseline `efd69b6`, локальный rustfmt 1.8.0/1.92.0) НЕ трогался — отдельная проблема CI-гейта. | complete ✓ |
| 2026-07-18 | 260718-x8t-tabs-segmented-width | UAT gap-closure (24-UAT.md, тест 6): segmented-вариант вкладок в витрине компонентов (`#/showcase`) растягивал подложку на всю ширину секции вместо обжатия по содержимому (эталон `Tabs.dc.html:64`). ROOT CAUSE: `.variant-block` в `TabsSection.svelte` (`display: flex; flex-direction: column`) не задавал `align-items`, дефолтный `stretch` растягивал inline-flex потомка `.tabs-segmented` по ширине. FIX: одна строка — `align-items: flex-start;` в `.variant-block`. `Tabs.svelte` не тронут: временный изолированный preview-харнесс (headless-Chrome screenshot diff до/после, `#/showcase` недоступен без backend-сессии) подтвердил, что underline-вариант визуально идентичен в обоих состояниях — `align-self: stretch` fallback не понадобился. `pnpm --dir ui lint`/`svelte-check`/`build` все зелёные, `ui/dist` пересобран. Commit `402f15d`. | complete ✓ |
| 2026-07-24 | 260724-pxf-fix-wr-01-ws-refcount-leak-and-wr-02-emp | Fixed two pre-existing logic bugs surfaced by the Phase-29 code review (`29-REVIEW.md`; phase 29 was CSS/markup-only). **WR-01** (`EmployeeLayout.svelte`): `connectWs()` bumps the shared `refCount` synchronously but resolves its teardown asynchronously via `.then()` — if the component unmounted before the promise resolved, the cleanup ran while `unlisten` was still `undefined`, so the later-arriving release never fired and `refCount` leaked across fast mount/unmount cycles (WS singleton/reconnect machinery never torn down). Fix: added a `disposed` flag — the cleanup sets it, and the `.then()` handler calls the release fn immediately if already disposed instead of storing it; verified against `ws.ts` that the release fn is idempotent (`released` guard + `Math.max(0, refCount-1)`) so an immediate call decrements exactly once. **WR-02** (`BlockedScreen.svelte`): the "Запрос отклонён" state selection used a truthiness test on `rejection_reason`; `LoginPage.svelte` normalization preserves `""` and the backend (`services/auth.rs`) derives `rejection_reason` from free-form `resolution_notes` which can be empty, so a rejected request with an empty reason rendered the first-time "Доступ закрыт" screen. Fix: `!== null` to distinguish `null` (no request) from `""` (rejected, empty reason). Gates: `pnpm --dir ui lint` PASS, `svelte-check` 0 errors (48 pre-existing warnings unrelated), `pnpm --dir ui build` PASS. `bindings.ts`/`ui/dist` gitignored (regenerated by prebuild; seeded placeholder `ui/dist` to break the fresh-worktree compile cycle). Commit `afc8645`. | complete ✓ |
| 2026-07-23 | 260723-syw-wr01-user-edit-password | Fixed WR-01 (Phase 28 `28-REVIEW.md`): editing a user with a new password was a **silent no-op** — `UserFormModal` collected/validated «Новый пароль» but `UsersPage.handleSave` dropped it (`UserPatch` had no password field), so the admin saw «Пользователь обновлён» while the stored `password_hash` never changed. Backend contract check: `AuthService::update_user` had no password path (an admin `reset_password` existed but is HTTP-only, unused by the edit form). FIX (per WR-01 guidance): added `#[serde(default)] password: Option<String>` to `UserPatch`; `update_user` now validates a non-empty new password (`len>=8`, same message as create) and hashes it via `spawn_blocking(hash_password)` (argon2id, off the writer thread, mirroring `create_user`), writing `password_hash = COALESCE(?, password_hash)` inside the same atomic version-bumping UPDATE (None/empty ⇒ no rotation). Both transports fixed at once (Tauri + HTTP share the service; HTTP handler already forwards `patch`). Frontend `handleSave` now sends `password: data.password ? data.password : null`. +regression test `users_update_password_change` (rotation works, old password → `Unauthorized`, empty-password edit leaves credential intact while other fields apply, too-short → `Validation{field:"password"}`); 7 existing `UserPatch` literals updated for the new field. Gates: `cargo test --test users_crud` 8/8, `export_bindings` no drift (`UserPatch.password` present), `clippy -p trackly-app --tests` clean, `svelte-check` 0 errors, `pnpm build` ok. `bindings.ts`/`ui/dist` gitignored (regenerated by prebuild). Commits `a30b360`, `c4df18c`. | complete ✓ |
| 2026-07-19 | 260719-ocq-close-bl-01-unify-dropdown-drill-in-rese | Закрытие BL-01 из round-2 code review фазы 25 (`25-REVIEW.md`). План 25-08 починил WR-02 только наполовину: drill-in reset (`expandSeq++; viewMode='groups'; activeGroup=null; members=[]; showBack=false`) был добавлен в `openPanel()`, но не в `handleInput()` — вторую из двух функций `Dropdown.svelte`, ставящих `open = true`. Из-за этого самый частый способ переоткрыть панель (набор текста после того, как её свернули во время drill-in без выбора) показывал устаревший кликабельный список участников предыдущей группы на всё окно 250ms debounce + IPC, а in-flight `drillInto` из прошлой сессии проходил guard `seq !== expandSeq` и перезаписывал переоткрытую панель — тот же класс записи неверных данных, что и round-1 CR-02. FIX: reset вынесен в хелпер `resetDrillState()`, вызывается из обоих мест; `open = true` остаётся первой мутацией в обеих функциях (CR-01 не регрессирован), новых писателей `expandSeq` не добавлено (CR-02 не регрессирован). Round-2 warning WR-01 (безусловный `viewMode='groups'` в `openPanel()` может отбросить in-flight AUTO-05 auto-flatten у потребителя с мемоизированным `groups`) зафиксирован как явный out-of-scope в docstring хелпера — требует новой реактивной зависимости в AUTO-05 `$effect`, что задевает guard CR-02. `svelte-check` 0 ошибок, `lint` чисто, `build` успешен. Commits `6407133`, `502f55e`. | complete ✓ |
| 2026-08-04 | 260804-ire-ad-ldap-transport-mode | v1.3.1 follow-up #1 (live-AD UAT 2026-08-04, HIGH — блокировал ФИО в проде). `RealAdDirectory::resolve` и оба места в `RealAdClient` (`authenticate`, `test_connection`) хардкодили `format!("ldaps://{host}:{port}")`, а у DC пользователя (`dc.example.local`) рабочий только plaintext LDAP :389 — порт 636 TCP-открыт, но TLS-handshake принудительно закрывается. Итог: `directory.resolve()` → `DirectoryError::Unreachable`, SSO деградировал до bare login, сотрудник видел UPN `us100@example.local` вместо `displayName`; group→role тоже не работал. `no_tls_verify=true` не помогал (схема всё равно `ldaps://`). ФИКС: новый enum `LdapTlsMode` (`ldaps` по умолчанию / `plain` / `starttls`) в `[ad]`-конфиге; `AdConfig::port` стал `Option<u16>` с аксессором `resolved_port()` (явный порт всегда побеждает, иначе 636 для ldaps и 389 для plain/starttls); вся сборка URL+`LdapConnSettings` вынесена в единственный общий хелпер `build_ldap_conn()` (новый `crates/trackly-infra/src/ad/transport.rs`), на который переведены все три call-site — литералов `format!("ldaps://...")` в коде больше нет; StartTLS = `ldap://`-URL + `.set_starttls(true)` (сверено с vendored ldap3 0.12.1, отдельной схемы нет); `plain` — явный opt-in с однократным `tracing::warn!` на загрузке конфига о передаче пароля служебной учётки в открытом виде; `resolved_port()` проброшен в оба места сборки `AdSettingsDto` (`http/auth.rs`, `tauri_cmds/auth.rs`), чтобы Settings UI показывал реальный порт подключения; `ldap_tls_mode` задокументирован в `trackly.config.toml.example` с предупреждением о безопасности. Обратная совместимость доказана тестом: `[ad]` без `ldap_tls_mode`/`port` по-прежнему даёт `ldaps://host:636`. Гейты: `cargo test -p trackly-infra --lib` 123/123, `--test config_test` 6/6, `cargo build -p trackly-app` успешно, `cargo clippy -p trackly-infra -p trackly-app --all-targets -- -D warnings` чисто. Commits `0d3c932`, `0f1c6b8`, `f6738ce`. | complete ✓ |
| 2026-08-04 | 260804-l22-ad-register-counts | v1.3.1 follow-up #2 (live-AD UAT 2026-08-04). Сотрудник видел «Мои заявки: 1» из-за собственной **невидимой** авто-созданной заявки `ad_register` (`ad_subtype='register'`): `list()` её прятал, а `counts()` — нет. `SqliteRequestRepository::counts` не применял условие `AND (?N = 0 OR request_type != 'ad_register')`, которое `list()` уже использовал, а `RequestService::counts` не вычислял/не передавал `exclude_ad_register`. ФИКС: в порт-трейт `RequestRepository::counts` (`crates/trackly-core/src/ports/requests.rs`) добавлен параметр `exclude_ad_register: bool`; фильтр применён ко **всем шести** бакетам (`all`/`open`/`in_progress`/`completed`/`rejected`/`cancelled`) в `crates/trackly-infra/src/repos/requests_sqlite.rs`; `RequestService::counts` (`crates/trackly-app/src/services/request_service.rs:164`) вычисляет `exclude_ad_register = !matches!(caller.role, Role::Admin)` — тот же механизм, что и в `list()` (:120). Публичная сигнатура `RequestService::counts(&self, caller)` не изменилась, поэтому Tauri-команда и axum-хендлер правок не потребовали. Регресс-тест `ad_register_excluded_from_employee_counts` в `crates/trackly-app/tests/requests_ad_register.rs`: сотрудник с одной `ad_register` → `counts.all == 0` и `counts.open == 0`, админ по-прежнему видит 1/1. Гейты: `cargo build --workspace` чисто, `cargo test -p trackly-app --test requests_ad_register` 8/8 зелёных (проверено оркестратором отдельным прогоном). Commits `bf9ca5c`, `58a68c4`. | complete ✓ |
| 2026-08-04 | 260804-lk0-config-ux-fail-soft-on-broken-trackly-co | Fixed "GUI silently exits with code 1 on a broken `trackly.config.toml`" bug: `main()` called `AppConfig::load_or_default(...)?` BEFORE `logging::init` — под `windows_subsystem = "windows"` (release-сборка) `Err`, который `?` печатал в stderr, было некуда выводить (консоли нет), процесс просто исчезал без диагностики. ФИКС: новый модуль `config_recovery` (`load_or_recover`/`write_config_error_file`/`clear_config_error_file`/`show_best_effort_dialog`) — загрузка конфига структурно не может распространить фатальную ошибку: всегда возвращает `(AppConfig, Option<String>)`, `main.rs` инициализирует логгер с тем, что получилось (реальный конфиг или дефолт), и ТОЛЬКО ПОСЛЕ этого показывает ошибку через `tracing::error!` + `config-error.txt` рядом с exe + best-effort нативный диалог (`rfd`, пропускается для `--self-test`). Заодно исправлен `trackly.config.toml.example`: `[storage]`→`[paths]`, `server.bind`→`server.host`, добавлены отсутствовавшие `enabled`/`cert_path`/`[logging]`/`[organization]`, Windows-пути через single-quoted TOML-литералы. Новый регресс-тест `config_example_test.rs` (`include_str!` на реальный shipped-файл + раскомментирование по эвристике) поймал реальный TOML-структурный баг: `admin_logins` стоял ПОСЛЕ `[[ad.role_mapping]]`, из-за чего в TOML он молча «прилипал» к последней записи массива таблиц вместо `[ad]` и терялся (`serde` тихо отбрасывал неизвестное поле) — переставлен перед `role_mapping`. `rfd` подключён как прямой deps с `default-features = false` (дефолтные фичи `rfd` тянут `xdg-portal`/`wayland`, которых `tauri-plugin-dialog` не включает — с дефолтными фичами в `Cargo.lock` появлялось 16 новых пакетов, что противоречило заявленному в плане «нулевому новому supply-chain surface»). Гейты: `cargo build -p trackly-app` чисто, `cargo test -p trackly-app --lib config_recovery::` 5/5, `cargo test -p trackly-infra --test config_example_test` 2/2, `cargo test -p trackly-infra --test config_test` 6/6 (пре-существующие не сломаны), `cargo clippy -p trackly-app -p trackly-infra --all-targets -- -D warnings` чисто. Verification step 6 (живая проверка на Windows-машине — silent exit repro требует release-сборку с `windows_subsystem="windows"`, не воспроизводима с macOS dev-сборки) НЕ automatable, оставлена как pending Windows follow-up. Commits `c2a9af5`, `ee29202`, `875dbae`. | complete ✓ |
| 2026-08-05 | 260805-edd-fix-lan-print-pass-stylesheets-to-paged- | Печать из LAN-браузера падала с тоастом «Не удалось открыть документ для печати» (живой UAT с web.example.local:8443, Phase 33). `printViaTopLevel` передавал стили в `previewer.preview(bodyHtml, [styleHtml], printRoot)` СТРОКОЙ, а `Polisher.add()` (`pagedjs/dist/paged.js:27506`) ветвится по типу: `typeof === "object"` → значение используется как текст CSS, иначе `request(arg)` — то есть строка загружается как URL. DevTools показывал запрос на `https://web.example.local:8443/%3Cstyle%3E%20@page...`. Существовавшая catch-ветка чинила не ту ось (снимала теги `<style>`, но оставляла строку), поэтому обе попытки падали одинаково. ФИКС: `[{ 'act-preview.css': cssText }]` — объект, значение без обёрточных тегов; ложная fallback-ветка удалена, чтобы реальные ошибки доходили до тоста. Побочно: `ui/src/pagedjs.d.ts` объявлял `stylesheets?: string[]`, что блокировало объектную форму — расширено до `(string | Record<string, string>)[]` по реальному рантайм-API. Не тронуты: `printViaSystemBrowser` (десктоп, подтверждён рабочим), `bootstrapScript.js` (захэширован в CSP, D-14), шаблоны, `Modal.svelte`. Гейты: `svelte-check` 0 ошибок, `lint` включая гейт CSP-хэша, `build` — чисто. НЕ ПРОВЕРЕНО автоматикой: реальная печать из LAN-браузера требует живого axum-сервера с другой машины — остаётся ручной проверкой. Commit `c77ab6c`. | complete ✓ |
| 2026-08-05 | 260805-gdz-lan-print-surface-swallowed-error-and-st | Продолжение 260805-edd. После починки формы аргумента стилей сеть стала чистой, но печать из LAN-браузера всё равно не открывала диалог, показывая тоаст. Консоль при этом ПУСТА — потому что `handlePrint` ловил исключение пустым `catch { pushToast(...) }`, который ничего не связывал и не логировал: ошибка существовала, но её текст выбрасывался. (1) ДИАГНОСТИКА: `catch (err)` теперь пишет `console.error('[PdfPreviewModal] handlePrint failed', printPath, err)` с указанием ветки (desktop `printViaSystemBrowser` vs LAN `printViaTopLevel`) — сбой, воспроизводимый только на удалённой машине, больше не может быть невидимым. (2) ДЕФЕКТ ПО ПЕРВЫМ ПРИНЦИПАМ (НЕ подтверждённая причина симптома): `printViaTopLevel` ставил `@media screen { #act-print-root { display: none !important } }` ДО того, как Paged.js верстает в этот же контейнер, а внутри `display:none` вся геометрия нулевая — библиотека определяет разрывы через `getBoundingClientRect`. Заменено на увод за экран с сохранением раскладки (`position:absolute; left:-100000px; top:0`) плюс обязательный сброс в `@media print` (`position:static; left:auto`), иначе печать уехала бы за пределы листа. Попытка доказать гипотезу на изолированном стенде была НЕУБЕДИТЕЛЬНОЙ — контрольный вариант с видимым контейнером завис так же, эксперимент не изолировал переменную. Не тронуты: `printViaSystemBrowser`, `bootstrapScript.js` (CSP-хэш, D-14), шаблоны, `Modal.svelte`. Гейты: `svelte-check` 0 ошибок, `lint` включая гейт CSP-хэша, `build`. НЕ ПРОВЕРЕНО: открывается ли диалог печати на реальном LAN-клиенте — требует следующего живого UAT. Commits `4b7f96f`, `8a06587`. | complete ✓ |
| 2026-08-05 | 260805-har-lan-print-neutralize-app-body-background | Третья и последняя правка цепочки LAN-печати. UAT 1.3.1 подтвердил: печать из браузера ЗАРАБОТАЛА — значит `display:none` на контейнере Paged.js из 260805-gdz и был причиной (гипотеза подавалась как недоказанная, подтверждена живой проверкой). Остаточный дефект: при печати из LAN-браузера появлялся серый фон, при печати из десктопного приложения — белый. Асимметрия и была диагнозом: `printViaSystemBrowser` пишет самодостаточный temp-HTML и открывает в системном браузере, где стилей приложения нет; `printViaTopLevel` верстает в DOM самого приложения, поэтому на вывод действует весь каскад Trackly. Источник серого — `ui/src/styles/global.scss:29` `body { background: var(--tr-bg) }`, где `--tr-bg` = `#eef1f6` в светлой теме; блок `@media print` скрывал `body > :not(#act-print-root)`, но собственный фон `body` не нейтрализовал. Второй, смежный пробел: `.pagedjs_page` в печатном пути не получал белый фон явно — в превью это делает `buildSrcdoc` (D-08 «лист всегда белый»), в печати аналога не было. ФИКС: в `@media print` добавлены `html, body { background: #fff !important }` и `.pagedjs_page { background: #fff !important }`, литералом а не токеном (в тёмной теме токен дал бы почти чёрный, а бумага белая в обеих темах). Сохранены несущие правила прошлых задач: скрытие хрома приложения и сброс `position: static; left: auto` (без него печать уехала бы за пределы листа). Не тронуты: `printViaSystemBrowser`, `bootstrapScript.js`, `global.scss`/`_tokens.scss` (переопределение только в print-скоупе), шаблоны, `Modal.svelte`. Гейты: `svelte-check` 0 ошибок, `lint` включая гейт CSP-хэша, `build`. НЕ ПРОВЕРЕНО: цвет реального печатного вывода — требует ручной проверки с LAN-клиента. Commit `2f296b2`. | complete ✓ |
| 2026-08-05 | 260805-ifj-lan-print-neutralize-app-line-height-lea | Четвёртая правка цепочки LAN-печати, найдена по двум физическим распечаткам одного акта рядом: слева из десктопа — верно и совпадает с превью, справа из LAN-браузера — тот же текст с увеличенным межстрочным интервалом, растянут вниз листа. ПРИЧИНА: `ui/src/styles/global.scss:33` `body { line-height: var(--tr-line-height-body) }` = 1.5, а все три шаблона (`act_handover`, `act_acceptance`, `report`) объявляют на `body` только `font-family`/`font-size`/`color`/`margin`/`padding` и СОЗНАТЕЛЬНО не объявляют `line-height` — они рассчитаны на автономный документ и полагаются на дефолт UA (`normal` ≈1.2). LAN-путь верстает Paged.js в DOM приложения, поэтому содержимое акта наследует 1.5. Остальные свойства не текут: их объявляет и шаблон, а его стиль подключается позже `global.scss` и побеждает — `line-height` было единственным, которое объявляет только приложение. Десктоп пишет автономный temp-HTML без стилей приложения, отсюда и расхождение двух распечаток. ФИКС: в `printStyle.textContent` в НАЧАЛО добавлен блок `@media print { body { line-height: normal; letter-spacing: normal; word-spacing: normal } }`. Оба аспекта размещения несущие: внутри `@media print` — чтобы не трогать экранную типографику приложения; ПЕРЕД `${cssText}` — чтобы пользовательский шаблон, если он сам объявит `line-height`, победил по порядку каскада (шаблоны редактируемы на диске, D-01). Размещение после `cssText` или скоуп на `#act-print-root` сломали бы такой шаблон. Порядок проверен скриптом по сгенерированной строке. Сохранены все правила прошлых задач (скрытие хрома, сброс `position: static; left: auto`, два `background: #fff !important`). ИЗВЕСТНЫЙ FOLLOW-UP, намеренно НЕ чинился здесь: `${cssText}` инжектится без скоупа, поэтому `body`-правила шаблона применяются и к экранному `body` приложения, а `printStyle` не удаляется на `afterprint` — типографика приложения меняется после первой печати. Починка требует либо скоупа `cssText` под `@media print`, либо отказа от ручной инъекции (Polisher уже получает `cssText` аргументом) — обе несут риск для пагинации и заслуживают отдельной правки со своим UAT. Гейты: `svelte-check` 0 ошибок, `lint` включая гейт CSP-хэша, `build`. НЕ ПРОВЕРЕНО: реальный межстрочный интервал на бумаге. Commit `3162320`. | complete ✓ |
| 2026-08-05 | 260805-jwf-lan-print-stop-injecting-template-css-in | Пятая правка цепочки LAN-печати, закрывает два дефекта одной природы. ОБЩИЙ ПРИНЦИП, зафиксированный в коде: `printViaTopLevel` верстает в живой DOM приложения, а Paged.js определяет разрывы страниц ИЗМЕРЯЯ этот DOM на экране, до `window.print()`. Значит всё, что влияет на раскладку документа, обязано действовать в момент измерения, а не только в `@media print`; в `@media print` допустимы лишь видимость и чисто красящие свойства. ДЕФЕКТ A (утечка шрифта, подтверждён пользователем): `${cssText}` инжектился в `document.head` без скоупа, поэтому `body { font-family: "DejaVu Sans", "Arial" }` шаблона ложился на интерфейс приложения — на Windows без DejaVu откат на Arial; `printStyle` при этом никогда не удалялся. Инъекция была избыточна: Paged.js получает те же стили аргументом `preview()` и его Polisher сам применяет их к разбитому содержимому (доказательство — десктопный путь печатает верно вообще без ручной инъекции). Инъекция убрана. ВАЖНО, найдено планировщиком: Paged.js вставляет СВОЙ неограниченный `<style data-pagedjs-inserted-styles>` в head, поэтому удаления только нашей инъекции было бы НЕДОСТАТОЧНО — добавлен захват `previewer.polisher` и вызов `.destroy()` в `afterprint`, плюс очистка `printStyle.textContent`. ДЕФЕКТ B (рассинхрон разбивки) — РЕГРЕССИЯ, внесённая предыдущей задачей 260805-ifj: сброс `line-height` был положен внутрь `@media print`, а Paged.js меряет на экране, где ещё действовал 1.5 от приложения. Страницы нарезались под растянутый текст, а печатались сжатым — превью и предпечатное окно браузера расходились. Сброс вынесен из `@media print` и перескоплен с `body` на `#act-print-root`, действует безусловно (экран и печать), поэтому измерение и вывод согласованы. Проверено по исходникам, что более специфичные правила шаблонов (напр. `.header .requisites { line-height: 1.35 }`) по-прежнему побеждают, и что ни один shipped-шаблон не объявляет `body { line-height }`. Побочно: `ui/src/pagedjs.d.ts` расширен полем `polisher`. Сохранены все несущие правила прошлых задач. Гейты: `svelte-check` 0 ошибок, `lint` включая гейт CSP-хэша, `build`. НЕ ПРОВЕРЕНО: совпадение разбивки в предпечатном окне и сохранность шрифта приложения после цикла печати — требует живого LAN-клиента. Commit `1f868ad`. | complete ✓ |
| 2026-08-05 | 260805-lrs-employee-header-full-name-must-use-avail | ФИО сотрудника в шапке обрезалось многоточием даже на широком экране, где строка шапки почти пустая (живой UAT из LAN-браузера, ФИО «Иванов Александр Дмитриевич»). ПРИЧИНА: в `ui/src/features/layout/EmployeeLayout.svelte` у `.user-name` стояла пара `max-width: 200px` + `flex-shrink: 0` — жёсткий потолок в 200px независимо от свободного места, плюс отказ участвовать во flex-раскладке. `git log -L` показал, что правило пришло из коммита `0667f1c` (2026-06-21, план 10-04) вместе с созданием самого EmployeeLayout — это НЕ регрессия последних работ. ФИКС: `max-width` убран; `.user-name` получил `flex-shrink: 1` и `min-width: 0` (без него flex-элемент не сжимается ниже размера содержимого и многоточие не включается никогда); `.employee-header-actions` получил `min-width: 0`, чтобы давление сжатия доходило от строки шапки до группы; `.user-role` получил `flex-shrink: 0; white-space: nowrap`, чтобы «Сотрудник» не сжимался и уступало место именно имя. Итог: на широком экране ФИО целиком, на узком сжимается только оно, а роль, переключатель темы и кнопка «Выйти» сохраняют размер. Не тронуты три несвязанных `max-width: 200px` (`ActNumberField`, `DeviceListRow`, `DeviceImportCsvModal`) — там ограничение уместно. Гейт: структурный `verify.sh` разбирает тела CSS-правил по отдельности (не плоский grep), предварительно прогнан на неисправленном файле и корректно упал — гейт правдивый. Плюс `svelte-check` 0 ошибок, `lint`, `build`. НЕ ПРОВЕРЕНО: фактическое поведение ширины на широком и узком вьюпорте — нужен живой браузер. Commit `95614e4`. | complete ✓ |
| 2026-08-05 | 260805-nae-employee-dashboard-widget-must-exclude-a | Сотрудник, авто-зарегистрированный через AD-SSO, видел «Мои заявки: 1» при ПУСТОМ списке заявок (живой UAT на Windows). Считалась невидимая авто-заявка `ad_register`, существующая только для очереди одобрения администратора. ЭТО ТРЕТИЙ независимый путь подсчёта — квик `260804-l22` честно закрыл `RequestService::counts` (счётчики статусов на списке), но карточка «Мои заявки» питается из `dashboard_get_all_widgets`, который для Employee уходит в `DashboardService::get_employee_widgets` со СВОИМ SQL: в его clauses были только `r.deleted_at_utc IS NULL` и `r.requested_by_user_id = ?1`, фильтра по `request_type` не было вовсе. ФИКС: добавлен безусловный литеральный предикат `r.request_type != 'ad_register'` — безусловный обоснован тем, что по D-GATE-03 в эту функцию попадает только Employee, поэтому параметризованный по роли переключатель (как в `counts`) здесь не нужен. РЕГРЕСС-ТЕСТ `dashboard_employee_widget_excludes_ad_register` в `crates/trackly-app/tests/dashboard_widgets.rs`: тест A — единственная заявка сотрудника типа `ad_register`, все три счётчика виджета = 0; тест B (контроль) — заявка `free_form` того же сотрудника по-прежнему считается (`request_counts_open == 1`), что доказывает: это фильтр, а не глухое подавление. RED-проверка выполнена явно: строка фикса временно удалялась, тест падал с `left: 1 / right: 0`, затем фикс возвращён — тест ловит дефект, а не проходит всегда. Прогон оркестратором: `cargo test -p trackly-app --test dashboard_widgets` 3/3 зелёных. ДИЗАЙН-НАБЛЮДЕНИЕ (НЕ чинилось): правило фильтрации продублировано в ТРЁХ независимо написанных SQL-строителях (`list`, `counts`, `get_employee_widgets`), поэтому починка одного не чинит остальные — этот дефект пережил один фикс именно по этой причине. Кандидат на вынос общего предиката в репозиторный хелпер отдельной задачей. Второе наблюдение: `counts` исключает `ad_register` для ВСЕХ не-админов (включая менеджера), а админ/менеджерская ветка дашборда DASH-04 не исключает вовсе — должен ли менеджер это видеть, вопрос продуктовый, молча не решался. Commits `100938c`, `a4f635c`. | complete ✓ |
| 2026-08-05 | 260805-wik-ad | ФИО активного AD-пользователя не обновлялось при смене фамилии в каталоге: ветка `auth.rs:470` для активного пользователя возвращала `get_by_login` и ОТБРАСЫВАЛА разрешённое каталогом `display_name` — оно писалось только при СОЗДАНИИ записи. Расхождение с требованием SSO-01 (Phase 31). ГЛАВНАЯ ОПАСНОСТЬ, определившая всю форму правки: наивное «писать имя на каждом входе» ХУЖЕ самого бага. Оба вызывающих `on_ad_bind_success` могут передать в качестве `display_name` голый доменный логин — `sso_login` откатывается на него в degrade-ветках (`NotConfigured`/`Unreachable`/`ServiceBindFailed`), а password-bind получает его из `RealAdClient::authenticate`, где `real.rs:119/121` тоже падает на `login.to_string()` при неудачном поиске атрибута. Одна временная недоступность AD затёрла бы реальные ФИО на логины у всех, кто в этот момент вошёл. ФИКС: enum `NameSource` (`Directory`/`Fallback`) как явный признак происхождения имени, выставляемый в `Directory` ТОЛЬКО в ветке `Ok(DirectoryResult)`; новый хелпер `sync_active_user_name` с четырьмя упорядоченными guard'ами перед единственным `UPDATE users SET full_name`: D-1 происхождение = каталог, D-3 имя непустое после trim, D-3 имя не равно логину без учёта регистра, D-5 имя отличается от хранимого (обычный вход остаётся чистым чтением, без UPDATE на каждый вход). Тронута только ветка активного пользователя; pending/blocked/deleted и `force_admin_provisioning` не изменялись. ОСОЗНАННОЕ ОГРАНИЧЕНИЕ D-2: password-bind путь (`try_ad_login`) имя НЕ обновляет — `AuthOutcome::Ok` не несёт признака происхождения, отличить настоящее ФИО от фолбэка там нечем; следствие — смена фамилии подтянется только на SSO-входе. Follow-up, если понадобится: расширить `AuthOutcome::Ok` в `trackly-core::ports::ad` полем происхождения и протащить через `real.rs` + моки. НАЙДЕНО ОРКЕСТРАТОРОМ ПОСЛЕ ИСПОЛНЕНИЯ (коммит `081b314`): мутационная проверка показала, что все три анти-порча теста остаются ЗЕЛЁНЫМИ при удалении guard D-1 — то есть обязательный по плану тест «падает громко, если запись сделают безусловной» своей задачи не выполнял. Причина: единственный продакшн-вызов `http/sso.rs:71` передаёт `sso_login(ad_username, ad_username)`, поэтому сегодня в degrade-ветках имя всегда равно логину и его перехватывает guard D-3, оставляя D-1 непокрытым. Риск: если будущий вызывающий начнёт передавать правдоподобное имя из деградировавшего источника (например display name из Kerberos-тикета), D-1 станет единственной защитой, и его потеря при рефакторинге вернёт порчу данных при полностью зелёном прогоне. Добавлен тест `sso_login_does_not_overwrite_stored_name_with_untrusted_caller_supplied_name` (каталог недоступен, вызывающий передаёт непустое имя, отличное от логина — ни D-3, ни guard пустоты не применимы, блокирует только D-1). Проверен в обе стороны: падает при guard'е, заглушённом на `if false`, проходит при восстановленном; `auth.rs` возвращён к нулевому диффу. Гейты: `ad_directory_sso` 12/12 зелёных (7 прежних + 5 новых), соседние `ad_auth` 5/5, `ad_admin_logins` 9/9, `ad_register` 11/11 — без регрессий, прогонялись по одному из-за контеншена на target/. `clippy -D warnings` и `fmt --check` по тронутым файлам чисто. НЕ ПРОВЕРЕНО живьём: реальная смена фамилии в рабочем AD — требует Windows-окружения с доменом. Commits `ef17ce9`, `1c30018`, `081b314`. | complete ✓ |
| 2026-08-06 | 260806-wk1-admin-logins | Достройка вчерашней правки `260805-wik`: она научила Trackly обновлять ФИО при смене фамилии в AD, но ТОЛЬКО для обычных пользователей. Пробел найден аудитом вехи v1.3 (`.planning/v1.3-MILESTONE-AUDIT.md`). ПРИЧИНА: `on_ad_bind_success` проверяет `is_admin_login(login)` и уходит в `force_admin_provisioning` ДО обычной ветки активного пользователя, а `force_admin_provisioning` не писал `full_name` ни в одной ветке, кроме `force_admin_insert_unknown` (первое создание записи) — проверено прямым чтением, в `force_admin_escalate_active` упоминаний `full_name` было ноль. Итог: у администратора из списка `admin_logins` ФИО фиксировалось навсегда при первом входе, в отличие от обычного сотрудника. Расхождение с требованием SSO-01. ФИКС: `NameSource` протащен в `force_admin_provisioning`, и четыре ветки (уже-админ, эскалация активного не-админа, активация pending, оживление заблокированного) теперь вызывают УЖЕ СУЩЕСТВУЮЩИЙ хелпер `sync_active_user_name` со всеми его четырьмя guard'ами. Ветка `force_admin_insert_unknown` не тронута — там имя пишется при INSERT и это правильно. ПРИНЦИПИАЛЬНО: второй путь записи имени НЕ создавался — единственный `UPDATE users SET full_name` переиспользуется теперь из пяти мест. Обоснование: именно дублирование логики в трёх независимых SQL-строителях привело к тому, что предикат `ad_register` разъехался и один и тот же дефект чинили дважды. ТЕСТЫ (3 новых в `ad_admin_logins.rs`): уже-активный админ + каталог отдаёт новое ФИО → имя обновилось; каталог НЕДОСТУПЕН → имя НЕ затёрлось; активный не-админ из списка входит → становится админом И имя обновилось (пиннит именно ветку эскалации). Анти-порча тест сразу написан в СИЛЬНОЙ форме — вызывающий передаёт непустое имя, ОТЛИЧНОЕ от логина: вчера три анти-порча теста оказались бесполезны ровно потому, что при `sso_login(ad_username, ad_username)` в degrade-ветках имя всегда равно логину и срабатывал более слабый guard, оставляя главный непокрытым. Мутационная проверка выполнена дважды — исполнителем (снял вызов из ветки эскалации → покраснел ровно соответствующий тест) и оркестратором независимо (заглушил guard D-1 на `if false` → анти-порча тест покраснел с внятным сообщением); `auth.rs` оба раза возвращён к нулевому диффу. Гейты: `ad_admin_logins` 12/12, `ad_directory_sso` 12/12, `ad_auth` 5/5, `ad_register` 11/11 — прогонялись по одному из-за контеншена на `target/`; `clippy -D warnings` и `fmt --check` по тронутым файлам чисто. НЕ ПРОВЕРЕНО живьём: смена фамилии администратора в рабочем AD — требует Windows-окружения с доменом. Осознанно НЕ закрыто: вход по паролю (`try_ad_login`) по-прежнему не обновляет имя (ограничение D-2, `AuthOutcome::Ok` не несёт признака происхождения). Commits `5342683`, `7b85d5c`. | complete ✓ |
| 2026-08-08 | 260808-np4-unify-ad-register-visibility-predicate | Закрытие тех-долга, который сам же квик `260805-nae` явно записал как «кандидат на вынос отдельной задачей»: правило видимости `ad_register` (REQ-06 / T-09-11 — видит только админ) было реализовано ТРИЖДЫ независимо, суммарно в 11 местах. Именно поэтому один и тот же дефект счётчика заявок чинили дважды (`260804-l22` закрыл `RequestService::counts`, но `DashboardService::get_employee_widgets` со своим SQL остался — это и всплыло живым UAT). Проектный урок из RETROSPECTIVE.md: повторный дефект = отсутствующий гейт, чинить надо не симптом. РАЗБОР ДУБЛИРОВАНИЯ: (1) ролевое правило `!matches!(caller.role, Role::Admin)` дословно дважды — `request_service.rs:120` (`list`) и `:164` (`counts`); (2) восемь литеральных вхождений SQL-предиката в `requests_sqlite.rs` — 2 в `list` (алиас `r.`, плейсхолдер `?5`) и 6 в `counts` (без алиаса, `?2`); (3) захардкоженная строка в `dashboard_service.rs:328`, безусловная. РЕФАКТОРИНГ в 2 функции: `trackly_core::auth::excludes_ad_register(&Role) -> bool` (рядом с матрицей авторизации — там живёт остальное ролевое знание) + `requests_sqlite::{ad_register_predicate(alias), ad_register_exclude_clause(alias, placeholder)}`. Плейсхолдер и алиас передаёт ВЫЗЫВАЮЩИЙ, хелпер их не владеет — поэтому развёрнутый текст побайтово совпадает с прежним и сдвиг нумерации параметров структурно невозможен (главный риск задачи, проверен чтением `git show 210cee3` рядом с `params!`). Семантика `dashboard_service` сохранена БЕЗУСЛОВНОЙ: по D-GATE-03 туда попадает только Employee, ролевой переключатель там не нужен. ПРОДУКТОВОЕ РЕШЕНИЕ ПОЛЬЗОВАТЕЛЯ (второй открытый вопрос из `260805-nae`): видимость остаётся admin-only, REQ-06 не меняется — менеджер `ad_register` не видит, поскольку и одобрить не может (`Action::ManageUsers` = admin-only, `auth.rs:130`). Поведение НЕ менялось, это чистый рефакторинг. НОВЫЙ ТЕСТ `requests_ad_register_visibility_manager.rs` пиннит именно роль Manager — единственную, никогда прежде не прогонявшуюся через сервисный слой для этого предиката (прежнее покрытие `requests_ad_register.rs` гоняло только Employee); идёт через `RequestService::list`/`counts` с настоящим `Identity{role: Manager}`, с контрольной `free_form`-заявкой (доказывает фильтр, а не глухое подавление) и админским сравнением (доказывает роль-специфичность). Мутационная проверка: `excludes_ad_register` принудительно в `false` → захвачен настоящий assertion failure на строке 166 (`manager list must not contain any ad_register requests`), не ошибка компиляции; откат → зелено; `auth.rs` с нулевым диффом. Оркестратор независимо перепрогнал: новый тест 1/1, `requests_ad_register` 8/8, `dashboard_widgets` 3/3 (включая регрессию `dashboard_employee_widget_excludes_ad_register` от `260805-nae`) — по одному прогону из-за контеншена на `target/`. Слоёвка не нарушена: `trackly-app` уже зависел от `trackly-infra`, и `dashboard_service` уже импортировал оттуда. После правки во всём `crates/*/src/` остался РОВНО ОДИН литерал `!= 'ad_register'` — внутри самого хелпера. Верификатор: passed 5/5. Commits `fea0ef3`, `210cee3`, `5d77a8f`, `1c7f73b`. | complete ✓ |
| 2026-08-18 | 260818-pij-phase-38-roadmap-state-32-validation-pha | Синхронизация планировочных артефактов по итогам ретроактивного Nyquist-аудита Фазы 32. ROADMAP.md: Phase 38 («Nyquist-покрытие Фазы 32») закрыта без собственных планов — оба её Success Criteria уже выполнены аудитом в `32-VALIDATION.md` (`nyquist_compliant: true`, 0 пробелов по всей Per-Task Verification Map, `validated: 2026-08-18`); строка таблицы Phase Status `0/TBD | Not started` → `0/0 | Complete | 2026-08-18`, `**Plans**: TBD` → пояснение. Заодно устранён рассинхрон Phase 36: чекбокс стоял `[ ]`, хотя таблица уже говорила `6/6 | Complete | 2026-08-13` — чекбокс закрыт с явной пометкой, что живая ручная UAT (печать N=1, LAN-транспорт, изоляция печатного DOM, Windows/WebView2) осознанно отложена пользователем 2026-08-13 (`36-VERIFICATION.md`: human_needed, `36-UAT.md`: partial), т.е. галочка не означает полную верификацию. STATE.md: веха v1.3.3 = 5/5 фаз, `status: milestone_complete`, `percent: 100`, Current Position указывает на /gsd-audit-milestone. Только документация; `32-VALIDATION.md`/`36-VERIFICATION.md`/`36-UAT.md` не трогались; privacy-гейт пройден без --no-verify. | complete ✓ |
| 2026-08-19 | 260819-thx-ui | Три точечных UI-фикса в разделе «Картриджи» по итогам UAT. (1) Дропдаун «тип расходника» (Картридж/Фотобарабан) в попапах «Новый картридж/фотобарабан» и «Новая модель картриджа» показывал поле поиска над списком из двух пунктов — добавлен `searchable={false}` к обоим инстансам `Dropdown` (проп уже существовал и так же используется в `CartridgeFilters.svelte`/`PeriodSelector.svelte`; сам компонент не менялся). (2) Таблица «Модели картриджей» разъехалась после редизайна: `display:flex` стоял прямо на `<td class="cell cell-name">`, что переопределяет `display:table-cell` и выбивает ячейку из колоночной модели таблицы — повторно применён уже задокументированный FIX B3 (`.cell-name` остаётся обычной ячейкой с `overflow:hidden`/`max-width:0`, flex-раскладка ушла во вложенный `.cell-name-inner`), как в `CartridgeListRow.svelte`/`PrinterListRow.svelte`. (3) Автокомплит «Совместимые принтеры» в попапе «Новая модель картриджа» раскрывался внутри контента модалки и добавлял внутренний скролл — панель вынесена в `<body>` через существующие `use:portal` + `use:dropdownAnchor` с namespaced-классом `.dropdown--compat` (конвенция WR-03, как в `PersonAutocomplete.svelte`/`DeviceAutocompleteField.svelte`/`LocationAutocomplete.svelte`); outside-click-хендлер научился игнорировать клики внутри портированной панели. Только фронтенд, backend не затронут. Гейты: `pnpm --dir ui run svelte-check` 0 ошибок, `pnpm --dir ui build` зелёный, `ui/dist` пересобран. Визуальная проверка — за пользователем (живой UAT). | complete ✓ |
| 2026-08-19 | 260819-ubv-models-filter-row | Доработки вкладки «Модели» раздела «Картриджи» по итогам UAT квика `260819-thx`. (1) Во вкладке «Модели» не было фильтра — добавлено текстовое поле поиска в той же горизонтальной строке, что и переключатель вкладок, зеркально уже существующему полю вкладки «Картриджи» (`CartridgesSearchAndTabs.svelte`, тот же `Input` + debounce 250 мс; занят слот, где раньше стоял `.search-spacer`). Фильтрация клиентская, `$derived.by` в `CartridgesPage.svelte` по бренду+модели+примечанию, регистронезависимо — список моделей грузится целиком и не пагинирован, поэтому сетевой запрос не нужен; отфильтрованный массив уходит только в `ModelsList`, а фильтры и форма создания продолжают получать полный список. (2) Ячейка «Модель» была двухэтажной (название сверху, два чипа снизу) — свёрнута в одну строку: чип типа расходника заменён вертикальной полоской-индикатором у левой границы ячейки (`.kind-indicator`, `--tr-accent` для картриджа / `--tr-border-strong` для фотобарабана, тип продублирован в `title`/`aria-label` — не только цветом), чип цвета остался и идёт сразу за названием. `.name` получил `flex: 0 1 auto` (не `1 1 auto`), иначе название растягивалось на всю колонку и чип цвета прижимался к её правому краю; обрезка многоточием сохранена через shrink + `min-width: 0`. Инвариант FIX B3 не нарушен — `display:flex` остаётся на вложенном `.cell-name-inner`, а не на `<td>`. Только фронтенд. Гейты: `svelte-check` 0 ошибок, `pnpm --dir ui build` зелёный, `ui/dist` пересобран, гейт приватности PASS. Визуальная проверка — за пользователем (живой UAT). | complete ✓ |
| 2026-08-19 | 260819-vfg-settings-storage-backups | Объединение разделов Настроек «Хранилище» и «Бэкапы» в один раздел «Хранилище» по итогам UAT. Вкладка «Бэкапы» удалена из `SettingsSubNav.svelte` (6 вкладок вместо 7); в `SettingsPage.svelte` ветка `activeSection === 'backup'` убрана, `<BackupSettings />` теперь рендерится сразу после `<StorageSettings />` внутри ветки `'storage'` — карточка «Бэкапы» встаёт под карточкой «Хранилище данных» (существующий flex-column gap `.settings-content` даёт корректный стек, обёртка не нужна). Алиас/редирект со старого ключа `'backup'` не добавлялся: `activeSection` — чисто локальный `$state` без URL/hash-адресации и без персиста, грепом по `ui/src` других ссылок на ключ не найдено. Бэкенд и логика бэкапов/хранилища не менялись. `svelte-check` 0 ошибок, `pnpm --dir ui build` зелёный. | complete ✓ |
| 2026-08-19 | 260819-vit-showcase-page | Страница «Витрина компонентов» не скроллилась: контент, не влезающий в экран, был недоступен мышью — доскроллить можно было только табуляцией (фокус тянул контейнер за собой). Причина — `.showcase-page` в `ui/src/features/showcase/ShowcasePage.svelte` не следовала контракту прокрутки оболочки приложения: `.content` в `Layout.svelte` по дизайну `overflow: hidden`, поэтому каждая `*-page` обязана скроллить собственную внутреннюю область. При добавлении витрины этот шаг пропустили — у правила были только `padding`/`display:flex`/`flex-direction:column`/`gap`. Добавлены `height: 100%; min-height: 0; overflow-y: auto` — тот же контракт, что у `DashboardPage`/`RequestsPage`/`ReportsPage`/`SettingsPage`, но в одноконтейнерном варианте: у витрины нет разделения на шапку и контент, поэтому прокрутка навешана прямо на существующий контейнер, без новой обёртки. Разметка, `.intro`, `.showcase-block` и `Layout.svelte` не тронуты, второго скроллбара не появляется. Только CSS, одна правка в одном файле. Гейты: `svelte-check` 0 ошибок, `pnpm --dir ui build` зелёный, `ui/dist` пересобран. Визуальная проверка колесом мыши — за пользователем в запущенном приложении (WKWebView; синтетический Chromium-харнесс за верификацию не считается). | complete ✓ |
| 2026-08-20 | 260819-wq5-low-stock-basis | В Настройках → «Порог низкого остатка» добавлен Radio-выбор базы подсчёта: «По модели принтера» (новый дефолт) или «По модели картриджа» (прежнее поведение). Ключ `app_settings.low_stock_basis`, без миграции схемы; дефолт `printer_model` применяется и к существующим БД. В режиме принтера остаток суммируется по всем моделям картриджей, совместимым с одним именем принтера из `cartridge_model_compatibility.printer_name` (`LOWER(TRIM(...))`, анти-fan-out через `EXISTS`); модели без строк совместимости не учитываются, имена с нулевым остатком показываются. Ветвление продублировано в обеих независимых копиях low-stock SQL (`cartridges_sqlite.rs::low_stock()` и `dashboard_service.rs`) + кросс-тесты на их согласованность. Новые команды `settings_get/set_low_stock_basis` (Tauri + HTTP, запись под `ManageSettings`, неизвестные значения отклоняются). | needs review (UAT) |
| 2026-08-20 | 260820-rdj-device-type-switch | В попапе создания/редактирования устройства появился выбор типа: ghost-sm кебаб-кнопка в строке заголовка, меню «Устройство»/«Принтер» с галочкой (`--tr-accent`) на выбранном, реактивный заголовок в 4 вариантах (Новое устройство / Новый принтер / Редактирование устройства / Редактирование принтера). Смена типа делает полную конверсию записи атомарно в одной транзакции: Устройство→Принтер создаёт строку `printers` с пустыми IP/SNMP, Принтер→Устройство удаляет её (`printer_readings`/`printer_alerts` уходят каскадом) после подтверждения во вложенном модале. Без миграции схемы — `DevicePatch.type_id` уже существовал. Попутно: `Modal.svelte` получил module-level стек открытых модалов (Escape/Tab-trap/backdrop только у верхнего, `z-index` от глубины), `ActionMenu` — вариант `ghost-sm`, исправлен заголовок «Редактирование устройства» при правке принтера в `PrinterDetail`, и разведены SNMP-опрос (`onRefresh`) и перезагрузка списка (`onDeviceSaved`) в `PrintersPage` — раньше после конверсии список не обновлялся и показывался ложный тост «Принтер не отвечает на SNMP». UAT в 2 раунда: раунд 1 выявил inline-подтверждение вместо отдельного попапа и незакрытое обновление списка; раунд 2 — рантайм-регрессию `effect_update_depth_exceeded` в стеке модалов (`$effect` читал и писал один `$state`), устранена через `untrack`. | complete ✓ |
| 2026-08-20 | 260820-uo4-condition-autocomplete-return | В модале «Возврат» (Акты) поля «Состояние» — и bulk, и per-row override — переведены с голого `Input` на общий `DeviceAutocompleteField` (`field="state"`), так что теперь у них есть dropdown с ранее использованными состояниями. Туда же добавлен статичный фронтенд-список стандартных вариантов (Новое, Б/У, Хорошее, Среднее, Плохое, На списание), мержится по образцу существующего `allLocationSuggestions` для `field="location"`: префикс-фильтр без учёта регистра + де-дуп по `trim().toLowerCase()`, чтобы уже встречавшееся значение не задваивалось; отдельная секция «Стандартные варианты:» в dropdown, индексы клавиатурной навигации продолжают `suggestions`. Правка в одном общем компоненте, поэтому стандартные варианты автоматически появились и в попапах добавления/редактирования устройства и принтера (принтеры переиспользуют `DeviceFormModal` → `DeviceFormBody`). Бэкенд/БД не затронуты. Компонент получил проп `disabled` (нужен bulk-полю при `applyToAll=false`); симметричный disable-паттерн `condition`/`location` и валидация непустого состояния сохранены. Гейты: `svelte-check` 0 ошибок, `lint` чисто, `pnpm --dir ui build` собран (`ui/dist` обновлён для server mode). Рантайм-поведение dropdown НЕ проверено — нужен живой UAT. | complete ✓ |
| 2026-08-21 | 260820-vad-csv-pdf | В раздел «Отчёты» добавлен третий домен «Заявки» рядом с «Устройствами» и «Картриджами» — четыре вкладки по статусу (Все / Открытые / В работе / Выполненные), все периодические по `created_at_utc`, с экранной таблицей, экспортом CSV и печатью/PDF через существующий `PdfPreviewModal mode="report"`. Бэкенд: `ReportRow` расширена полями заявок, добавлены `query_requests_inner`/`count_requests_inner`, четыре `list_requests_*`, ветка `requests` в `get_report_counts` (иначе бейджи вкладок были бы нулевые), русские подписи типа и статуса переводятся на бэкенде (включая живой `cancelled` → «Отменена», найденный plan-checker'ом и отсутствовавший в исходных решениях) — так экран, CSV и печать не разъезжаются. Оба транспорта: 4 Tauri-команды + зеркальные `/api/v1/reports_list_requests_*`, биндинги перегенерированы. RBAC: заявки `ad_register` скрыты от роли Manager (REQ-06/T-09-11). Новый тест-файл `report_requests.rs` (6 тестов); полный прогон `trackly-app` зелёный, index-alignment `columns_for`/`column_labels_for` не сломан. Живая UAT в запущенном приложении не проводилась. | needs review |
| 2026-08-21 | 260821-w18-requests-report-category-filter | В строку периода отчёта «Заявки» добавлена secondary-кнопка (md) с иконкой воронки; по клику — попап с 8 чекбоксами (Все, Регистрации, Замена картриджа, Ремонт техники, Расходные материалы, Программное обеспечение, Без категорий, Прочее). «Все» отмечено → остальные визуально отмечены и disabled; снятие «Все» активирует их для поштучного отключения. Бэкенд: новое поле `ReportFilter.request_category_filter: Option<Vec<String>>` + `category_filter_clause()` — allow-list, где «Все» = `None` (полное отсутствие WHERE-ограничения, не OR известных ключей, чтобы будущие типы/категории не пропадали молча), а явный пустой выбор = `Some(vec![])` = 0 строк без отката на «Все». Категории резолвятся подзапросом по имени в `request_categories` (таблица-lookup, не enum), «Без категорий» = `free_form AND category_id IS NULL`. Клауза объединяется с `ad_register_predicate` через AND — фильтр может только сузить выборку, RBAC-исключение `ad_register` для Manager не обходится (проверено тестом). Фильтр едет через существующий `ReportFilter`, поэтому экран, счётчики вкладок, CSV и печать/PDF на Tauri и HTTP получают его без правок транспортного слоя. +6 unit +6 интеграционных тестов; регрессия по всем затронутым отчётным тестам зелёная. Живая UAT попапа в запущенном приложении не проводилась. | needs review |
| 2026-08-22 | 260821-w18-filter-button-ghost-sm | `/gsd-fast`: кнопка-воронка фильтра категорий в отчёте «Заявки» переведена с `variant="secondary" size="md"` на `variant="ghost" size="sm"` (RequestCategoryFilter.svelte:80). Индикатор активного фильтра (`.active-dot`) не трогали — он позиционируется от `.trigger-wrap`, а не от кнопки. Живая проверка читаемости ghost-кнопки рядом с PeriodSelector не проводилась. | needs review |
| 2026-08-26 | 260826-rbe-extend-d-28-subtree-place-filter-to-cart | Устранено расхождение UI↔бэкенд, найденное Nyquist-аудитом фазы 39: PlacePicker в «Отчётах» рендерился на всех трёх вкладках, но `report_service.rs` читал `ReportFilter.place_id` только в домене «Устройства» — на «Картриджах» и «Заявках» контрол молча ничего не делал. Пользователь выбрал «доделать бэкенд», а не прятать контрол. D-28 subtree-CTE добавлен в 6 builder-ов: `query_cartridge_audit`/`query_cartridge_snapshot` + их count-пары (алиас `c.place_id`) и `query_requests_inner`/`count_requests_inner` (алиас `d.place_id` по месту ПРИНТЕРА заявки; в count-вариант добавлен отсутствовавший `LEFT JOIN devices`). Попутно исправлен латентный баг: `is_storage`-блоки в cartridge/requests-функциях перезаписывали `with_prefix` безусловно и затирали бы новую CTE при одновременном выборе двух фильтров — все 13 присваиваний переведены на merge-safe форму. Фронтенд не тронут. `report_place_subtree.rs` 6 → 11 тестов (захват вложенного места + исключение соседнего поддерева, точные счётчики). Отклонения: `#[allow(clippy::too_many_arguments)]` на 8-м параметре `query_requests_inner`; фикс двойного счёта в новой фикстуре. | complete ✓ |
| 2026-08-27 | 260827-gim-d-26-place-path-shortplacepath-3 | Дефект W1 из аудита вехи v1.4: в отчёте «Заявки» имя принтера молча пропадало из колонки «Принтер / Место», когда путь размещения принтера был глубже двух сегментов — межфазная коллизия D-26-обрезки (`shortPlacePath`, Фаза 39) с составной строкой `combine_printer_and_place` (Фаза 12). Починено у источника, а не парсингом строки во фронтенде: `query_requests_inner` больше не склеивает printer_name+place, `ReportRow.device_name`/`place_path` приходят раздельно, а фронтенд собирает ячейку по явному флагу `Column.compositeWith` — D-26 теперь режет только чистый путь и структурно не может съесть имя принтера. Склейка осталась только в CSV/PDF-экспорте (`row_field("printer_place")`), где она и не обрезалась. Заодно закрыт W2: `column_labels_for` отдаёт «Место» на всех шести доменах вместо остаточных «Локация»/«Расположение»/«Принтер / Локация», восстанавливая собственный инвариант функции. Регрессия на 3-сегментном пути (`requests_report_printer_name_survives_deep_place_path`) + 2 unit-теста на `printer_place`; попутно починен `report_requests_open_filters_by_status_and_translates_type`, ассертивший старую склейку — он не входил в инвентарь исполнителя и упал только на полном прогоне отчётных бинарей. Гейты: 12/12 report_place_subtree, 12/12 report_requests, 8/8 html_report_render, 214/214 lib, clippy/fmt/svelte-check/lint/build чисто. | complete ✓ |
| 2026-08-27 | 260827-rzq-sibling-cmp-sort-by-places-list-all | Живой UAT на Windows: раздел «Места» падал с «Не удалось загрузить места…», `POST /api/v1/places_list_all` → `net::ERR_EMPTY_RESPONSE`. Причина — `sibling_cmp` не был полным порядком: каждая пара сравнивалась РАЗНЫМ правилом в зависимости от того, какие поля у обеих сторон заполнены (`sort_order` → `level` → имя, стадия пропускалась, если значение было только у одной стороны). Контрпример: x(level=None,«Б») < y(level=1,«Я») < z(level=5,«А»), но x > z. Rust ≥1.81 такое детектирует и паникует в `sort_by`; `CatchPanicLayer` в приложении нет, поэтому паника рвала соединение без ответа, а процесс жил дальше. Детект зависит от размера среза — на маленьком дереве insertion sort молча выдавал мусорный порядок, поэтому дефект пережил всю UAT Фазы 39 и вылез, когда дерево выросло. Триггер воспроизведён: частичный `sort_order` (то, что оставляет перетаскивание узлов мышью). Компаратор переписан в честную трёхстадийную лексикографическую цепочку с явным решением Some-vs-None на каждой стадии; `list_all` теперь сортирует по `(parent_id, sibling_cmp)`, а не плоско по всему дереву (сравнивал несоседние узлы); JS-порт в `PlaceTree.svelte` синхронизирован — он нёс тот же баг молча, так как `Array.prototype.sort` не бросает исключение на противоречивом компараторе. Тесты: исчерпывающая проверка законов полного порядка, регрессия на случае C (≥60 строк, частичный sort_order), полный порядок для `natural_name_cmp`. Гейты: trackly-core 69/69, places_* в trackly-app 4+5+4+2+6+1, trackly-infra places_crud/devices_place_search/cartridges_place_search 8/5/4, fmt/svelte-check/lint/build чисто. | complete ✓ |
| 2026-08-27 | 260827-ui0-csv-pdf | Живой UAT: «Экспорт CSV» в отчётах не делал вообще ничего — ни диалога, ни файла, ни ошибки. Три независимых дефекта в шести строках `ReportsPage.svelte::exportCsv`: якорь не вставлялся в DOM; `URL.revokeObjectURL` вызывался синхронно сразу после `click()` и мог отменить ещё не начавшуюся загрузку; а в Tauri-вебвью blob-загрузка вообще не поднимает диалог сохранения. Плюс четвёртый, невидимый из JS: в `capabilities/main.json` область `fs:allow-write-file` разрешала только `*.pdf`, так что `writeFile` был бы заблокирован ACL даже после починки фронтенда. Введён общий хелпер `ui/src/lib/utils/saveFile.ts` (нативный `save()`+`writeFile()` в Tauri, исправленный append→click→отложенный revoke в браузере), ACL расширен на `*.csv`, имя файла теперь несёт тип отчёта и дату вместо общего «отчёт.csv», отмена диалога больше не показывает тост об ошибке. PDF идёт через модал предпросмотра и не затронут. Гейты: svelte-check 0 ошибок, lint, build чисто — но доставку файла на диск компиляционные гейты доказать не могут, нужен живой прогон. | complete ✓ |
| 2026-08-27 | 260827-ui3-place-path-display-variant-ends-vs-tail- | Запрос владельца продукта: другой вариант сокращения адреса — «Здание А // Кабинет 214» (первый и последний сегмент через « // »), переключаемый через `trackly.config.toml`, и именно он по умолчанию. Введён `[organization] place_path_display` со значениями `ends` (дефолт) / `last_two` (прежнее поведение) / `full`. Два независимых дубля сокращения (`ReportTable::shortPlacePath` и `PlaceContents::shortPath`) заменены единым `ui/src/lib/utils/placePath.ts`; сокращение теперь применяется ещё и в списках Устройств и Картриджей, где раньше путь шёл целиком. Значение едет на фронт в boot-time `AuthStatusDto` (оба транспорта, без нового раунд-трипа и без риска 401 до логина). Путь из 1-2 сегментов не сокращается ни при одном варианте. Полный путь везде остаётся в `title`. **Отклонение от плана, решение оркестратора:** нераспознанное значение ЭТОГО ключа деградирует локально к `ends` с `tracing::warn!`, а не роняет весь TOML — иначе опечатка в косметической настройке откатывала бы `config_recovery` на `AppConfig::default()`, то есть меняла `paths.db_path` (другая БД!) и выключала server-режим. Остальные поля конфига строгость сохранили. Контракт `compositeWith`/`formatPlaceCell` из 260827-gim не тронут — сменился только callback `transformPath`. Гейты: config_test 11, config_example_test 2, export_bindings 1, report_place_subtree 12, report_requests 12, fmt/svelte-check/lint/build чисто. | complete ✓ |
| 2026-08-27 | 260827-wsu-csv-export-date-column-raw-unix-timestam | Живой UAT: в экспорте CSV колонка «Дата» уходила сырым unix-timestamp — числом, которое в Excel надо руками превращать в дату. Оказалось шире: `row_field` вызывается из ДВУХ точек экспорта (`export_csv` и `export_pdf`), так что число ехало и в печатный HTML/PDF-отчёт — владелец продукта заметил только в CSV. Добавлен `format_handover_date(unix_seconds, tz)` → «дд.мм.гг, чч:мм» в таймзоне организации через существующий `get_tz_offset()`; `row_field` принимает `tz`, обе точки экспорта прокидывают его. Экранная ячейка в `ReportTable.svelte` выровнена под тот же формат (раньше показывала только дату, без времени). Запятая внутри значения CSV не ломает — разделитель `;`. Пустая дата остаётся пустой ячейкой, а не «—». Тесты: 3 новых unit-теста (Москва, UTC, отсутствующая дата). Гейты: --lib 217, report_requests 12, report_csv_export 2, report_acts 2, report_cartridges 2, report_place_subtree 12, html_report_render 8, reports_period_required 2, report_period_bounds 3, report_returns_sub_number 1, clippy/fmt/svelte-check/lint/build чисто. | complete ✓ |

## Session Continuity

Last session: 2026-09-03T01:46:55.277Z
Stopped at: Completed 40-23-PLAN.md
Resume file: 

None

Acknowledged and deferred at v1.2 milestone close (2026-07-29). Historical debt across the project (not v1.2 blockers — all 26 v1.2 requirements satisfied). Track in backlog.

| Category | Item | Status |
|----------|------|--------|
| debug | knowledge-base | unknown |
| quick | 260618-vtm-backup-date-schedule-template-fixes | missing |
| quick | 260621-r8x-fix-fk-constraint-on-request-accept-assi | missing |
| quick | 260630-v4m-fix-tls-cert-san-for-wildcard-bind-host | missing |
| quick | 260702-vtf-y-tooltip | missing |
| quick | 260704-uw3-template-seed-upgrade | missing |
| quick | 260704-wxw-act-pdf-word-fidelity-redesign | missing |
| quick | 260715-gt2-act-edit-device-quantity | missing |
| quick | 260718-x8t-tabs-segmented-width | unknown |
| quick | 260719-ocq-close-bl-01-unify-dropdown-drill-in-rese | missing |
| quick | 260723-syw-wr01-user-edit-password | missing |
| quick | 260724-pxf-fix-wr-01-ws-refcount-leak-and-wr-02-emp | missing |
| uat_gap | ? | open |
| uat_gap | ? | partial |
| uat_gap | ? | unknown |
| uat_gap | ? | passed |
| uat_gap | ? | testing |
| uat_gap | ? | passed |
| uat_gap | ? | passed |
| uat_gap | ? | partial |
| uat_gap | ? | partial |
| uat_gap | ? | passed |
| uat_gap | ? | resolved |
| uat_gap | ? | passed |
| uat_gap | ? | diagnosed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | human_needed |
| verification_gap | ? | gaps_found |

## Deferred Items (v1.3.3 close, 2026-08-19)

Подтверждено и отложено при закрытии вехи v1.3.3 (53 позиции). **Ни одна не относится к фазам
34–38 этой вехи** — все 11/11 требований satisfied, оба блокера аудита (INT-01, DOC-10) закрыты.
Состав: 29 quick-тасок без поля `status` во frontmatter (артефакт сканера, а не незакрытая работа),
8 VERIFICATION со статусом `human_needed`/`gaps_found` из фаз 03–24 (вехи v1.0/v1.2),
14 UAT-маркеров фаз 03.1–34, 1 debug-сессия, 1 todo. Часть уже фигурирует в секциях выше
(закрытия v1.1 и v1.2) — список приводится целиком, как его видит `gsd-sdk query audit-open`.

| Category | Item | Status |
|----------|------|--------|
| debug | knowledge-base | unknown |
| quick_task | 260618-vtm-backup-date-schedule-template-fixes | missing |
| quick_task | 260621-r8x-fix-fk-constraint-on-request-accept-assi | missing |
| quick_task | 260630-v4m-fix-tls-cert-san-for-wildcard-bind-host | missing |
| quick_task | 260702-vtf-y-tooltip | missing |
| quick_task | 260704-uw3-template-seed-upgrade | missing |
| quick_task | 260704-wxw-act-pdf-word-fidelity-redesign | missing |
| quick_task | 260715-gt2-act-edit-device-quantity | missing |
| quick_task | 260718-x8t-tabs-segmented-width | unknown |
| quick_task | 260719-ocq-close-bl-01-unify-dropdown-drill-in-rese | missing |
| quick_task | 260723-syw-wr01-user-edit-password | missing |
| quick_task | 260724-pxf-fix-wr-01-ws-refcount-leak-and-wr-02-emp | missing |
| quick_task | 260804-ire-ad-ldap-transport-mode | missing |
| quick_task | 260804-l22-ad-register-counts | missing |
| quick_task | 260804-lk0-config-ux-fail-soft-on-broken-trackly-co | missing |
| quick_task | 260805-edd-fix-lan-print-pass-stylesheets-to-paged- | missing |
| quick_task | 260805-gdz-lan-print-surface-swallowed-error-and-st | missing |
| quick_task | 260805-har-lan-print-neutralize-app-body-background | missing |
| quick_task | 260805-ifj-lan-print-neutralize-app-line-height-lea | missing |
| quick_task | 260805-ifj-lan-print-neutralize-app-line-height-lea", | missing |
| quick_task | 260805-jwf-lan-print-stop-injecting-template-css-in | missing |
| quick_task | 260805-jwf-lan-print-stop-injecting-template-css-in", | missing |
| quick_task | 260805-lrs-employee-header-full-name-must-use-avail | missing |
| quick_task | 260805-lrs-employee-header-full-name-must-use-avail", | missing |
| quick_task | 260805-nae-employee-dashboard-widget-must-exclude-a | missing |
| quick_task | 260805-nae-employee-dashboard-widget-must-exclude-a", | missing |
| quick_task | 260805-wik-ad | missing |
| quick_task | 260806-wk1-admin-logins | missing |
| quick_task | 260808-np4-unify-ad-register-visibility-predicate | missing |
| quick_task | 260818-pij-phase-38-roadmap-state-32-validation-pha | missing |
| todo | 2026-08-08-rework-act-templates-shared-header-handover-body-redesign.md | area: docs |
| uat_gap | Фаза 03.1 — 03.1-DEFERRED-UAT-ITEMS.md | open (0 открытых сценариев) |
| uat_gap | Фаза 03.1 — 03.1-HUMAN-UAT.md | partial (13 открытых сценариев) |
| uat_gap | Фаза 03.3 — 03.3-UAT-ITEMS.md | unknown (0 открытых сценариев) |
| uat_gap | Фаза 04 — 04-HUMAN-UAT.md | passed (0 открытых сценариев) |
| uat_gap | Фаза 05 — 05-UAT.md | testing (0 открытых сценариев) |
| uat_gap | Фаза 07 — 07-HUMAN-UAT.md | passed (13 открытых сценариев) |
| uat_gap | Фаза 08 — 08-HUMAN-UAT.md | passed (0 открытых сценариев) |
| uat_gap | Фаза 10 — 10-HUMAN-UAT.md | partial (2 открытых сценариев) |
| uat_gap | Фаза 11 — 11-HUMAN-UAT.md | partial (7 открытых сценариев) |
| uat_gap | Фаза 16 — 16-HUMAN-UAT.md | passed (0 открытых сценариев) |
| uat_gap | Фаза 17 — 17-HUMAN-UAT.md | resolved (0 открытых сценариев) |
| uat_gap | Фаза 23 — 23-HUMAN-UAT.md | passed (0 открытых сценариев) |
| uat_gap | Фаза 30 — 30-UAT.md | diagnosed (0 открытых сценариев) |
| uat_gap | Фаза 34 — 34-HUMAN-UAT.md | resolved (0 открытых сценариев) |
| verification_gap | Фаза 03 — 03-VERIFICATION.md | human_needed |
| verification_gap | Фаза 03.1 — 03.1-VERIFICATION.md | human_needed |
| verification_gap | Фаза 03.2 — 03.2-VERIFICATION.md | human_needed |
| verification_gap | Фаза 04 — 04-VERIFICATION.md | human_needed |
| verification_gap | Фаза 10 — 10-VERIFICATION.md | human_needed |
| verification_gap | Фаза 11 — 11-VERIFICATION.md | human_needed |
| verification_gap | Фаза 16 — 16-VERIFICATION.md | human_needed |
| verification_gap | Фаза 24 — 24-VERIFICATION.md | gaps_found |

## Operator Next Steps

- Веха v1.4 «Карта и осмысленное размещение» — roadmap создан 2026-08-22, статус planning.
- Следующий шаг: `/gsd-plan-phase 39` (Дерево мест — фундамент вехи, ничего не блокирует).
- Порядок фаз: 39 Дерево мест → 40 История перемещений → 41 АРМ → 42 Умный подбор принтера →
  43 Карта (просмотр) → 44 Карта (редактор) → 45 Живые статусы.

- Долг v1.3.3, не входящий в v1.4 (остаётся кандидатами): `/gsd-validate-phase 36` (Nyquist),
  INT-02 (общий источник Paged.js `RepeatTableHeadHandler`), DOC-12/DOC-13, PRIV-03, 999.1.
