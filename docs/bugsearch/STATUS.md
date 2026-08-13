# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `6906474`

## Current focus
F52: server `send_response` 2xx CONNECT with Content-Length.

## Last actions
1. Confirmed **F52**: server could generate 2xx CONNECT response with Content-Length (RFC 9110 §9.3.6 MUST NOT); F51 covered client ignore + request reject only.
2. Fix: mark `is_connect` on traditional CONNECT request accept; `send_response` rejects CL when `is_connect && status.is_success()`.
3. Regression: `send_connect_response_rejects_content_length`.

## Next recommended step
1. Package PRs for F3–F52.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
