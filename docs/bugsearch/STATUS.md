# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F19: clear_queue dropped unsent PUSH_PROMISE without freeing promised child.

## Last actions
1. Confirmed **F19** (F18 residual): parent `send_reset` / clear_queue discarded queued PUSH_PROMISE frames without closing the promised stream — orphaned `is_pending_push` child (no wire frames, slab leak until accidental free).
2. Fix: when dropping a PushPromise in `clear_queue`, locally CANCEL the promised stream (never on wire → no RST) and `transition_after`.
3. Regression: `parent_reset_discards_unsent_push_promise_child`.

## Next recommended step
1. Package PRs for F3–F19.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
