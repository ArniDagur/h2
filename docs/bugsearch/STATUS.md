# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
`pending_open` occupancy in send concurrency backpressure — fixed as F9.

## Last actions
1. Investigated #848 (cloned `SendRequest::ready` always Ready at max concurrent).
2. Full clone-at-max-open `poll_ready` wait conflicts with existing queue-beyond-max tests (design: per-handle `pending` only). Documented as residual.
3. Confirmed **F9**: `next_send_stream_will_reach_capacity` ignored `pending_open`, so floods before `poll_complete` could queue unbounded pending streams while `num_send_streams == 0`. Track `num_pending_open` in occupancy for per-handle `is_full` / `Rejected`.

## Next recommended step
1. Package PRs for F3–F9.
2. Or residual #848: connection-level ready wait when open count is at max (API design).
3. Or poll_capacity vs poll_reset shared `send_task`.

## Blockers
None.
