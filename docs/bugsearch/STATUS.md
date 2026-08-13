# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `8b75b59`

## Current focus
F55: server SETTINGS_ENABLE_PUSH = 1 accepted by client.

## Last actions
1. Confirmed **F55**: client applied remote SETTINGS with ENABLE_PUSH=1 (RFC 9113 §6.5.2: server MUST NOT send value 1 → connection PROTOCOL_ERROR).
2. Fix: in `Send::apply_remote_settings`, if client and `is_push_enabled()==Some(true)` → GOAWAY PROTOCOL_ERROR.
3. Regression: `server_enable_push_one_is_connection_error`.

## Next recommended step
1. Package PRs for F3–F55.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (`reserve_capacity` truncation residual).

## Blockers
None.
