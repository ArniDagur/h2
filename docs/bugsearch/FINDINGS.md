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
- **Fix branch:** `fix/settings-decrease-wake-capacity`

### F9 — `pending_open` not counted toward send concurrency backpressure
- **Severity:** Medium (resource / API): `next_send_stream_will_reach_capacity` used only `num_send_streams`, so many `send_request`s before `poll_complete` could enqueue unbounded `pending_open` while open count stayed 0 (per-handle `pending` never set → no `Rejected`).
- **Evidence:** With max=2 after SETTINGS applied, two undriven `send_request`s fill occupancy; third is `Rejected` with fix. Pre-fix third would succeed.
- **Fix branch:** `fix/pending-open-occupancy-backpressure`
- **Change:** `Counts::num_pending_open` inc/dec on queue/pop/clear; occupancy = open + pending_open for `next_send_stream_will_reach_capacity`.
- **Related #848:** Cloned handles still clear `pending` and report Ready when *open* count is at max but no per-handle pending stream — intentional queue-beyond-max design used by existing tests; not changed.

### F10 — Local INITIAL_WINDOW_SIZE increase applied only on SETTINGS_ACK
- **Severity:** Medium (correctness / interoperability): peer may use the new stream window as soon as it processes SETTINGS; DATA before our ACK was rejected with connection FLOW_CONTROL_ERROR.
- **Evidence:** Exhaust default stream window, `set_initial_window_size(2×default)`, peer sends 16KB DATA then SETTINGS_ACK. Pre-fix: GOAWAY FLOW_CONTROL_ERROR with stream remaining ≈16383. Post-fix: DATA accepted (window expanded when SETTINGS written).
- **Fix branch:** `fix/local-settings-window-increase-before-ack`
- **Change:** On send of local SETTINGS, apply INITIAL_WINDOW_SIZE *increases* immediately; ACK path remains for decreases (and is no-op if increase already applied). Builder-configured size > default seeds `Recv::init_window_sz`.
- **Not a Go mismatch:** Go applies peer SETTINGS to *send* windows on receipt; local recv expansion timing is an h2 race on the advertiser side.

### F11 — Cancelled `pending_open` leaks when no send concurrency slot
- **Severity:** Medium (resource leak / hang): implicitly cancelled streams (`drop` → `ScheduledLibraryReset`) stayed in `pending_open` until `can_inc_num_send_streams()`. With peer `MAX_CONCURRENT_STREAMS=0` that never happens.
- **Evidence:** After remote max=0, `send_request` + drop handles; connection hung / stream never freed. Post-fix: stream aborted locally (no wire frames), connection closes cleanly.
- **Fix branch:** `fix/abort-cancelled-pending-open-at-max-zero`
- **Change:** `buffer_pending` aborts head of `pending_open` when `is_scheduled_reset()` (clear queue, reclaim, free) without needing a concurrency slot; wake connection from `schedule_implicit_reset` for pending_open; skip reset-expiration for never-sent streams.

### F12 — Explicit `send_reset` on `pending_open` stuck when no concurrency slot
- **Severity:** Medium (resource leak / hang): F11 residual. `send_reset` always kept HEADERS+RST for pending_open so RST is not sent on idle; with max=0 the slot never arrives.
- **Evidence:** After remote max=0, `send_request` + `send_reset(CANCEL)`; connection hung. Post-fix: local discard, connection closes; with capacity available, open-then-RST still works (`reset_before_headers_reaches_peer_without_headers`).
- **Fix branch:** `fix/send-reset-pending-open-at-max-zero`
- **Change:** On `send_reset`, if `pending_open && !can_inc_num_send_streams()`, clear queue and wake; expand abort to `is_scheduled_reset || (is_reset && pending_send.is_empty())`.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → F4
### #878 / #880 — fixed upstream
### `dec_send_window` underflow — i32 extremes only
### #848 clone ready-at-max-open — design (queue beyond max); F9 only fixes pending_open occupancy hole

## Suspects
None active.
