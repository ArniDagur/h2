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

### F18 — Cancelled `pending_push` stream never RST after PUSH_PROMISE
- **Severity:** Medium (protocol / cancellation): Server `push_request` creates a child with `is_pending_push`. Dropping `SendPushedResponse` without `send_response` calls `schedule_implicit_reset`, but `schedule_send` no-ops while `is_pending_push`. After PUSH_PROMISE is written, the pop path only scheduled the child if `pending_send` was non-empty — so a pure cancel left the peer with a reserved stream and no RST.
- **Evidence:** `drop_pushed_stream_before_response_sends_reset` expects PUSH_PROMISE then `RST_STREAM(CANCEL)` on stream 2.
- **Fix branch:** `fix/pending-push-cancel-sends-reset`
- **Change:** wake connection when cancelling `pending_push`; on PUSH_PROMISE pop, queue scheduled-reset children onto `pending_send` for RST emission.

### F19 — `clear_queue` orphaned never-sent PUSH_PROMISE children
- **Severity:** Medium (resource / cancel): Parent `send_reset` or other `clear_queue` paths dropped queued `PushPromise` frames without updating the promised stream. Child stayed `is_pending_push` in the store; peer never saw PP (so no RST required) but the local stream leaked and `send_response` on the handle could never leave pending_push.
- **Evidence:** `parent_reset_discards_unsent_push_promise_child` — parent CANCEL only; no PP/RST for stream 2; connection closes cleanly.
- **Fix branch:** `fix/clear-queue-discards-unsent-push-children`
- **Change:** `clear_queue` takes `counts`; on dropped PushPromise, clear child frames, library CANCEL, `transition_after`.

### F20 — PUSH_PROMISE after parent closed
- **Severity:** Medium (protocol): RFC 9113 §6.6 requires PUSH_PROMISE only on open or half-closed (remote) peer-initiated streams. After `send_response(..., true)` with client request EOS, parent is Closed, but `push_request` still reserved a child id and queued PP.
- **Evidence:** `push_request_after_response_eos_is_user_error` expects `UserError`; pre-fix succeeded and would emit PP on a closed stream.
- **Fix branch:** `fix/push-promise-parent-state-check`
- **Change:** `is_send_push_promise_allowed` on `State`; `send_push_promise` rejects before allocating promised stream.

### F21 — Authority without scheme on non-CONNECT requests
- **Severity:** Medium (protocol): RFC 9113 §8.3.1 requires `:method`, `:scheme`, and `:path` on all non-CONNECT requests. `convert_send_message` handled relative URIs (no authority) but left a `// TODO: Error` for authority-present / scheme-absent (e.g. `Uri` of `example.com:8080` / OPTIONS host form), so HEADERS went on the wire without `:scheme`.
- **Evidence:** Pre-fix `send_request(GET, "example.com:8080")` succeeded; post-fix `UserError::MissingUriSchemeAndAuthority` for `Version::HTTP_2`. Regression `request_with_authority_without_scheme_is_user_error`. CONNECT authority-only remains valid. HTTP/1.x-version requests still default `:scheme` to `http`.
- **Fix branch:** `fix/reject-authority-without-scheme`
- **Change:** Reject missing scheme on HTTP/2-version non-CONNECT in `convert_send_message`; same scheme check on server `convert_push_message`; run convert before `Send::open()` so validation does not burn a stream id.

### F22 — Host header vs `:authority` on outbound requests (#876)
- **Severity:** Medium (protocol / interop): User-supplied `Host` was kept as a regular header while `:authority` came from the URI. Differing values violate RFC 9113 §8.3.1 (“MUST NOT generate a request with a Host header field that differs from the :authority…”). curl/Go promote Host → `:authority` and strip `host`.
- **Evidence:** `Host: example.net` + URI `https://example.com/` previously wired both; post-fix only `:authority: example.net`. Matching Host is stripped. Relative `/path` + Host works for HTTP/1.x-version. Regression `host_header_promoted_to_authority_and_stripped`.
- **Fix branch:** `fix/host-header-vs-authority`
- **Change:** `Pseudo::promote_host_header`; applied in client `convert_send_message` and server `convert_push_message`. Invalid Host → `MalformedHeaders`.

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
