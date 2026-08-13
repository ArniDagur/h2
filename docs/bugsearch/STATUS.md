# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F43: 204/205/304 response HEADERS without END_STREAM accepted (body allowed).

## Last actions
1. Confirmed **F43**: 204 without EOS then DATA body was delivered; stream closed EndStream.
2. Fix: reject status 204/205/304 without END_STREAM before `recv_open` (client).
3. Regression: `no_content_without_end_stream_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F43.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
