# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked F5 open_task split, F17 abort flag, I1 unlink vs `is_closed()`.

## Last actions
1. `poll_pending_open` still parks only on `open_task` (F5). `wait_send` is poll_capacity/poll_reset only.
2. Queue `pop` clears `is_pending_open`. F17 abort then `poll_ready` sees missing/non-pending id → Ready.
3. I1 ids-only vs unlink: `Stream::is_closed()` requires empty `pending_send` and `buffered==0`, so unlink does not happen while assigned send capacity remains (RST still queued or body still buffered). Not an I1/F106-class hole.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
