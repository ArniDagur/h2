# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F70 @ `fd16a62`)

## Current focus
F70: Content-Length on informational (1xx) responses.

## Last actions
1. Confirmed **F70**: RFC 9110 §8.6 forbids CL on 1xx; nghttp2 rejects; outbound already rejects; pre-fix only skipped body tracking (F34).
2. Fix: reject informational HEADERS with Content-Length before `recv_open`.
3. Regressions: `informational_with_content_length_is_stream_error`, `informational_without_content_length_then_body_ok`.

## Next recommended step
1. Package PRs for F3–F70.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]`; empty `:protocol`).

## Blockers
None.
