# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F97)

## Current focus
F97: parent reset after advertised PUSH_PROMISE left reserved children without RST.

## Last actions
1. Parent `send_reset` / recv RST / implicit cancel now RST still-`ReservedLocal` push children (RFC 9113 §8.4.1).
2. Unsent PP still discarded locally (F19). Children that already `send_response`'d are not auto-RST'd.
3. Tests: `parent_reset_after_push_promise_resets_reserved_child`, `parent_recv_reset_resets_reserved_push_child`.

## Next recommended step
1. Package PRs for F3–F97.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` / drop queued PP when `promised_id > send.max_stream_id`.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
