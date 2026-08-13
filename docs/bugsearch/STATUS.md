# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F77 @ `9f7c8de`)

## Current focus
F77: last SendStream drop reclaims unused reserved send capacity.

## Last actions
1. Confirmed **F77**: `SendStream` drop with `ResponseFuture` still held left `reserve_capacity` assigned (shared `ref_count`, no cancel).
2. Other streams could not send DATA until all handles dropped.
3. Fix: `send_ref_count` on `StreamRef`; last send handle `reclaim_reserved_capacity`.
4. Regression: `drop_send_stream_reclaims_reserved_capacity`.

## Next recommended step
1. Package PRs for F3–F77.
2. Residual #848 API ready-at-max-open.
3. `settings_decrease_wakes_poll_capacity_on_reclaim` drain loop still infinite after F29 (test hygiene).

## Blockers
None.
