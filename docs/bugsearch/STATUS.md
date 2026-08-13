# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `2bd46c8`

## Current focus
F48: `send_informational` after final `send_response` still queued 1xx.

## Last actions
1. Confirmed **F48**: docs promise error after final response, but `send_interim_informational_headers` ignored send-half state and queued 1xx (even on Closed after EOS).
2. Fix: `State::is_send_informational_allowed` (local AwaitingHeaders / ReservedLocal); reject → `UserError::UnexpectedFrameType`.
3. Regression: `send_informational_after_final_response_is_user_error`.

## Next recommended step
1. Package PRs for F3–F48.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
