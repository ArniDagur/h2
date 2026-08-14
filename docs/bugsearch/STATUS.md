# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F97)

## Current focus
No new hang/FC this fire. Rechecked F30 + mid-flight SETTINGS window 0.

## Last actions
1. After implicit NO_ERROR, `pop_frame` with stream capacity 0 and `!has_unavailable` leaves the stream off `pending_send`/`pending_capacity`. A later **stream** WU still `recv_stream_window_update` → `try_assign` (does not need those queues).
2. Peer SETTINGS `INITIAL_WINDOW_SIZE=0` is the same as “no WU”: F30 only CANCEL at schedule-time window 0; mid-flight still waits. Not a silent queue-loss hang.
3. `send_reset` reclaim still wakes `actions.task` via `try_assign` when another stream has buffered DATA (F76 class).

## Next recommended step
1. Package PRs for F3–F97.
2. Residual #848 API ready-at-max-open.
3. Optional: GOAWAY-PP `max_stream_id` (send + promised recv).
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
