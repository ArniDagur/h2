# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked codec `poll_ready` flush vs control-frame ACK vs DATA.

## Last actions
1. `FramedWrite::poll_ready` flushes when `!has_capacity`. `poll2` `poll_ready` (pong / SETTINGS ACK) therefore drains DATA before returning Pending. `poll_complete` cannot steal that space: if `poll2` is Pending, flush is already Pending (same TCP).
2. Stopping `poll_next` while write-blocked is intentional (apply SETTINGS before further frames). A mutual stall needs the peer to also stop reading; not filing as a library hang. WU still only in `poll_complete` (known fairness / PING flood).

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
