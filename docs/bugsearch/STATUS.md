# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F11: cancelled `pending_open` streams leak when no concurrency slot is available.

## Last actions
1. Confirmed **F11**: with peer `MAX_CONCURRENT_STREAMS=0`, a dropped request stayed in `pending_open` forever (`pop_pending_open` required `can_inc_num_send_streams()`).
2. Fix: abort `ScheduledLibraryReset` heads of `pending_open` in `buffer_pending` (never on wire → discard HEADERS, no RST); wake conn on implicit reset for pending_open; skip reset-expiration for those streams.
3. Regression: `drop_pending_open_with_max_concurrent_streams_zero`.

## Next recommended step
1. Package PRs for F3–F11.
2. Or residual #848: connection-level ready when open count is at max (API design).
3. Or explicit `send_reset` on pending_open still stuck if max stays 0 (rarer than drop).

## Blockers
None.
