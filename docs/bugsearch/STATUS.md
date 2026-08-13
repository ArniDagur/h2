# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F68 @ `9dfb450`)

## Current focus
F68: non-zero Content-Length on 204 responses.

## Last actions
1. Confirmed **F68**: RFC 9110 §8.6 forbids CL on 204; nghttp2 rejects non-zero (allows CL:0). Pre-fix END_STREAM non-zero CL exception included 204 (for 304 only).
2. Fix: reject non-zero CL on 204 before `recv_open` (RST still sent after request EOS); EOS exception is 304-only.
3. Regressions: `no_content_nonzero_content_length_is_stream_error`, `no_content_zero_content_length_and_304_cl_accepted`.

## Next recommended step
1. Package PRs for F3–F68.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` authority residual of F66).

## Blockers
None.
