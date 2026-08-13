# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F63 pending merge)

## Current focus
F63: empty Content-Length field accepted as 0.

## Last actions
1. Confirmed **F63**: RFC 9110 requires `1*DIGIT` for Content-Length; `parse_u64("")` returned Ok(0).
2. Fix: reject empty in `parse_u64` (all CL call sites).
3. Regressions: `empty_content_length_is_stream_error`, `send_request_rejects_empty_content_length`.

## Next recommended step
1. Package PRs for F3–F63.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (try_assign saturating_sub when available > window).

## Blockers
None.
