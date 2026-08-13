# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
No new high-signal bug this fire (hang/FC/cancel/wakeup).

## Last actions
1. `SendRequest::clone` clears `pending` (no shared `open_task` waker theft).
2. `pending_send` is FIFO + `push_front` on newly opened streams, not ID-sorted. `pending_open` is allocation-order FIFO (IDs increase under the lock). One open per `buffer_pending` turn; codec CONTINUATION makes `has_capacity` false before the next open. Lower-id HEADERS still leave first.
3. Small DATA is reclaimed immediately after `buffer` (`last_data_frame`); `in_flight_data_frame` does not stick across frames.

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2 instrumentation, not more header-name nits.

## Blockers
None.
