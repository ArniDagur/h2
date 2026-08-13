# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
SETTINGS decrease reclaim without waking `poll_capacity` — fixed as F8.

## Last actions
1. Investigated `unclaimed_capacity` negative-window edges and `dec_send_window` underflow TODO.
2. Confirmed **F8**: SETTINGS_INITIAL_WINDOW_SIZE decrease reclaims connection assignment but did not notify capacity waiters (TODO in `apply_remote_settings`); parked `poll_capacity` stayed Pending until an unrelated increase.
3. Fix: `notify_capacity()` when `reclaimed > 0`. Hardened `unclaimed_capacity` (checked sub, non-negative threshold, cap at MAX). Unit tests for unclaimed edges; `dec_send_window` underflow → FLOW_CONTROL_ERROR (documented, not a fix-worthy protocol bug for normal peers).

## Next recommended step
1. Package PRs for F3–F8.
2. Or cloned `SendRequest` / `pending_open` backpressure (#848 design).
3. Or poll_capacity vs poll_reset shared `send_task` (low practical risk).

## Blockers
None.
