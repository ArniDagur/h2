# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F57 @ `0ff732e` + docs)

## Current focus
F57: 101 Switching Protocols not allowed in HTTP/2.

## Last actions
1. Confirmed **F57**: RFC 9113 §8.1 forbids 101 Switching Protocols; pre-fix accepted as 1xx on recv and generate.
2. Fix: stream PROTOCOL_ERROR on recv 101; `send_informational` rejects 101.
3. Regressions: `switching_protocols_101_is_stream_error`, `send_informational_rejects_101_switching_protocols`.

## Next recommended step
1. Package PRs for F3–F57.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
