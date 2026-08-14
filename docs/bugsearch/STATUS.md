# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked F89-class waiters, FlowControl clone vs F80, PING poll ordering.

## Last actions
1. Remote GOAWAY / `recv_eof` / `handle_error` notify send/open/recv/push. `poll_capacity` does not read `conn_error`; remaining streams (`id <= last`) wait for WU or `pop_pending_open` (`notify_send`). Reset streams `Ready(None)` via `!is_send_streaming`.
2. `FlowControl` clone after `RecvStream` drop: drop releases `in_flight` first; later `release_capacity` is `ReleaseCapacityTooBig`. No double-credit.
3. Second PING while pong unflushed: `poll_ready` before `poll_next` (same class as SETTINGS ACK). `recv_ping` assert holds.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
