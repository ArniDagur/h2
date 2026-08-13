# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`

### F3 — Connection recv `in_flight` not released on non-Reset DATA errors
- **Fix branch:** `fix/recv-data-error-releases-conn-capacity`

### F4 — Sticky `poll_data` / `data()` errors after stream reset (#882)
- **Severity:** Medium (API footgun / busy loop)
- **Fix branch:** `fix/recv-stream-error-not-sticky`
- **Test:** `recv_stream_reset_error_is_not_sticky`.

### F5 — Shared `send_task` steals `SendRequest::ready` waker (pending_open)
- **Severity:** Medium (missed wakeup / hang)
- **Fix branch:** `fix/pending-open-send-task-waker`
- **Test:** `pending_open_ready_not_stolen_by_poll_capacity`.

### F6 — `poll_capacity` hangs after SETTINGS reclaim with capacity already assigned
- **Severity:** Medium (missed progress / hang)
- **Fix branch:** `fix/poll-capacity-after-settings-reclaim`
- **Test:** `settings_decrease_reclaims_conn_capacity_to_waiting_stream`.

### F7 — `push_promise` not woken when parent response ends receive (#811)
- **Severity:** Medium (missed wakeup / hang): client drains all `PUSH_PROMISE`s then parks on `push_task` waiting for more or end. Parent response HEADERS (EOS) / DATA EOS / trailers only notified `recv_task`, so the push stream never returned `None` until connection EOF or a timer re-poll.
- **Evidence:** Issue #811; reporter + seanmonstar agree `notify_push` missing on `recv_headers`. Regression with `wakened()` fails without fix (timeout alone re-polls and masks the bug).
- **Fix branch:** `fix/push-promise-wake-on-parent-end`
- **Change:** `notify_push()` after non-informational `recv_headers`, after end-stream `recv_data`, and after `recv_trailers`.
- **Test:** `push_promises_stream_ends_when_parent_response_finishes`.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → promoted to F4
### #878 — fixed upstream `#893`
### #880 — fixed upstream `#896`

## Suspects
None active.
