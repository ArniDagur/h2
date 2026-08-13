# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F20: PUSH_PROMISE allowed after parent stream closed (RFC violation).

## Last actions
1. Confirmed **F20**: `send_push_promise` did not check parent state; after `send_response(..., true)` on a client-EOS stream the parent is Closed, but `push_request` still queued PUSH_PROMISE (RFC 9113 §6.6 allows only open / half-closed remote).
2. Fix: `State::is_send_push_promise_allowed`; reject with `UnexpectedFrameType` before allocating promised id.
3. Regression: `push_request_after_response_eos_is_user_error`; adjusted F18 test to push before parent EOS.

## Next recommended step
1. Package PRs for F3–F20.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
