# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F58 fix pending merge)

## Current focus
F58: `requested_send_capacity` drops to 0 while buffered DATA remains.

## Last actions
1. Confirmed **F58**: requested capacity is capped at `MAX_WINDOW_SIZE` but buffered can exceed it; write path only subtracted `len`, so requested could hit 0 with data left → no further `try_assign_capacity`.
2. Fix: floor requested to `min(buffered, MAX_WINDOW_SIZE)` after each write.
3. Unit regressions in `stream::tests`.

## Next recommended step
1. Package PRs for F3–F58.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
