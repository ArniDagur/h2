# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F107)

## Current focus
F107: PUSH_PROMISE hung behind window-blocked parent DATA.

## Last actions
1. Confirmed hang: IWS=0 + `send_data` then `push_request` — PP never left (2s timeout).
2. `pop_frame` now promotes a later PP when DATA cannot be written (not flow-controlled).
3. Regression `push_promise_flushes_ahead_of_window_blocked_data` passes; pre-fix fails.

## Next recommended step
1. Package PRs for F3–F107.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
