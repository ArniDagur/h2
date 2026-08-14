# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked recv WU wake and pending_capacity evict.

## Last actions
1. `release_capacity` wakes `actions.task`. `poll_complete` sets that task on Complete; CodecFull parks on the write waker. Parked-on-read after Complete has `task=Some`. send_data `take`s the task but already woke the connection.
2. `assign_connection_capacity` evicts closed streams from `pending_capacity` without `transition_after` (delayed slab reap). `recv_eof` `clear_pending_capacity` reaps. Not a waiter hang/FC leak.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
