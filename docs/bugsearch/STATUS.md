# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `5cab42a`

## Current focus
F56: `reserve_capacity` silent `as WindowSize` truncation.

## Last actions
1. Confirmed **F56**: `SendStream::reserve_capacity(usize)` cast to `u32` truncated large values (e.g. 2^32+n → n); prioritize used `WindowSize::MAX` (u32::MAX) not HTTP/2 max (2^31-1).
2. Fix: clamp public API and prioritize requested capacity to `MAX_WINDOW_SIZE`.
3. Regression: `reserve_capacity_clamps_to_max_window_size`.

## Next recommended step
1. Package PRs for F3–F56.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
