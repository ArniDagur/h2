# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F33: 1xx informational HEADERS with END_STREAM accepted.

## Last actions
1. Confirmed **F33**: 1xx + END_STREAM half-closed receive then queued InformationalHeaders (malformed per RFC / Go).
2. Fix: reject before `recv_open` with stream `PROTOCOL_ERROR`.
3. Regression: `informational_response_with_end_stream_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F33.
2. Residual #848 API ready-at-max-open.
3. Optional: cap number of 1xx responses (Go `max1xxResponses = 5`).
4. Further FC/wakeup hunt.

## Blockers
None.
