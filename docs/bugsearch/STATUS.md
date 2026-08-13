# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F26: reserved PUSH_PROMISE streams uncapped (memory DoS).

## Last actions
1. Confirmed **F26**: `open(PushPromise)` checked `can_inc_num_recv_streams` but reserved streams never incremented until push HEADERS, so a peer could flood PP while open count stayed low.
2. Fix: `num_reserved_streams` occupancy; refuse PP when open+reserved ≥ max; promote reserved→open without double-count; clear on close.
3. Regression: `recv_push_promise_over_max_concurrent_is_refused` (max=1 → second PP `REFUSED_STREAM`).

## Next recommended step
1. Package PRs for F3–F26.
2. Or residual #848 API ready-at-max-open.
3. Or hunt new FC/wakeup/cancel bugs.

## Blockers
None.
