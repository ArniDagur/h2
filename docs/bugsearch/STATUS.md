# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked F106 siblings and reserved-local idle/`has_streams`.

## Last actions
1. `poll_trailers` + `ErrorAfterEndStream` + unread DATA: `ensure_recv_open` is `Ok(false)` — same drain-DATA-then-trailers park as clean EOS (already in IDEAS).
2. DATA/RST/WU on unadvertised `pending_push` (PP still queued): peer sees idle; we STREAM_CLOSED / apply WU (F79/F92 split). Leniency, not hang.
3. `has_streams()` omits reserved-local / `pending_push`. Graceful shutdown after parent EOS may `go_away_now` while `SendPushedResponse` is still held; client push future gets EOF, not a parked waiter. Same class as pending_open `has_streams` note / S5.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
