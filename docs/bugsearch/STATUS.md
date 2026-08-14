# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F98)

## Current focus
No new hang/FC. Rechecked I1 pending_push available and oversize-after-EOS RST.

## Last actions
1. `try_assign` skips `pending_push`; `Stream::new` send available is 0; SETTINGS inc only grows window. pending_push cannot hoard conn send capacity (I1 only asserts pending_open).
2. Oversize HEADERS+EOS after request EOS: `is_over_size` runs after `recv_open`, stream is `Closed(EndStream)`, `send_reset` no-ops. User still sees PROTOCOL_ERROR on `poll_response`. F74-class missing RST, not a waiter hang.
3. Server handshake `initial_max_send_streams=0` is replaced by the first client SETTINGS in `poll` before `accept` can `push_request`.

## Next recommended step
1. Package PRs for F3–F98.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
