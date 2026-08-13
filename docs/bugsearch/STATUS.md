# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F23: DATA after recv EOS was connection GOAWAY PROTOCOL_ERROR.

## Last actions
1. Confirmed **F23**: `recv_data` on non-recv-streaming streams always returned connection GOAWAY `PROTOCOL_ERROR` (TODO noted stream error). Late DATA after response EOS killed the connection.
2. Fix: `ignore_data` for connection FC + stream error `STREAM_CLOSED` (RFC 9113 §6.1; matches Go `processData`).
3. Regression: `data_after_response_eos_is_stream_closed_not_goaway` (RST then second request + ping).

## Next recommended step
1. Package PRs for F3–F23.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
