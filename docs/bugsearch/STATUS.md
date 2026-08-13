# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. Checked `has_streams()` vs `pending_open` idle-close / `maybe_close`.
2. Rechecked F30 residual (SETTINGS window 0 mid-NO_ERROR flush) — still peer-WU-by-design.
3. send_trailers / send_data EOS reclaim; SETTINGS decrease skip of send-closed empty streams is safe (available already 0).

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or instrumentation (I1/I2), not more header-name nits.

## Blockers
None.
