# Ideas backlog

## Tried
- F1–F6 fixes; #853 dismiss; I1/I2 conservation.
- S2 sticky poll_data → F4.
- Shared send_task pending_open vs capacity → F5.
- SETTINGS decrease multi-stream → F6 (`poll_capacity` hang after reclaim).
- #878/#880 already fixed upstream.

## High priority next
1. Package PRs for F3–F6.
2. **poll_capacity vs poll_reset** still share `send_task` (low practical risk; `SendStream` not `Clone`).
3. `dec_send_window` underflow TODO / `unclaimed_capacity` negative edges.

## Lower priority
- Cloned `SendRequest` bypasses backpressure (#848 design / unbounded `pending_open`).
- Upstream notes on #853 / #882 / F5 / F6.
