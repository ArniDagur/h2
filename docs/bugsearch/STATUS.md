# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. Peer RST reclaim: `handle_error` uses `task=None`, but RST is processed on the connection poll loop so `poll_complete` flushes reassigned `pending_send` (not F76).
2. `poll_accept` after `poll_closed` Ready drops `pending_accept` (existing TODO / not a waiter hang; peer sees close).
3. Mid-connection `max_frame_size` has no public setter; builder size is applied at handshake (no F10-class race).
4. Placeholder ignore `stream_close_by_recv_reset_frame_releases_capacity` is unimplemented, not a failing regression.

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2 instrumentation, not more header-name nits.

## Blockers
None.
