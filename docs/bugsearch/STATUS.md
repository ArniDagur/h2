# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F40: `Content-Length` accepted (recv) / generatable (send) in trailer HEADERS.

## Last actions
1. Confirmed **F40**: trailer block with `content-length: 5` closed stream cleanly; `send_trailers` with CL succeeded pre-fix.
2. Fix: reject CL in `recv_trailers` (PROTOCOL_ERROR) and `send_trailers` (MalformedHeaders).
3. Regressions: `recv_trailers_with_content_length_is_stream_error`; send path covered in `send_trailers_rejects_connection_specific_headers`.

## Next recommended step
1. Package PRs for F3–F40.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (PRIORITY / GOAWAY edges).

## Blockers
None.
