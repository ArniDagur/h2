# Ideas backlog

## Tried
- Review recent master FC/wakeup commits (#893–#898, #930–#931).
- Map open GitHub issues #853, #878, #880, #882.
- F1 ScheduledLibraryReset error kind; F2 capacity-0 pending_capacity requeue.
- Stress-repro #853 → dismissed post-#860.
- **Send-side capacity conservation debug asserts (I1)** — no violations in test suite.

## High priority next
1. **Recv-side conservation** — `recv.in_flight_data` vs Σ `stream.in_flight_recv_data` (+ padding paths).
2. **S2 sticky poll_data** after reset.
3. **Differential / ported tests** for SETTINGS_INITIAL_WINDOW_SIZE decrease with in-flight streams (Go/nghttp2).
4. **Shared `send_task`** multi-waiter missed wakeup (capacity vs reset).

## Lower priority
- `unclaimed_capacity` threshold edge cases with negative windows.
- `dec_send_window` underflow TODO.
- Upstream note on #853.
