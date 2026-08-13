# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`

### F3 — Connection recv `in_flight` not released on non-Reset DATA errors
- **Fix branch:** `fix/recv-data-error-releases-conn-capacity`

### F4 — Sticky `poll_data` / `data()` errors after stream reset (#882)
- **Fix branch:** `fix/recv-stream-error-not-sticky`

### F5 — Shared `send_task` steals `SendRequest::ready` waker (pending_open)
- **Fix branch:** `fix/pending-open-send-task-waker`

### F6 — `poll_capacity` hangs after SETTINGS reclaim with capacity already assigned
- **Fix branch:** `fix/poll-capacity-after-settings-reclaim`

### F7 — `push_promise` not woken when parent response ends receive (#811)
- **Fix branch:** `fix/push-promise-wake-on-parent-end`

### F8 — SETTINGS decrease reclaim does not wake `poll_capacity` waiters
- **Severity:** Medium (missed wakeup): when `SETTINGS_INITIAL_WINDOW_SIZE` shrinks, excess connection capacity is reclaimed from streams (`reclaimed > 0`) but producers parked on `poll_capacity` were not notified (code TODO). Waiters stayed `Pending` until some later capacity *increase*.
- **Relation to F6:** F6 made `poll_capacity` return `Ready` when already fully assigned without a flag; F8 wakes waiters that were pending for *more* capacity so they observe the post-reclaim assignment (via `notify_capacity` / `send_capacity_inc`).
- **Evidence:** `settings_decrease_wakes_poll_capacity_on_reclaim` times out without notify; passes with `notify_capacity` after reclaim.
- **Fix branch:** `fix/settings-decrease-wake-capacity`
- **Change:** after reclaim in `apply_remote_settings` decrease path, `stream.notify_capacity()` if `reclaimed > 0`. Also hardened `unclaimed_capacity` for negative windows.
- **Note:** `dec_send_window` i32 underflow → `FLOW_CONTROL_ERROR` / GOAWAY is intentional for extreme values; not treated as a separate bug.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → F4
### #878 — fixed upstream `#893`
### #880 — fixed upstream `#896`
### `dec_send_window` underflow — only at i32 extremes; library GOAWAY is acceptable

## Suspects
None active.
