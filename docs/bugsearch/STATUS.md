# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F81)

## Current focus
No new library bug this fire.

## Last actions
1. Checked CodecFull `poll_complete` + user `send_data` while `actions.task` is unset: parked on codec write waker; next poll drains the new frames.
2. Graceful shutdown waits forever for shutdown-PING ACK (no timeout). Go uses ~1s `goAwayTimeout`. h2 has `abrupt_shutdown`; treat as policy, not a correctness hang.
3. GOAWAY + `pending_open`: `handle_error` → `is_reset` + empty queue → `abort_closed_pending_open`.
4. F81 + `send_request(..., true)` still pending_open: send half closed, no RST; HEADERS+EOS wait for a slot. Correct.

## Next recommended step
1. Package PRs for F3–F81.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
