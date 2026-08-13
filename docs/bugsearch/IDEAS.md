# Ideas backlog

## Tried
- F1–F8 fixes; #853 dismiss; I1/I2 conservation.
- unclaimed_capacity negative edges (hardened + unit tests); dec_send_window underflow dismissed for normal peers.
- #878/#880 already fixed upstream.

## High priority next
1. Package PRs for F3–F8.
2. Cloned `SendRequest` / unbounded `pending_open` backpressure (#848).
3. poll_capacity vs poll_reset shared `send_task` (low practical risk).

## Lower priority
- Upstream notes on #853 / #882 / #811 / F5–F8.
