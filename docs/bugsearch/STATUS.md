# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F80)

## Current focus
F80: RecvStream drop after poll_data releases leftover in_flight capacity.

## Last actions
1. Confirmed **F80**: `poll_data` charges `in_flight`; drop without `release_capacity` left it assigned while `SendStream` kept the stream alive.
2. Connection WINDOW_UPDATE never sent; peer send window stalled.
3. Fix: `clear_recv_buffer` releases remaining `in_flight_recv_data` after draining the queue.
4. Regression: `drop_recv_stream_after_read_releases_unreleased_capacity`.

## Next recommended step
1. Package PRs for F3–F80.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
