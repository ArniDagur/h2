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

### F13 — Reset `pending_open` after SETTINGS max→0 (queued open-then-RST)
- **Severity:** Low–medium (resource leak): F12 residual. If HEADERS+RST were queued while a slot existed, then peer set max concurrent to 0 before open, abort only handled empty queues / scheduled reset — stream stuck with frames still queued.
- **Evidence:** max=2 → `send_request`+`send_reset` without driving → SETTINGS max=0 → drive; with fix GOAWAY last_stream_id=0 (discarded, never opened).
- **Fix branch:** `fix/abort-reset-pending-open-when-max-zero`
- **Change:** `abort_closed_pending_open` also matches `is_reset && max_send_streams==0`.

### F14 — RecvStream drop / ignored DATA skips stream flow control
- **Severity:** Medium (correctness / interop): dropping `RecvStream` (or DATA after `is_recv=false`) only called `release_connection_capacity`. Stream window was not decreased then re-credited, so (1) peer never got stream WINDOW_UPDATE and could stall, (2) a peer could send past the real stream window without FLOW_CONTROL_ERROR.
- **Evidence:** unit test `ignored_data_when_not_recv_consumes_stream_window`; integration `drop_recv_stream_releases_stream_window_update` expects stream+conn WU after drop of ~48KB unread body with `SendStream` held.
- **Fix branch:** `fix/recv-drop-releases-stream-window`
- **Change:** `!is_recv` path does stream `send_data` + `release_capacity`; `clear_recv_buffer` uses `release_capacity` for both levels.

### F15 — Healthy `pending_open` hangs when max concurrent is 0
- **Severity:** Medium (hang / API): F11–F13 only freed cancelled/reset pending_open. A live request still queued when peer sets `MAX_CONCURRENT_STREAMS=0` never opened; `ResponseFuture` hung. New `send_request` with max already 0 also queued forever.
- **Evidence:** `pending_open_refused_when_max_drops_to_zero` (max=2 → queue → max=0 → `REFUSED_STREAM`); max=0 `send_request` is `Rejected`.
- **Fix branch:** `fix/pending-open-refused-when-max-zero`
- **Change:** reject `send_request` when `max_send_streams==0`; abort all pending_open heads when max is 0 (healthy → library `REFUSED_STREAM`).

### F16 — Cancelled buried `pending_open` only aborted at queue head
- **Severity:** Medium (resource leak): F11–F15 abort path used `pop_if` on the *head* only. With max concurrent saturated, a healthy stream at the head of `pending_open` blocked scanning; a cancelled stream behind it stayed `is_pending_open` with `ref_count==0` and was never released from the store until the head eventually opened.
- **Evidence:** max=1 hold open; queue healthy head + buried stream via clone; drop buried (+ clone `pending` ref); pre-fix `num_wired_streams()==3`; post-fix `==2`. Regression `cancel_buried_pending_open_is_aborted`.
- **Fix branch:** `fix/abort-buried-cancelled-pending-open`
- **Change:** `abort_closed_pending_open` drains and rebuilds the full queue; aborts every matching stream; re-queues survivors FIFO.

### F17 — `SendRequest::pending` ref blocked cancel of `pending_open`
- **Severity:** Medium (cancellation safety): On full occupancy, `send_request` set `pending = Some(stream.clone_to_opaque())`. That extra `OpaqueStreamRef` kept `ref_count ≥ 1` after the user dropped `ResponseFuture` and `SendStream`, so `is_canceled_interest` was false. When a concurrency slot later opened, HEADERS for the “cancelled” request were still sent.
- **Evidence:** max=1 hold open; second `send_request` into `pending_open`; drop only response+send handles; drive until hold completes. Pre-fix: stream 3 HEADERS on wire / `num_wired_streams()==2`. Post-fix: no stream-3 HEADERS; wired==1 after cancel. Regression `drop_stream_handles_cancels_despite_sendrequest_pending`.
- **Fix branch:** `fix/pending-open-cancel-without-pending-ref`
- **Change:** `SendRequest.pending` is `Option<StreamId>` (no ref); `poll_pending_open` / Rejected use `find_mut` (absent stream ⇒ ready / not rejected).

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → F4
### #878 / #880 — fixed upstream
### `dec_send_window` underflow — i32 extremes only
### #848 clone ready-at-max-open — design (queue beyond max); F9 only fixes pending_open occupancy hole
### Go #80035 SETTINGS window overflow — h2 already FLOW_CONTROL_ERROR via `inc_window` (matches Go intent)
### #882 `is_end_stream` false after reset — intentional (#810); sticky `data()` fixed by F4

## Suspects
None active.
