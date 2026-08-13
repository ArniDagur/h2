# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F82)

## Current focus
F82 — local HEADER_TABLE_SIZE increase applied only on SETTINGS_ACK (connection-kill race).

## Last actions
1. Found F10-class race: decoder stayed at 4096 until SETTINGS_ACK; peer size-update to builder `header_table_size` was InvalidMaxDynamicSize → GOAWAY.
2. Apply increases when SETTINGS is written (handshake + mid-connection); decreases still on ACK.
3. Tests: `header_table_size_increase_applied_before_settings_ack`, `server_header_table_size_increase_applied_before_settings_ack`, decoder units.

## Next recommended step
1. Package PRs for F3–F82.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
