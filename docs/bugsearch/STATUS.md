# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F71 @ `3e62e66`)

## Current focus
F71: empty / whitespace `:protocol` on extended CONNECT.

## Last actions
1. Confirmed **F71**: nghttp2 rejects empty pseudo values; empty `Protocol` was accepted as extended CONNECT; SP/HTAB padding also accepted.
2. Fix: reject empty or leading/trailing-WS `:protocol` on server recv and client send.
3. Regressions: `reject_empty_protocol_pseudo`, `send_request_rejects_empty_protocol`.

## Next recommended step
1. Package PRs for F3–F71.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` residual).

## Blockers
None.
