# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F98)

## Current focus
No new hang/FC. Rechecked remaining `pending_open` idle checks after F98.

## Last actions
1. `recv_headers` still GOAWAYs every `pending_open` id. Exempting server reserved (F98-style) would still GOAWAY: `recv_open` has no ReservedLocal / HalfClosedRemote(send_response) transition. Client HEADERS on a push id is not a hang/FC path.
2. F97 skips children that already `send_response`'d even if HEADERS are still queued in `pending_open`. RFC §8.4.1 SHOULD cancel unsent promised requests — optional, not a waiter hang (client already has PP).
3. F98 RST on reserved `pending_open` uses `send_reset` local-discard when `!can_inc`; `abort_closed_pending_open` still emits RST (test passed).

## Next recommended step
1. Package PRs for F3–F98.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
