# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F29: `poll_capacity` hung with usable capacity when assigned < requested.

## Last actions
1. Confirmed **F29**: after `send_capacity_inc` was consumed, `poll_capacity` waited until `assigned >= requested`. With `max_send_buffer_size` and/or a partial stream window, `capacity() > 0` while assigned still below a large reservation → hang after the first send.
2. Fix: if `capacity() > 0`, always `Ready(Some(Ok(capacity)))`; only Pending when usable capacity is 0.
3. Regression: `poll_capacity_ready_with_usable_capacity_below_requested` (window=10, max_buffer=5, reserve 20).

## Next recommended step
1. Package PRs for F3–F29.
2. Or residual #848 API ready-at-max-open.
3. Or further FC/wakeup hunt.

## Blockers
None.
