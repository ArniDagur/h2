# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F95)

## Current focus
F95: explicit `send_reset` on `pending_push` waited for a send slot after PP.

## Last actions
1. F94 handled drop (`ScheduledLibraryReset`) on PP pop.
2. `send_reset` `set_reset`s and queues RST; `schedule_send` no-ops while `pending_push`.
3. PP pop then `queue_open`'d that child; abort required an empty send queue, so RST waited for a concurrency slot.
4. Fix: PP pop pushes already-reset children to `pending_send` (no slot). Server `pending_open` abort also treats `is_reset` with queued RST.
5. Regression: `send_reset_pending_push_does_not_wait_for_send_slot`.

## Next recommended step
1. Package PRs for F3–F95.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
