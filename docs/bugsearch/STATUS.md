# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F88)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. `try_assign` does not skip `is_pending_push`. Child can hold connection capacity before PP is written.
2. Not a self-deadlock: `send_push_promise` queues PP on the parent *before* the child handle can `reserve`/`send_data`. Later parent DATA is behind PP; earlier parent DATA either already has assignment or is blocked by a third stream that can still send.
3. Residual is the same as any stream `reserve_capacity` hoarding (F76/F77), not an extra hang.

## Next recommended step
1. Package PRs for F3–F88.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2, not more header-name nits.

## Blockers
None.
