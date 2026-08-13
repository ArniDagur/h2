# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `3fefdf3`

## Current focus
F49: outbound Content-Length on 1xx/204 (and non-zero 205).

## Last actions
1. Confirmed **F49**: `send_response(204)` / `send_informational(100)` accepted `Content-Length` (RFC 9110 §8.6 MUST NOT); 205 with non-zero CL also accepted.
2. Fix: reject → `UserError::MalformedHeaders`; 304 still allows CL; 205 allows only CL:0.
3. Regressions: `send_response_rejects_content_length_on_no_content`, `send_informational_rejects_content_length`.

## Next recommended step
1. Package PRs for F3–F49.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
