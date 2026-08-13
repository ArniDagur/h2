# Ideas backlog

## Tried
- F1–F9 fixes; #853 dismiss; I1/I2 conservation.
- #848 full clone-at-max-open ready wait — conflicts with queue-beyond-max tests; F9 only.
- unclaimed_capacity negative edges; dec_send_window underflow dismissed.

## High priority next
1. Package PRs for F3–F9.
2. Optional #848 follow-up: connection-level ready when *open* count is at max (API design change).
3. poll_capacity vs poll_reset shared `send_task` (low practical risk).

## Lower priority
- Upstream notes on findings.
