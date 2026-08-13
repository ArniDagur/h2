# Ideas backlog

## Tried
- F1–F7 fixes; #853 dismiss; I1/I2 conservation.
- S2 sticky poll_data → F4.
- Shared send_task pending_open vs capacity → F5.
- SETTINGS decrease multi-stream → F6.
- Push promise end wakeup → F7 (#811).
- #878/#880 already fixed upstream.

## High priority next
1. Package PRs for F3–F7.
2. `dec_send_window` underflow TODO / `unclaimed_capacity` negative edges.
3. **poll_capacity vs poll_reset** still share `send_task` (low practical risk).

## Lower priority
- Cloned `SendRequest` bypasses backpressure (#848 design / unbounded `pending_open`).
- Upstream notes on #853 / #882 / #811 / F5–F7.
