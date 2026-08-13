# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F65 @ `10069de`)

## Current focus
F65: try_assign_capacity u32 wrap when available > window.

## Last actions
1. Confirmed **F65**: plain `window - available` / `requested - available` wraps if available exceeds either bound.
2. Fix: `additional_send_capacity` with saturating_sub (returns 0 when over-assigned).
3. Unit tests: `additional_send_capacity_tests`.

## Next recommended step
1. Package PRs for F3–F65.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
