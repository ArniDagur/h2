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
| `fix/recv-drop-releases-stream-window` | F14 |
| `fix/pending-open-refused-when-max-zero` | F15 |
| `fix/abort-buried-cancelled-pending-open` | F16 |
| `fix/pending-open-cancel-without-pending-ref` | F17 |
| `fix/pending-push-cancel-sends-reset` | F18 |
| `fix/clear-queue-discards-unsent-push-children` | F19 |
| `fix/push-promise-parent-state-check` | F20 |
| `fix/reject-authority-without-scheme` | F21 |
| `fix/host-header-vs-authority` | F22 |
| `fix/data-after-eos-stream-closed` | F23 |
| `fix/headers-after-eos-stream-closed` | F24 |
| `fix/push-convert-before-reserve` | F25 |
| `fix/cap-reserved-push-streams` | F26 |
| `fix/push-validate-before-reserve` | F27 |
| `fix/client-validate-before-open` | F28 |
| `fix/poll-capacity-usable-when-partially-assigned` | F29 |
| `fix/no-error-reset-zero-window` | F30 |
| `fix/poll-reset-after-end-stream` | F31 |
| `fix/reject-pseudo-in-trailers` | F32 |
| `fix/reject-informational-end-stream` | F33 |
| `fix/ignore-content-length-on-1xx` | F34 |
| `fix/cap-recv-informational` | F35 |
| `fix/reject-response-missing-status` | F36 |
| `fix/reject-request-missing-path-authority` | F37 |
| `fix/reject-response-request-pseudos` | F38 |
| `fix/reject-mismatched-content-length` | F39 |
| `fix/reject-content-length-in-trailers` | F40 |
| `fix/goaway-requires-stream-zero` | F41 |
| `fix/reject-host-authority-mismatch` | F42 |
| `fix/reject-no-content-without-end-stream` | F43 |
| `fix/reject-authority-userinfo` | F44 |
| `fix/reject-outbound-authority-userinfo` | F45 |
| `fix/reject-send-response-informational` | F46 |
| `fix/reject-send-no-content-without-end-stream` | F47 |
| `fix/reject-informational-after-final` | F48 |
| `fix/reject-outbound-content-length-no-content` | F49 |
| `fix/reject-outbound-cl-with-end-stream` | F50 |
| `fix/connect-ignore-content-length` | F51 |
| `fix/reject-connect-response-content-length` | F52 |
| `fix/reject-outbound-mismatched-content-length` | F53 |
| `fix/poll-informational-after-final-none` | F54 |
| `fix/reject-server-enable-push-one` | F55 |
| `fix/reserve-capacity-clamp-max-window` | F56 |
| `fix/reject-101-switching-protocols` | F57 |
| `fix/requested-capacity-floor-after-send` | F58 |
| `fix/reject-empty-scheme` | F59 |
| `fix/reject-asterisk-path-non-options` | F60 |
| `fix/reject-invalid-scheme-token` | F61 |
| `fix/require-authority-or-host` | F62 |

## experimental contains
F1–F62, I1–I2, deadlock stress test, docs.
