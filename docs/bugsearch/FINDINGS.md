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

### F23 — DATA after recv EOS → connection GOAWAY instead of stream STREAM_CLOSED
- **Severity:** Medium (protocol / resilience): `Recv::recv_data` treated any DATA while `!is_recv_streaming()` as connection `PROTOCOL_ERROR` (GOAWAY). RFC 9113 §6.1 requires stream error `STREAM_CLOSED` when DATA arrives outside open / half-closed (local) receive; §5.1 half-closed (remote) same. Go `processData` uses `streamError(STREAM_CLOSED)` and still applies connection flow control.
- **Evidence:** Response headers with EOS then an extra DATA frame: pre-fix connection dies; post-fix `RST_STREAM(STREAM_CLOSED)`, ping + second request succeed. Regression `data_after_response_eos_is_stream_closed_not_goaway`. Forgotten-stream path already used STREAM_CLOSED; this covers streams still in the store (Closed / half-closed remote / awaiting headers).
- **Fix branch:** `fix/data-after-eos-stream-closed`
- **Change:** `ignore_data(sz)` then `Error::library_reset(id, STREAM_CLOSED)`; `reset_on_recv_stream_err` emits RST.

### F24 — HEADERS after recv EOS → connection GOAWAY instead of stream STREAM_CLOSED
- **Severity:** Medium (protocol / resilience): F23 sibling. When `!is_recv_headers()`, HEADERS was always treated as trailers; if recv half already ended (`is_recv_end_stream`), `recv_trailers` → `recv_close` hit Closed/HalfClosedRemote and returned connection GOAWAY `PROTOCOL_ERROR`. RFC 9113 §5.1 half-closed (remote) requires stream error `STREAM_CLOSED` for extra HEADERS; Go `processHeaders` does `streamError(STREAM_CLOSED)`.
- **Evidence:** Response headers with EOS then another HEADERS+EOS: pre-fix connection dies; post-fix `RST_STREAM(STREAM_CLOSED)`, ping + second request succeed. Valid trailers (HEADERS+EOS while still streaming) unchanged. Regression `headers_after_response_eos_is_stream_closed_not_goaway`.
- **Fix branch:** `fix/headers-after-eos-stream-closed`
- **Change:** In `recv_headers`, if `is_recv_end_stream()`, `library_reset(STREAM_CLOSED)` before `recv_trailers`.

### F25 — Invalid `push_request` burned promised stream ids
- **Severity:** Low–medium (resource / API): `send_push_promise` reserved a local stream id and inserted a child before `convert_push_message`. Rejected pushes (unsafe method, missing `:scheme`, bad Host, …) removed the child but left `next_stream_id` advanced, so the next valid push used a higher id. HTTP/2 allows skips, but wasting ids is unnecessary and mismatched client `send_request` (F21 convert-before-open).
- **Evidence:** `push_request(POST…)` + `push_request(GET example.com:8080)` errors then `push_request(GET https://…/style.css)` emits `PUSH_PROMISE` promised id **2** (post-fix); pre-fix would use **4** after two burns. Regression `push_request_validation_error_does_not_burn_stream_id`.
- **Fix branch:** `fix/push-convert-before-reserve`
- **Change:** `ensure_next_stream_id` → convert → `reserve_local`; assert promised id matches frame.

### F26 — Uncapped reserved PUSH_PROMISE streams (memory DoS)
- **Severity:** Medium (resource / DoS): RFC 9113 §5.1.2 says reserved streams do not count toward concurrent streams. h2 checked `can_inc_num_recv_streams` on PP `open()` but only `inc_num_recv_streams` on push HEADERS, so while open count stayed below max a peer could emit unlimited PUSH_PROMISE entries in the store (TODO in streams.rs).
- **Evidence:** Client `max_concurrent_streams(1)`: first PP accepted; second PP gets `RST_STREAM(REFUSED_STREAM)`; connection remains usable (ping). Pre-fix second PP was stored. Regression `recv_push_promise_over_max_concurrent_is_refused`.
- **Fix branch:** `fix/cap-reserved-push-streams`
- **Change:** `Counts::num_reserved_streams` + `Stream::is_reserved`; `can_reserve_recv_stream` (open+reserved < max); PP `open` uses it; `recv_push_promise` incs reserved; `inc_num_recv_streams` clears reserved when promoting; `transition_after` clears reserved on close.

