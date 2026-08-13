# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F18: cancelled pending_push child never sends RST after PUSH_PROMISE.

## Last actions
1. Confirmed **F18**: dropping a server `SendPushedResponse` before `send_response` schedules CANCEL while `is_pending_push`; `schedule_send` is a no-op until PUSH_PROMISE is written, and the PP pop path only scheduled the child if `pending_send` was non-empty — so no RST after PP on the wire.
2. Fix: wake on pending_push cancel; after writing PUSH_PROMISE, push scheduled-reset children onto `pending_send` for RST.
3. Regression: `drop_pushed_stream_before_response_sends_reset`.

## Next recommended step
1. Package PRs for F3–F18.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
