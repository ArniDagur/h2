# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F30: server early-response NO_ERROR hang when peer stream window is already 0.

## Last actions
1. Confirmed **F30**: `#896` keeps buffered DATA for scheduled NO_ERROR so the response can complete. Peer `INITIAL_WINDOW_SIZE=0` → DATA never flushes → NO_ERROR RST deferred forever after server drops handles on an early response.
2. Fix: `maybe_cancel` uses CANCEL when unsent body remains and stream window is already closed; keep NO_ERROR when fully flushed or window can still progress (mid-response WU).
3. Regression: `early_response_zero_window_uses_cancel_not_hang`; existing NO_ERROR body-then-WU tests still pass.

## Next recommended step
1. Package PRs for F3–F30.
2. Or residual #848 API ready-at-max-open.
3. Or further FC/wakeup hunt.

## Blockers
None.
