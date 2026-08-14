# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F96)

## Current focus
No new hang/FC this fire. Checked GOAWAY + queued PUSH_PROMISE.

## Last actions
1. `send.max_stream_id` is set on recv GOAWAY but not consulted when sending PP/HEADERS.
2. `send_request` is blocked by `conn_error` (F89). `push_request` is not.
3. RFC §6.8: receiver SHOULD NOT initiate streams > last; the GOAWAY sender **ignores** those frames (unlike F96 ENABLE_PUSH=0 → client PROTOCOL_ERROR).
4. Logged as optional hardening, not a silent hang/FC.

## Next recommended step
1. Package PRs for F3–F96.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` / drop queued PP when `promised_id > send.max_stream_id`.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
