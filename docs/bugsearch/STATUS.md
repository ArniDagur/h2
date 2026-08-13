# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F91)

## Current focus
F91: `pending_push` child assigned conn send capacity then `queue_open`'d at max concurrent, starving open streams.

## Last actions
1. F89+F90 were still uncommitted; included with F91.
2. Confirmed `try_assign` skip of only `pending_open` lets a promised child hoard the window.
3. If `MAX_CONCURRENT_STREAMS` is full, PP pop `queue_open`s that child *with* the assignment (I1 panic / DATA hang).
4. Fix: skip assign while `pending_push`; `try_assign` when the child is opened; reclaim before `queue_open`.
5. Regression: `pending_push_queued_open_does_not_hoard_send_capacity` (fails pre-fix via I1).

## Next recommended step
1. Package PRs for F3–F91.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
