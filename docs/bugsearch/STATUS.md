# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F39: mismatched multiple `Content-Length` header values accepted.

## Last actions
1. Confirmed **F39**: two `Content-Length` fields (5 and 6) → framed as Remaining(5); response delivered.
2. Fix: `get_all(CONTENT_LENGTH)` must parse to one decimal value; mismatch → stream PROTOCOL_ERROR.
3. Regression: `mismatched_content_length_headers_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F39.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (Cookie merge #699 is interop/design; PRIORITY / GOAWAY edges).

## Blockers
None.
