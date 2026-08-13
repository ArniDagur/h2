# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F15: healthy `pending_open` hang when max concurrent is 0.

## Last actions
1. Confirmed **F15**: with peer `MAX_CONCURRENT_STREAMS=0`, healthy streams queued in `pending_open` (or left there after max drops) never opened; `ResponseFuture` / `poll_ready` hung.
2. Fix: `send_request` returns `Rejected` when max is 0; `abort_closed_pending_open` aborts *all* pending_open heads when max is 0 (healthy → `REFUSED_STREAM`).
3. Regression: `pending_open_refused_when_max_drops_to_zero`; max=0 send tests expect Rejected.

## Next recommended step
1. Package PRs for F3–F15.
2. Or residual #848 API design.
3. Or connection window recovery threshold vs Go/nghttp2 (COMPARISONS).

## Blockers
None.
