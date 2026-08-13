# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F34: Content-Length on 1xx applied to final message body.

## Last actions
1. Confirmed **F34**: `recv_headers` set `stream.content_length` for 1xx frames; final body without CL failed length checks.
2. Fix: skip CL bookkeeping when `frame.is_informational()`.
3. Regression: `informational_content_length_does_not_apply_to_final_body`.

## Next recommended step
1. Package PRs for F3–F34.
2. Residual #848 API ready-at-max-open.
3. Optional: cap number of 1xx responses (Go `max1xxResponses = 5`).
4. Further FC/wakeup hunt.

## Blockers
None.
