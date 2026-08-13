# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Suspect S3: in-flight DATA cancel may leak send-side flow-control for unsent bytes (`InFlightData::Drop`).

## Last actions
1. Hunt after F30: GOAWAY vs `pending_open` occupancy — already freed via `abort_closed_pending_open` (`is_reset` + empty queue); cannot repro occupancy stuck because remote GOAWAY sets `conn_error` (no new streams on that conn).
2. Identified **S3**: when `clear_queue` marks `in_flight_data_frame = Drop` during cancel, `reclaim_frame_inner` discards remaining payload without restoring stream/connection send windows charged when the frame was popped. Permanent send capacity shrink for unsent bytes (up to max frame size). Needs repro with `mock::new_with_write_capacity` partial writes.
3. No new confirmed fix this fire.

## Next recommended step
1. Prove/fix S3 (in-flight Drop capacity restore) with limited write capacity.
2. Package PRs for F3–F30.
3. Residual #848 API ready-at-max-open.

## Blockers
None.
