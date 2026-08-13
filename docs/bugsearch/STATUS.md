# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F92)

## Current focus
F92: WU/RST on a reserved push sitting in `pending_open` was treated as idle → GOAWAY.

## Last actions
1. After PP pop, a push child with no send slot is `queue_open`'d (`is_pending_open`).
2. Peer sees reserved (local), not idle; RFC §5.1 allows RST, WINDOW_UPDATE, PRIORITY.
3. `recv_reset` / `recv_window_update` used the client-request idle check on every `pending_open`.
4. Fix: idle GOAWAY only when the peer is a server (client unsent request). Server pending_open is an advertised push.
5. Regressions: `window_update_on_pending_open_push_is_not_goaway`, `reset_on_pending_open_push_is_not_goaway`.

## Next recommended step
1. Package PRs for F3–F92.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
