# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `08b27f7`

## Current focus
F50: outbound non-zero Content-Length with END_STREAM.

## Last actions
1. Confirmed **F50**: `send_request`/`send_response` accepted non-zero `Content-Length` with `end_stream=true` (RFC 9113 §8.1.1 malformed; peers already RST on receive).
2. Fix: reject → `UserError::MalformedHeaders`; 304 still allowed (representation length).
3. Regressions: `send_request_rejects_nonzero_content_length_with_end_stream`, `send_response_rejects_nonzero_content_length_with_end_stream`.

## Next recommended step
1. Package PRs for F3–F50.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
