# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F100)

## Current focus
F100: oversize HEADERS+EOS after request EOS dropped RST.

## Last actions
1. Clarified S4 `recv_too_big_headers`: 40-byte cap < `:status` (42) so F36 RST'd before `recv_open` (not the oversize-after-close hole).
2. With `:status` stored then a later field oversize, `recv_open` closed the stream and `send_reset` no-op'd.
3. Oversize now rejected before `recv_open`. Regression `oversize_response_eos_after_request_eos_sends_reset`.

## Next recommended step
1. Package PRs for F3–F100.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
