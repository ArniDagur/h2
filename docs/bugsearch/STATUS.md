# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F67 @ `9390e87`)

## Current focus
F67: header field values with leading/trailing SP/HTAB.

## Last actions
1. Confirmed **F67**: RFC 9113 §8.2.1 requires discard or reject of field values with leading/trailing SP/HTAB; `http::HeaderValue` accepts them; nghttp2 rejects.
2. Fix: reject on HPACK load (stream PROTOCOL_ERROR) and on generate (`Send::check_headers` → UserError).
3. Regressions: `recv_header_value_leading_trailing_ws_is_stream_error`, `send_request_rejects_header_value_leading_trailing_ws`.

## Next recommended step
1. Package PRs for F3–F67.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (e.g. empty IPv6 `[]` authority residual of F66).

## Blockers
None.
