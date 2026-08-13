# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F10: local SETTINGS_INITIAL_WINDOW_SIZE increase applied too late (ACK-only).

## Last actions
1. Confirmed **F10**: after `set_initial_window_size` increase, peer may send DATA under the new stream window before SETTINGS_ACK is processed. Expanding recv windows only on ACK caused FLOW_CONTROL_ERROR on that DATA.
2. Fix: expand when writing SETTINGS (increases only); decreases stay ACK-only. Builder path seeds `Recv::init_window_sz` when advertised size > default.
3. Regression: `initial_window_increase_accepts_data_before_settings_ack`.

## Next recommended step
1. Package PRs for F3–F10.
2. Or residual #848: connection-level ready wait when open count is at max (API design).
3. Or dismiss/document poll_capacity vs poll_reset shared `send_task` (low practical risk: both need `&mut SendStream`).

## Blockers
None.
