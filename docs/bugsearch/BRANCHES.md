# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream |
| `experimental/bugsearch` | All fixes + docs + instrumentation |
| `fix/scheduled-reset-error-kind` | F1 |
| `fix/pending-capacity-requeue-on-zero` | F2 |
| `fix/recv-data-error-releases-conn-capacity` | F3 |
| `fix/recv-stream-error-not-sticky` | F4 |
| `fix/pending-open-send-task-waker` | F5 |
| `fix/poll-capacity-after-settings-reclaim` | F6 |
| `fix/push-promise-wake-on-parent-end` | F7 |
| `fix/settings-decrease-wake-capacity` | F8 |
| `fix/pending-open-occupancy-backpressure` | F9 |
| `fix/local-settings-window-increase-before-ack` | F10 |
| `fix/abort-cancelled-pending-open-at-max-zero` | F11 |
| `fix/send-reset-pending-open-at-max-zero` | F12 |
| `fix/abort-reset-pending-open-when-max-zero` | F13 |

## experimental contains
F1–F13, I1–I2, deadlock stress test, docs.
