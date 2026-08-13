# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F46: `send_response` accepted 1xx status (emitted 1xx+EOS on the wire).

## Last actions
1. Confirmed **F46**: `send_response(100, end_stream=true)` queued HEADERS with END_STREAM (malformed per F33/RFC).
2. Fix: reject informational status on `StreamRef::send_response` → `UserError::UnexpectedFrameType`; use `send_informational` for 1xx.
3. Regression: `send_response_rejects_informational_status`.

## Next recommended step
1. Package PRs for F3–F46.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
