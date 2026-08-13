# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F27: push validation after reserve still burned stream ids (headers / push-disabled).

## Last actions
1. Confirmed **F27** (F25 residual): connection-specific headers and `is_push_enabled` were checked only in `send_push_promise` after `reserve_local`.
2. Fix: check push enabled before convert/reserve; connection-header check in `convert_push_message`.
3. Regression: `push_request_connection_headers_do_not_burn_stream_id`.

## Next recommended step
1. Package PRs for F3–F27.
2. Or residual #848 API ready-at-max-open.
3. Or hunt new FC/wakeup/cancel bugs.

## Blockers
None.
