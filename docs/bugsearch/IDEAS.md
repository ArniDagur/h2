# Ideas backlog

## Tried
- F1–F4 fixes; #853 dismiss; I1/I2 conservation.
- S2 sticky poll_data → F4.

## High priority next
1. **SETTINGS_INITIAL_WINDOW_SIZE decrease** differential vs Go/nghttp2.
2. **Shared `send_task`** multi-waiter missed wakeup (capacity vs reset).
3. Full test suite / PR packaging for F3+F4.

## Lower priority
- `unclaimed_capacity` negative window edges.
- `dec_send_window` underflow TODO.
- Upstream notes on #853 / #882.