### F27 — Push disabled / connection headers still burned stream ids after F25
- **Severity:** Low (resource / API): F25 ordered convert before `reserve_local`, but `PeerDisabledServerPush` and connection-specific header rejection still ran inside `send_push_promise` after the id was reserved and a child inserted (then torn down).
- **Evidence:** `push_request` with `Connection: close` then a valid GET push emits promised id **2** (not 4). Regression `push_request_connection_headers_do_not_burn_stream_id`.
- **Fix branch:** `fix/push-validate-before-reserve`
- **Change:** `is_push_enabled()` check before convert/reserve; connection-header check in `convert_push_message` (mirrors `Send::check_headers`).

### F28 — Client connection-headers after `open()` burned stream ids
- **Severity:** Low (resource / API): F21 converted before `open()`, but `check_headers` still ran only in `send_headers` after open+insert. Rejected requests (`Connection`, illegal `TE`, …) removed the stream but left `next_stream_id` advanced.
- **Evidence:** `send_request` with `Connection: close` then a valid GET uses stream id **1** (not 3). Regression `connection_header_does_not_burn_stream_id`.
- **Fix branch:** `fix/client-validate-before-open`
- **Change:** `Send::check_headers(headers.fields())` after convert, before `open()`.

### F29 — `poll_capacity` Pending while usable capacity remains
- **Severity:** Medium (hang / API): After the first assignment notification (`send_capacity_inc`), `poll_capacity` required `available >= requested_send_capacity` before Ready. `capacity()` is `min(available, max_send_buffer_size) - buffered`, so a large `reserve_capacity` with a smaller stream window and/or max buffer leaves `capacity() > 0` while `available < requested` after the first send — callers that only `send_data` after `poll_capacity` hang.
- **Evidence:** Peer `INITIAL_WINDOW_SIZE=10`, client `max_send_buffer_size(5)`, `reserve_capacity(20)`: first Ready(5) and send works; second poll was Pending pre-fix despite capacity 5. Post-fix all four 5-byte slices Ready. Regression `poll_capacity_ready_with_usable_capacity_below_requested`.
- **Fix branch:** `fix/poll-capacity-usable-when-partially-assigned`
- **Change:** `poll_capacity` returns Ready whenever `capacity() > 0`; Pending only when usable capacity is 0.

### F30 — Server early-response NO_ERROR hangs when stream window is 0
- **Severity:** Medium (hang / DoS): `maybe_cancel` scheduled `RST_STREAM(NO_ERROR)` for server streams with send half closed and recv still open (early response, unread request body). `#896` deliberately does **not** discard buffered DATA for NO_ERROR so the complete response can flush first. If the peer advertised `INITIAL_WINDOW_SIZE=0`, unsent response DATA never leaves and NO_ERROR is deferred forever — connection cannot shut down.
- **Evidence:** Client SETTINGS `initial_window_size(0)`; server `send_response` + `send_data(..., eos)` + drop handles. Pre-fix: no RST within timeout; connection hung. Post-fix: `RST_STREAM(CANCEL)` promptly (window already closed → cannot complete response). Regression `early_response_zero_window_uses_cancel_not_hang`. Existing large-body NO_ERROR + later WU path unchanged (`no_error_response_body_delivered_before_rst`).
- **Fix branch:** `fix/no-error-reset-zero-window`
- **Change:** `maybe_cancel` uses NO_ERROR only when response can still flush (`buffered==0` or stream `window_size>0` or `available>0`); otherwise CANCEL so pop_frame discards body and emits RST.

### F31 — `poll_reset` hangs after clean EndStream
- **Severity:** Medium (hang / API): `State::ensure_reason` treated `Closed(EndStream)` like an open stream (`Ok(None)`), so `SendStream::poll_reset` registered a waker and returned `Pending` forever when the exchange finished without `RST_STREAM`.
- **Evidence:** Request with EOS + response EOS; `poll_reset` timed out pre-fix. Post-fix: ready `Err` (`inactive stream`). RST path still works (`send_stream_poll_reset`). Regression `poll_reset_after_clean_eos_must_not_hang`.
- **Fix branch:** `fix/poll-reset-after-end-stream`
- **Change:** `Closed(EndStream)` → `Err(UserError::InactiveStreamId)`; docs note clean close does not hang.

