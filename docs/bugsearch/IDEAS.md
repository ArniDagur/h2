# Ideas backlog

## Tried
- F1/F2/F3 fixes; #853 stress dismiss; I1/I2 conservation asserts.
- Recv-side: post-consume DATA error release (F3); slab vs ids for in_flight sum.

## High priority next
1. **S2 sticky poll_data** after reset (#882).
2. **SETTINGS_INITIAL_WINDOW_SIZE decrease** differential vs Go/nghttp2.
3. **Shared `send_task`** multi-waiter missed wakeup.
4. Full test suite / targeted fuzz after F3.

## Lower priority
- `unclaimed_capacity` negative window edges.
- `dec_send_window` underflow TODO.
- Upstream notes on #853 / F3.
