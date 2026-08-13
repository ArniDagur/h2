# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. `set_initial_window_size` / `enable_connect_protocol` queue SETTINGS without an explicit wake — they take `&mut Connection`, so the next `poll`/`poll_closed` runs `poll_ready` and writes the frame (not F76).
2. Server HEADERS on a skipped id (`id < next_stream_id`) is connection PROTOCOL_ERROR (implicit idle close / reuse). Match typical Go/nghttp2, not stream STREAM_CLOSED.
3. `release_capacity` still wakes `actions.task` when unclaimed crosses the WU threshold.

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2 instrumentation, not more header-name nits.

## Blockers
None.
