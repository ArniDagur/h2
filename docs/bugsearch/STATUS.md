# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `d0984aa`

## Current focus
F47: `send_response` accepted 204/205/304 without `end_of_stream`.

## Last actions
1. Confirmed **F47**: `send_response(204, end_stream=false)` queued HEADERS without END_STREAM (illegal per RFC 9110; peers RST via F43).
2. Fix: reject 204/205/304 when `!end_of_stream` → `UserError::UnexpectedFrameType`.
3. Regression: `send_response_rejects_no_content_without_end_stream`.

## Next recommended step
1. Package PRs for F3–F47.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (e.g. outbound CL on 204, RST stream-0 framing).

## Blockers
None.
