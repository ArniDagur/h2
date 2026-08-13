# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F24: HEADERS after recv EOS was connection GOAWAY PROTOCOL_ERROR.

## Last actions
1. Confirmed **F24** (F23 sibling): post-EOS HEADERS treated as trailers → `recv_close` on Closed/HalfClosedRemote → GOAWAY `PROTOCOL_ERROR`.
2. Fix: if `is_recv_end_stream()`, stream error `STREAM_CLOSED` before `recv_trailers` (RFC 9113 §5.1; matches Go `processHeaders`).
3. Regression: `headers_after_response_eos_is_stream_closed_not_goaway`.

## Next recommended step
1. Package PRs for F3–F24.
2. Or residual #848 / reserved-stream concurrency cap.
3. Or push convert-before-`reserve_local` id-burn residual.

## Blockers
None.
