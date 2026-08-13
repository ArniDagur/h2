# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. Recv WINDOW_UPDATE increment 0 already connection error at frame load (RFC MAY stream-error; valid).
2. `poll_trailers` with DATA still queued is intentional Pending (existing test); DATA+EOS-only then trailers-only is an API footgun, not a missed wakeup.
3. Refused-stream `assert!(refused.is_none())` is safe under poll_ready-before-next-frame; late DATA uses forgotten-stream STREAM_CLOSED.
4. `try_assign` skips `pending_open` (not `pending_push`); PP is not FC-gated so open streams are not starved in practice.
5. SETTINGS MAX_FRAME_SIZE range already rejected (no empty-DATA spin).

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2 instrumentation, not more header-name nits.

## Blockers
None.
