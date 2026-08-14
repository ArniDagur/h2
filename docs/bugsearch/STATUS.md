# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F108)

## Current focus
F108: `send_reset` on `pending_open` hung behind window-blocked DATA.

## Last actions
1. F107 residual: explicit RST is not scheduled-reset and was not promoted.
2. `pending_open` `send_reset` now drops DATA (keeps HEADERS); `pop_frame` drops DATA when already `is_reset`.
3. Regression `send_reset_pending_open_does_not_wait_for_data_window` passes; pre-fix 2s timeout.

## Next recommended step
1. Package PRs for F3–F108.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