### F32 — Pseudo-headers in trailers accepted (malformed)
- **Severity:** Medium (protocol): RFC 9113 §8.1 requires trailer sections not include pseudo-header fields. `load_hpack` still places `:status`/etc. into `Pseudo` (no trailer context). `recv_trailers` called `into_fields()` and dropped Pseudo without error.
- **Evidence:** Response headers then HEADERS+EOS carrying `:status` — pre-fix accepted (empty trailers); post-fix `RST_STREAM(PROTOCOL_ERROR)`, connection survives. Regression `recv_trailers_with_pseudo_header_is_stream_error`.
- **Fix branch:** `fix/reject-pseudo-in-trailers`
- **Change:** Reject non-empty `Pseudo` (and content-length mismatch) **before** `recv_close` so `send_reset` is not a no-op on already-closed streams; `Pseudo::is_none()`.

### F33 — Informational (1xx) HEADERS with END_STREAM accepted
- **Severity:** Medium (protocol): Informational responses must not end the message. Pre-fix `recv_open` treated EOS first and half-closed remote, then queued `InformationalHeaders` — client never gets a final response. Go: `"1xx informational response with END_STREAM flag"`.
- **Evidence:** `100 Continue` with EOS → pre-fix half-closed recv; post-fix `RST_STREAM(PROTOCOL_ERROR)`. Regression `informational_response_with_end_stream_is_stream_error`.
- **Fix branch:** `fix/reject-informational-end-stream`
- **Change:** Reject `is_informational() && is_end_stream()` before `recv_open`.

### F34 — Content-Length on 1xx applied to final message body
- **Severity:** Medium (protocol / correctness): `recv_headers` set `stream.content_length` from any HEADERS frame before branching on informational. A `100 Continue` with `Content-Length` bound the final body; final 200 without CL then failed `ensure_content_length_zero` (or enforced the wrong length).
- **Evidence:** 100 + `Content-Length: 100`, then 200 + 5-byte DATA EOS — pre-fix stream PROTOCOL_ERROR; post-fix body `hello` accepted. Regression `informational_content_length_does_not_apply_to_final_body`.
- **Fix branch:** `fix/ignore-content-length-on-1xx`
- **Change:** Skip Content-Length bookkeeping when `frame.is_informational()`.

### F35 — Uncapped informational (1xx) HEADERS (memory DoS)
- **Severity:** Medium (resource / DoS): Each 1xx is stored as `Event::InformationalHeaders` in `pending_recv` until drained or skipped by `poll_response`. A peer could flood 1xx without a final response. Go caps at `max1xxResponses = 5`.
- **Evidence:** Six 100 Continue frames — post-fix sixth gets `RST_STREAM(ENHANCE_YOUR_CALM)`; first five still deliverable. Regression `too_many_informational_responses_is_stream_error`.
- **Fix branch:** `fix/cap-recv-informational`
- **Change:** `Stream::recv_informational_count` + `DEFAULT_MAX_RECV_INFORMATIONAL` (5); reject further 1xx with ENHANCE_YOUR_CALM.

### F36 — Response HEADERS missing `:status` accepted as 200
- **Severity:** Medium (protocol): RFC 9113 §8.3.2 requires exactly one `:status` on responses. Client `convert_poll_message` only set status when present; `http::Response::builder` defaults to 200 OK. Go: `"malformed response from server: missing status pseudo header"`.
- **Evidence:** Server sends HEADERS without pseudo + EOS after client request EOS — pre-fix delivered status 200; post-fix `RST_STREAM(PROTOCOL_ERROR)`. Check runs **before** `recv_open` so RST is not dropped on already-closed stream. Regression `response_headers_missing_status_is_stream_error`.
- **Fix branch:** `fix/reject-response-missing-status`
- **Change:** Client-side reject missing status before `recv_open`; keep defensive check in `convert_poll_message`.

