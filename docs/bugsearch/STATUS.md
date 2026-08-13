# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Push promise missed wakeup when parent response ends — fixed as F7 (#811).

## Last actions
1. Investigated open issue #811: after draining PUSH_PROMISEs, `push_promise()` parks on `push_task`; parent response HEADERS/DATA EOS only called `notify_recv`, not `notify_push`.
2. Confirmed **F7** with wakened regression (plain `timeout` hides the bug by re-polling on timer fire).
3. Fix: `notify_push()` from `recv_headers`, end-stream `recv_data`, and `recv_trailers`. Test `push_promises_stream_ends_when_parent_response_finishes`.

## Next recommended step
1. Package PRs for F3–F7.
2. Or `dec_send_window` underflow / `unclaimed_capacity` negative edges.
3. Or cloned `SendRequest` backpressure (#848 design note).

## Blockers
None.
