# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F31: `poll_reset` hung after clean EndStream (no RST).

## Last actions
1. Dismissed **S3** (`InFlightData::Drop` FC leak): false positive — Drop means codec still owns/writes the frame; remaining body discard on cancel is intentional.
2. Confirmed **F31**: `ensure_reason` returned `Ok(None)` for `Closed(EndStream)`, so `poll_reset` Pending forever after a normal exchange.
3. Fix: `Closed(EndStream)` → `Err(InactiveStreamId)`. Regression `poll_reset_after_clean_eos_must_not_hang`.

## Next recommended step
1. Package PRs for F3–F31.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup hunt.

## Blockers
None.
