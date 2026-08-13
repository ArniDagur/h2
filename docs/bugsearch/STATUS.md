# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Recv-side flow-control accounting (post-consume DATA errors + conservation).

## Last actions
1. Found **F3**: after `consume_connection_window`, `Streams::recv_data` only released connection `in_flight` on `Error::Reset`, not on other errors (e.g. GoAway from bad `recv_close`). Capacity could stick on the connection with no stream owner.
2. Fixed by releasing inside `recv_data` for every post-consume error; removed Streams Reset-only release (would double-release).
3. Added **I2** recv in-flight conservation assert (`conn.in_flight_data == Σ slab in_flight_recv_data`); must sum **slab** not `ids` (unlinked closed streams still hold user capacity).
4. Unit test `stream_window_error_releases_connection_in_flight`; integration suites green.

## Next recommended step
1. S2 sticky `poll_data` after reset (#882), or shared `send_task` wakeups.
2. Or SETTINGS window-decrease differential vs Go/nghttp2.
3. Or run full `cargo test` / fuzz for F3 edge cases.

## Blockers
None.
