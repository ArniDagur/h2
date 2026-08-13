# Ideas backlog

## Tried
- F1–F13 fixes; #853 dismiss; I1/I2 conservation.
- #848 full clone-at-max-open ready wait — conflicts with queue-beyond-max tests; F9 only.
- unclaimed_capacity negative edges; dec_send_window underflow dismissed.
- poll_capacity vs poll_reset shared `send_task`: low practical risk (both need `&mut SendStream`).
- #878/#880 fixed upstream (#893/#896).
- SETTINGS max→0 after open-then-RST queued → F13.

## High priority next
1. Package PRs for F3–F13.
2. Optional #848 follow-up: connection-level ready when *open* count is at max (API design change).

## Lower priority
- Upstream notes on findings.
- Document shared send_task residual if dual waiters via Mutex become a real report.
- Recv: stream WINDOW_UPDATE skipped when `!is_recv` mid-stream (peer stream window stall until RST).
- Connection window recovery threshold vs Go/nghttp2.
