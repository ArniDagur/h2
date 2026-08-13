# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
SETTINGS decrease + `poll_capacity` hang after reclaim — fixed as F6.

## Last actions
1. Multi-stream SETTINGS_INITIAL_WINDOW_SIZE decrease: reclaim of connection capacity from stream A to waiting stream B works (windows/available updated correctly).
2. Confirmed **F6**: after SETTINGS shrinks assignment, `poll_capacity` required `send_capacity_inc` and hung forever even when `assigned >= requested` and `capacity() > 0` (stream B never woke to send reclaimed capacity).
3. Fix: if usable capacity > 0 and reservation fully assigned, return `Ready` without a fresh increase flag; still `Pending` when more was requested. Regression `settings_decrease_reclaims_conn_capacity_to_waiting_stream`. Full `flow_control` suite green (50 pass / 4 ignored).

## Next recommended step
1. Package PRs for F3–F6.
2. Or remaining shared-waker edge `poll_capacity` vs `poll_reset` (low practical risk).
3. Or `dec_send_window` underflow TODO / `unclaimed_capacity` negative edges.

## Blockers
None.
