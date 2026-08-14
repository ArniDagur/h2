# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
F106: cancelled `pending_push` reaped before PP flush (`find_mut` unwrap).

## Last actions
1. Drop `SendPushedResponse` before PP flush + `max_concurrent_reset_streams=0` (or expired reset / GOAWAY+drop) unlinked the child; `pop_frame` unwrap-panicked.
2. Keep `is_pending_push` streams linked/unreleased until PP is popped; missing child discards PP.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
