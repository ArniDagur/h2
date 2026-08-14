# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked server push at max=0, client idle-close refs, I1 pending_push available.

## Last actions
1. `push_request` does not reject `max_send_streams==0` (reserved ≠ concurrent). PP flush `queue_open`s; `abort_closed_pending_open` RST `REFUSED_STREAM` (F93). Waiters get `set_reset` notify. Optional API: reject like `send_request` to skip PP-then-RST.
2. Client `maybe_close` uses `has_streams_or_other_references`. Live `SendRequest` keeps `refs>1`. `pending_open` with dropped handles is F11–F17 abort, not idle GOAWAY hang.
3. `Stream::new` `inc_window` only (send `available==0`). I1 `pending_push` does not hoard capacity (F91 skip is complete).

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
