# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Send-side flow-control integrity instrumentation and hunting remaining capacity bugs.

## Last actions
1. Added **debug-only send capacity conservation** checks:
   - `sum(stream.send_flow.available_signed) + conn.available_signed == conn.window_size_signed`
   - `pending_open` streams must not hold send capacity
   - Hooks: `buffer_pending` entry/exit, connection WINDOW_UPDATE, SETTINGS initial window change
2. Ran flow_control, prioritization, stream_states, client_request, deadlock — **all pass** (no conservation violations).

## Next recommended step
1. Investigate **S2** sticky `poll_data` after reset (#882) — or shared `send_task` multi-waiter wakeups.
2. Or SETTINGS_INITIAL_WINDOW_SIZE decrease + multi-stream differential vs Go (port a regression).
3. Or extend conservation checks to **recv-side** (`in_flight_data` vs stream `in_flight_recv_data`).

## Blockers
None.
