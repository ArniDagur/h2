# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F25: invalid `push_request` burned promised stream ids.

## Last actions
1. Confirmed **F25** (F21 residual): `send_push_promise` called `reserve_local` before `convert_push_message`, so validation errors advanced `next_stream_id` and later valid pushes skipped ids (2→4).
2. Fix: peek id → convert → `reserve_local` (same order as client `send_request`).
3. Regression: `push_request_validation_error_does_not_burn_stream_id` (POST + scheme-less then GET → PP promised id 2).

## Next recommended step
1. Package PRs for F3–F25.
2. Or residual #848 API ready-at-max-open.
3. Or reserved-stream concurrency cap TODO (recv PP unbounded while open count low).

## Blockers
None.
