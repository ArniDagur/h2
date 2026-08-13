# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `57acd18`

## Current focus
F53: outbound mismatched multi Content-Length.

## Last actions
1. Confirmed **F53**: `send_request`/`send_response` accepted mismatched multi `Content-Length` (RFC 9110 §8.6; F39 receive only).
2. Fix: `validate_outbound_content_length` rejects unparseable or differing values → `MalformedHeaders`.
3. Regressions: `send_request_rejects_mismatched_content_length`, `send_response_rejects_mismatched_content_length`.

## Next recommended step
1. Package PRs for F3–F53.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
