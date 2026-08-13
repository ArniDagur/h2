# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F13: reset `pending_open` streams stuck after SETTINGS max→0 (F12 residual).

## Last actions
1. Confirmed **F13**: `send_reset` can queue HEADERS+RST while `can_inc` is true; if peer then sets `MAX_CONCURRENT_STREAMS=0` before open, the stream never left `pending_open`.
2. Fix: `abort_closed_pending_open` also drops `is_reset` pending_open streams when `max_send_streams == 0`.
3. Regression: `send_reset_pending_open_then_max_concurrent_streams_zero`.

## Next recommended step
1. Package PRs for F3–F13.
2. Or residual #848: connection-level ready when open count is at max (API design).
3. Or new hunt (recv WU when !is_recv mid-stream, connection window thresholds).

## Blockers
None.
