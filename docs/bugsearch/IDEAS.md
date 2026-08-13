# Ideas backlog

## Tried
- F1–F5 fixes; #853 dismiss; I1/I2 conservation.
- S2 sticky poll_data → F4.
- Shared send_task pending_open vs capacity → F5.
- SETTINGS decrease multi-stream survey (no new bug); #878/#880 already fixed upstream.

## High priority next
1. **poll_capacity vs poll_reset** still share `send_task` (lower practical risk; document or split if multi-task use appears).
2. **SETTINGS_INITIAL_WINDOW_SIZE decrease** multi-stream differential vs Go/nghttp2 / `dec_send_window` underflow TODO.
3. Full test suite / PR packaging for F3–F5.

## Lower priority
- `unclaimed_capacity` negative window edges.
- Upstream notes on #853 / #882 / F5.
