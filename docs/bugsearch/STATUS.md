# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F12: explicit `send_reset` on `pending_open` stuck when no concurrency slot (F11 residual).

## Last actions
1. Confirmed **F12**: with peer `MAX_CONCURRENT_STREAMS=0`, `send_reset` on a never-opened stream kept HEADERS+RST queued forever.
2. Fix: if `pending_open` and `!can_inc_num_send_streams()`, discard locally and wake conn; `abort_closed_pending_open` also frees `is_reset && pending_send.is_empty()`. When a slot exists, keep open-then-RST.
3. Regression: `send_reset_pending_open_with_max_concurrent_streams_zero` (still pass `reset_before_headers_reaches_peer_without_headers`).

## Next recommended step
1. Package PRs for F3–F12.
2. Or residual #848: connection-level ready when open count is at max (API design).
3. Or new hunt (recv window thresholds, mid-stream cancel WU, etc.).

## Blockers
None.