### F37 — Request missing `:path` / CONNECT missing `:authority` accepted
- **Severity:** Medium (protocol): RFC 9113 §8.3.1 requires `:path` on all non-CONNECT requests; §8.5 / §8.3.1 require `:authority` on CONNECT (incl. extended CONNECT / RFC 8441). Server `convert_poll_message` only rejected missing path for *extended* CONNECT, and never required CONNECT authority.
- **Evidence:**
  - GET with `:method`+`:scheme` only (no authority, no path): scheme dropped without authority; `http::Uri::from_parts` empty parts succeeds → request delivered. Post-fix `RST_STREAM(PROTOCOL_ERROR)`.
  - GET with authority but no path was already rejected via Uri builder (`path missing`); explicit check now covers both.
  - CONNECT with only `:method`: delivered pre-fix; post-fix PROTOCOL_ERROR.
  - Regressions: `reject_request_missing_path_pseudo`, `reject_connect_missing_authority_pseudo`.
- **Fix branch:** `fix/reject-request-missing-path-authority`
- **Change:** In `server::Peer::convert_poll_message`, `!is_connect || has_protocol` → require path (same shape as scheme); `is_connect && authority.is_none()` → malformed. Also applies to received PUSH_PROMISE request conversion.

### F38 — Response HEADERS with request pseudo-headers accepted
- **Severity:** Medium (protocol): RFC 9113 §8.3.2 forbids `:method`, `:scheme`, `:authority`, `:path`, and `:protocol` on responses. Client only required `:status` (F36) and ignored any request-side pseudos, so a peer could send `:status` + `:method` and the response was delivered as 200 OK.
- **Evidence:** Response HEADERS with `:status: 200` and `:method: GET` + EOS after request EOS — pre-fix client completed successfully; post-fix `RST_STREAM(PROTOCOL_ERROR)`. Regression `response_headers_with_request_pseudo_is_stream_error`.
- **Fix branch:** `fix/reject-response-request-pseudos`
- **Change:** `Pseudo::has_request_pseudos()`; reject in `Recv::recv_headers` before `recv_open` (client); defensive reject in `client::Peer::convert_poll_message`.

### F39 — Mismatched multiple `Content-Length` values accepted
- **Severity:** Medium (protocol / framing): RFC 9110 §8.6 requires rejecting messages with multiple `Content-Length` fields whose decimal values differ (unreliable framing). `Recv::recv_headers` used `HeaderMap::get` (first value only), so `Content-Length: 5` + `Content-Length: 6` set `Remaining(5)` and the response was accepted.
- **Evidence:** Response headers with two differing CL values — pre-fix client got Ready response with content_length Remaining(5); post-fix `RST_STREAM(PROTOCOL_ERROR)`. Identical duplicate CL values still accepted. Regression `mismatched_content_length_headers_is_stream_error`.
- **Fix branch:** `fix/reject-mismatched-content-length`
- **Change:** Walk `get_all(CONTENT_LENGTH)`; require all values parse equal; first establishes the length.

### F40 — `Content-Length` in trailer HEADERS accepted / generatable
- **Severity:** Medium (protocol): RFC 9113 §8.1 forbids framing-related trailer fields (`Content-Length`, `Transfer-Encoding`). TE/connection headers already rejected in `load_hpack` / `check_headers`; `Content-Length` was neither connection-specific nor checked in trailer context.
- **Evidence:** Response headers then trailer HEADERS+EOS with `content-length: 5` — pre-fix stream closed EndStream and delivered trailers; post-fix `RST_STREAM(PROTOCOL_ERROR)`. `send_trailers` with CL was `Ok` pre-fix. Regressions: `recv_trailers_with_content_length_is_stream_error`; send assertion in `send_trailers_rejects_connection_specific_headers`.
- **Fix branch:** `fix/reject-content-length-in-trailers`
- **Change:** `recv_trailers` rejects `CONTENT_LENGTH` before `recv_close`; `send_trailers` returns `MalformedHeaders` before state transition.

### F41 — GOAWAY with non-zero stream id accepted
- **Severity:** Medium (protocol): RFC 9113 §6.8 requires GOAWAY frames to be associated with stream 0; any other stream identifier is a connection error PROTOCOL_ERROR. `Settings::load` / `Ping::load` already enforced stream 0; `GoAway::load` only took the payload and ignored the frame header stream id.
- **Evidence:** Raw GOAWAY frame with stream_id=1, last_stream_id=0, NO_ERROR — pre-fix decoded as a normal GoAway; post-fix codec error PROTOCOL_ERROR. Regression `read_goaway_nonzero_stream_id_is_connection_error`. Valid stream-0 GOAWAY with debug data still works.
- **Fix branch:** `fix/goaway-requires-stream-zero`
- **Change:** `GoAway::load(head, payload)` returns `InvalidStreamId` when `!head.stream_id().is_zero()`; `framed_read` passes `head`.

