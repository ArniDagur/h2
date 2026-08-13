# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F16: cancelled `pending_open` buried behind healthy head leaked.

## Last actions
1. Confirmed **F16**: `abort_closed_pending_open` only inspected the queue *head*. A cancelled stream behind a healthy pending_open entry stayed in the slab for as long as the head could not open (e.g. max concurrent saturated by a long-lived stream).
2. Fix: rebuild `pending_open` scanning **all** entries; abort every cancelled/reset (or all when max=0); re-queue survivors in FIFO order.
3. Regression: `cancel_buried_pending_open_is_aborted`.

## Next recommended step
1. Package PRs for F3–F16.
2. Or residual #848 API design.
3. Or connection window recovery threshold vs Go/nghttp2 (COMPARISONS).

## Blockers
None.
