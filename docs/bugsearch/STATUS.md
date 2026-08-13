# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F41: GOAWAY frame with non-zero stream id accepted.

## Last actions
1. Confirmed **F41**: `GoAway::load` never inspected frame header stream id (unlike SETTINGS/PING).
2. Fix: `GoAway::load(head, payload)` rejects non-zero stream id as `InvalidStreamId` → connection PROTOCOL_ERROR.
3. Regression: `read_goaway_nonzero_stream_id_is_connection_error`.

## Next recommended step
1. Package PRs for F3–F41.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
