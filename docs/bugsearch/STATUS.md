# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Shared `send_task` multi-waiter (capacity vs pending_open) — fixed as F5.

## Last actions
1. Investigated multi-waiter `send_task` (priority idea: shared waker for capacity vs reset / ready).
2. Confirmed **F5**: `SendRequest::poll_ready` while a stream is `pending_open` parked on the same `send_task` slot as `SendStream::poll_capacity` / `poll_reset`. Capacity registration overwrote the ready waker → after a concurrent stream slot freed, `ready()` never woke (hang).
3. Fix: separate `Stream::open_task` + `wait_open`/`notify_open`; wake both on open and terminal events. Regression `pending_open_ready_not_stolen_by_poll_capacity` (fails without fix, passes with).
4. SETTINGS_INITIAL_WINDOW_SIZE multi-stream decrease surveyed vs Go; h2 reclaim of connection capacity on decrease matches design; no new FC bug confirmed this fire. `#878` already fixed by `#893`; `#880` by `#896`.

## Next recommended step
1. Remaining shared-waker edge: `poll_capacity` vs `poll_reset` still share `send_task` (harder: `SendStream` not `Clone`; select on one task is OK).
2. Or SETTINGS_INITIAL_WINDOW_SIZE decrease multi-stream differential / `dec_send_window` underflow TODO.
3. Or package PRs for F3–F5.

## Blockers
None.