### F42 — Request `Host` differing from `:authority` accepted
- **Severity:** Medium (protocol / authority confusion): RFC 9113 §8.3.1 says a server SHOULD treat a request as malformed when `Host` identifies a different entity than `:authority`. Pre-fix `convert_poll_message` left both fields intact; applications could see `req.uri().authority()` = example.com while `Host: evil.example`. Go is aligning on reject (golang/go#80065).
- **Evidence:** Headers with `:authority: example.com` + `Host: evil.example` — pre-fix request delivered; post-fix `RST_STREAM(PROTOCOL_ERROR)`. Matching `Host: example.com` still accepted. Regressions: `reject_host_header_differing_from_authority`, `matching_host_with_authority_is_accepted`.
- **Fix branch:** `fix/reject-host-authority-mismatch`
- **Change:** In `server::Peer::convert_poll_message`, if both Host and `:authority` present and byte-values differ → malformed. Applies to PUSH_PROMISE request conversion too.

### F43 — 204/205/304 response HEADERS without END_STREAM accepted
- **Severity:** Medium (protocol): RFC 9110: 204 No Content, 205 Reset Content, and 304 Not Modified are terminated by the header section and cannot include content or trailers. Pre-fix only special-cased non-zero Content-Length + EOS for 204/304; a 204 without END_STREAM left the stream recv-streaming so subsequent DATA was delivered as a body.
- **Evidence:** Response HEADERS `:status: 204` (no EOS) then DATA — pre-fix client got 204 + body bytes; post-fix `RST_STREAM(PROTOCOL_ERROR)` on the HEADERS. Valid 204 with EOS still works. Regression `no_content_without_end_stream_is_stream_error`.
- **Fix branch:** `fix/reject-no-content-without-end-stream`
- **Change:** In `Recv::recv_headers` (client), if status is 204/205/304 and `!frame.is_end_stream()`, library reset PROTOCOL_ERROR before `recv_open`.

### F44 — `:authority` with userinfo accepted
- **Severity:** Medium (protocol / security): RFC 9113 §8.3.1: `:authority` MUST NOT include the deprecated userinfo subcomponent for `http`/`https`. `http::uri::Authority` accepts `user:pass@host`, so server `convert_poll_message` delivered the request (URI authority with credentials).
- **Evidence:** HEADERS with `:authority: user:pass@example.com` — pre-fix request accepted; post-fix `RST_STREAM(PROTOCOL_ERROR)`. Regression `reject_authority_with_userinfo`.
- **Fix branch:** `fix/reject-authority-userinfo`
- **Change:** Reject when `:authority` bytes contain `@` before Authority parse. Also covers PUSH_PROMISE request conversion.

### F45 — Outbound URI userinfo generated as `:authority`
- **Severity:** Medium (protocol / security): F44 sibling on the generate path. RFC 9113 §8.3.1 forbids generating `:authority` with userinfo. `Pseudo::request` copies `http::Uri` authority verbatim, so `Request` URIs like `https://user:pass@example.com/` produced illegal HEADERS on the wire (and push_request would too).
- **Evidence:** `send_request` with userinfo URI succeeded pre-fix (HEADERS queued with userinfo authority); post-fix `UserError::MalformedHeaders` before open. Connection remains usable for a clean follow-up request. Regression `outbound_uri_userinfo_is_user_error`.
- **Fix branch:** `fix/reject-outbound-authority-userinfo`
- **Change:** After Host promotion, reject `@` in `:authority` in client `convert_send_message` and server `convert_push_message`.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860
### S2 — sticky poll → F4
### S3 — InFlightData::Drop capacity leak — false positive: Drop means codec still owns the frame and will write it; remaining body in the Take is intentional cancel discard, only the charged chunk is sent. FC accounting matches wire intent.
### #878 / #880 — fixed upstream
### `dec_send_window` underflow — i32 extremes only
### #848 clone ready-at-max-open — design (queue beyond max); F9 only fixes pending_open occupancy hole
### Go #80035 SETTINGS window overflow — h2 already FLOW_CONTROL_ERROR via `inc_window` (matches Go intent)
### #882 `is_end_stream` false after reset — intentional (#810); sticky `data()` fixed by F4

## Suspects
None active.
