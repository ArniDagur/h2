# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `65c5e10`

## Current focus
F54: `poll_informational` hang after final response (DATA at queue head).

## Last actions
1. Confirmed **F54**: after final response headers consumed, `poll_informational` saw DATA, pushed it back, fell through to `ensure_recv_open` → `Pending` forever while body half open.
2. Fix: non-1xx queue head → `Ready(None)`; also `Ready(None)` when not `is_recv_headers`.
3. Regression: `poll_informational_after_final_response_is_none`.

## Next recommended step
1. Package PRs for F3–F54.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
