# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F76 @ `86a3bd0`)

## Current focus
F76: reserve_capacity reclaim must wake connection for starved senders.

## Last actions
1. Confirmed **F76**: reclaim via `reserve_capacity(0)` assigned capacity onto a stream with buffered DATA (`pending_send`) without waking `actions.task`.
2. Hang: connection parked on read never flushed the starved stream.
3. Fix: thread waker through assign/try_assign; wake when scheduling `pending_send`.
4. Regression: `reserve_capacity_reclaim_wakes_connection_for_starved_send`.

## Next recommended step
1. Package PRs for F3–F76.
2. Residual #848 API ready-at-max-open.
3. Note: `settings_decrease_wakes_poll_capacity_on_reclaim` drain loop is infinite after F29 (always Ready when capacity>0) — test hygiene, not a library hang.

## Blockers
None.
