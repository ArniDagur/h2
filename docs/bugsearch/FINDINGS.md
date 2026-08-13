# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`

### F3 — Connection recv `in_flight` not released on non-Reset DATA errors
- **Fix branch:** `fix/recv-data-error-releases-conn-capacity`

### F4 — Sticky `poll_data` / `data()` errors after stream reset (#882)
- **Severity:** Medium (API footgun / busy loop): after one `Some(Err(reset))`, further polls repeated the same error instead of ending the stream.
- **Evidence:** `schedule_recv` used `ensure_recv_open()?` → `Poll::Ready(Some(Err(_)))` every time while state stayed `Closed(Error)`. Issue #882; seanmonstar notes local cancel stickiness.
- **Note:** `#922` `ErrorAfterEndStream` already makes clean EOS+reset return `None` / `is_end_stream`. Sticky issue remains for errors without prior EOS.
- **Fix branch:** `fix/recv-stream-error-not-sticky`
- **Change:** `recv_err_delivered` flag; first error delivered once, then `None`. `is_end_stream()` still false for unclean end.
- **Test:** `recv_stream_reset_error_is_not_sticky`.

### F5 — Shared `send_task` steals `SendRequest::ready` waker (pending_open)
- **Severity:** Medium (missed wakeup / hang): while stream N is `pending_open`, `SendRequest` parks `poll_ready` on `stream.send_task`. Concurrent `SendStream::poll_capacity` (or `poll_reset`) on the same stream overwrites that waker. When a concurrent slot frees, `pop_pending_open` only wakes the capacity waiter → `ready()` hangs until timeout/cancel.
- **Evidence:** Regression `pending_open_ready_not_stolen_by_poll_capacity` times out without fix; passes with separate open waker. Requires peer `max_concurrent_streams` already applied (warm-up request) so `pending` is set on `SendRequest`.
- **Fix branch:** `fix/pending-open-send-task-waker`
- **Change:** `Stream::open_task` + `wait_open`/`notify_open`; `poll_pending_open` uses open slot; open/reset/EOF paths notify both.
- **Note:** `poll_capacity` and `poll_reset` still share `send_task` (single `SendStream` owner; typical `select!` is same task).

### F6 — `poll_capacity` hangs after SETTINGS reclaim with capacity already assigned
- **Severity:** Medium (missed progress / hang): after `SETTINGS_INITIAL_WINDOW_SIZE` decreases, excess connection capacity is reclaimed and may fully satisfy another stream's reservation, but `poll_capacity` only completed when `send_capacity_inc` was set. Decrease paths clear excess without setting that flag → waiter parks forever while `capacity() > 0` and `assigned >= requested`.
- **Evidence:** Multi-stream test: stream A holds ~60k connection assignment; stream B reserves 1k; SETTINGS to 1000 reclaims from A; B ends with `available=1000` and `send_task` parked; `wait_for_capacity(B, 1000)` timed out before fix.
- **Fix branch:** `fix/poll-capacity-after-settings-reclaim`
- **Change:** If `capacity > 0` and `assigned >= requested`, return `Ready(Some(Ok(capacity)))` even without `send_capacity_inc`. Still `Pending` when more capacity was requested. Never `Ready(Ok(0))` (#898).
- **Test:** `settings_decrease_reclaims_conn_capacity_to_waiting_stream`.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → promoted to F4
### #878 — `try_assign_capacity` debug_assert on cancelled stream — fixed upstream `#893` / present on experimental
### #880 — implicit RST blocked behind buffered DATA — fixed upstream `#896`

## Suspects
None active.
