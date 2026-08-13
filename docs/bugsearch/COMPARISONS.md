# Comparisons (h2 vs nghttp2 / Go net/http2)

Cases where h2 matches reference implementations → **spec interpretation, not a fix-worthy bug**.

## Logged

### SETTINGS_INITIAL_WINDOW_SIZE decrease (send windows)
- **Go (`net/http2`):** on SETTINGS, adjusts **all** streams' flow windows by `new-old` (`processSettingInitialWindowSize` / client transport loop). Negative windows allowed; increase past 2^31-1 → connection FLOW_CONTROL_ERROR.
- **Rust h2:** adjusts open send streams; skips `is_send_closed() && buffered_send_data == 0` (no further DATA). On decrease, reclaims connection-assigned capacity when `available > window_size` (as_size floors negatives) and reassigns via `pending_capacity`.
- **Verdict:** difference is h2's internal connection-capacity assignment model, not a clear RFC violation. Multi-stream reclaim of connection assignment on decrease works; F6 was an h2-only waiter bug on top of that path (`poll_capacity` / `send_capacity_inc`), not a Go/nghttp2 mismatch. Underflow of `i32` window on extreme SETTINGS still maps to FLOW_CONTROL_ERROR (TODO in `send.rs`).

### Local SETTINGS_INITIAL_WINDOW_SIZE increase timing (recv)
- **Go:** on *receipt* of peer SETTINGS, adjusts send windows immediately (`processSettingInitialWindowSize`). Dynamic local recv expansion is not a separate public API path in the same way.
- **Rust h2 (pre-F10):** applied local INITIAL_WINDOW_SIZE changes only on SETTINGS_ACK → race if peer sent under new window first.
- **Rust h2 (F10):** increases applied when SETTINGS is written; decreases still on ACK (peer must shrink first).
- **Verdict:** F10 is an h2-local race fix, not a Go/nghttp2 mismatch.

### Connection WINDOW_UPDATE recovery threshold
- **Go (`inflow.add`):** send WINDOW_UPDATE when unsent ≥ 4KiB **or** unsent would at least double the peer’s current window (`inflowMinRefresh = 4<<10`).
- **Rust h2:** send when unclaimed ≥ half of peer’s current window (`window_size/2`), or any unclaimed if window ≤ 0 (SETTINGS decrease).
- **Verdict:** both batch updates; different heuristics (fixed 4KiB+double vs 50% ratio). Spec interpretation / performance tradeoff, not a correctness bug.

### Receiver drops interest mid-stream
- **Rust h2:** `RecvStream` drop does not RST (may still send on `SendStream`); F14 restores stream+conn FC for ignored DATA. Full ref drop → implicit CANCEL (or server NO_ERROR after complete response).
- **Planned:** further compare NO_ERROR vs CANCEL timing to Go/nghttp2 if new reports appear.

### Go #80035 — SETTINGS_INITIAL_WINDOW_SIZE overflow on existing streams
- **Go:** silent leave window as-is was wrong; fix reports connection FLOW_CONTROL_ERROR when increase exceeds 2^31-1.
- **Rust h2:** `FlowControl::inc_window` rejects overflow / `> MAX_WINDOW_SIZE` with FLOW_CONTROL_ERROR; integration coverage in flow_control tests (overflow after grow to max).
- **Verdict:** match (not a fix-worthy h2 gap).

### DATA on non-recv-streaming stream
- **Go (`processData`):** idle/id0 → connection PROTOCOL_ERROR; otherwise not open → stream STREAM_CLOSED + connection FC refund.
- **Rust h2 (pre-F23):** any `!is_recv_streaming` with stream in store → GOAWAY PROTOCOL_ERROR (over-aggressive).
- **Rust h2 (F23):** stream STREAM_CLOSED + `ignore_data` (connection FC); forgotten streams already STREAM_CLOSED. Idle not-in-store still connection PROTOCOL_ERROR.
- **Verdict:** F23 aligns with Go/RFC for late DATA after EOS.

### HEADERS after recv EOS
- **Go (`processHeaders`):** `stateHalfClosedRemote` → stream STREAM_CLOSED.
- **Rust h2 (pre-F24):** treated as trailers → `recv_close` → GOAWAY PROTOCOL_ERROR.
- **Rust h2 (F24):** `is_recv_end_stream` → stream STREAM_CLOSED before `recv_trailers`.
- **Verdict:** F24 aligns with Go/RFC.

### Informational (1xx) HEADERS with END_STREAM
- **Go:** rejects `"1xx informational response with END_STREAM flag"`.
- **Rust h2 (pre-F33):** `recv_open` applied EOS first → half-closed remote, then queued InformationalHeaders.
- **Rust h2 (F33):** stream PROTOCOL_ERROR before `recv_open`.
- **Verdict:** F33 aligns with Go/RFC.

### Cap on number of 1xx responses
- **Go:** `max1xxResponses = 5` when the user does not examine 1xx via trace hook.
- **Rust h2 (pre-F35):** unlimited queue in `pending_recv`.
- **Rust h2 (F35):** hard cap 5 per stream; further 1xx → ENHANCE_YOUR_CALM (always, including when user polls informational).
- **Verdict:** F35 matches Go intent; slightly stricter when user drains 1xx (still capped at 5 total).

### 101 Switching Protocols
- **RFC 9113 §8.1:** HTTP/2 does not support 101 (Switching Protocols); Upgrade is not used on HTTP/2.
- **Rust h2 (pre-F57):** 101 treated as ordinary informational 1xx on recv and generatable via `send_informational`.
- **Rust h2 (F57):** stream PROTOCOL_ERROR on recv; `InvalidInformationalStatusCode` on generate.
- **Verdict:** F57 aligns with RFC; HTTP/1.1 Upgrade must not be carried over HTTP/2.

### Response HEADERS missing `:status`
- **Go:** `"malformed response from server: missing status pseudo header"`.
- **Rust h2 (pre-F36):** `http::Response::builder` defaulted to 200 OK.
- **Rust h2 (F36):** stream PROTOCOL_ERROR before `recv_open`.
- **Verdict:** F36 aligns with Go/RFC 9113 §8.3.2.

### Request missing `:path` / CONNECT missing `:authority`
- **RFC 9113 §8.3.1 / §8.5:** non-CONNECT MUST include `:path`; CONNECT MUST include `:authority`.
- **Rust h2 (pre-F37):** missing path only rejected for extended CONNECT; CONNECT without authority accepted; scheme-only GET without path accepted (scheme dropped, empty Uri).
- **Rust h2 (F37):** explicit PROTOCOL_ERROR in server `convert_poll_message` (also covers PUSH_PROMISE request conversion).
- **Verdict:** F37 is an h2 protocol correctness fix (malformed request acceptance).

### Empty / invalid `:scheme`
- **RFC 3986 §3.1:** scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
- **nghttp2 `check_scheme`:** same grammar (empty and non-ALPHA start rejected).
- **Rust h2 (pre-F59):** missing scheme rejected; present-but-empty accepted (`http::uri::Scheme` parses `""`).
- **Rust h2 (F59):** empty rejected.
- **Rust h2 (pre-F61):** digit-leading tokens like `"1http"` still accepted via Scheme parse.
- **Rust h2 (F61):** full grammar via `frame::is_valid_scheme` on recv and generate.
- **Verdict:** F59+F61 align with RFC/nghttp2; http crate remains more permissive at Uri layer.

### `:authority` or Host on non-CONNECT requests
- **nghttp2:** non-CONNECT requires `NGHTTP2_HTTP_FLAG__AUTHORITY | NGHTTP2_HTTP_FLAG_HOST`.
- **Rust h2 (pre-F62):** scheme+path only accepted (relative Uri, not routable).
- **Rust h2 (F62):** require `:authority` or Host; Host alone populates request URI authority (origin-form).
- **Verdict:** F62 matches nghttp2; Host-only path preserves HTTP/1.1 origin-form translation.

### TE: trailers case sensitivity
- **RFC 9110:** transfer-coding tokens are case-insensitive.
- **nghttp2:** `lstrieq("trailers", ...)` for TE.
- **Rust h2 (pre-F64):** exact byte match to `"trailers"` only.
- **Rust h2 (F64):** ASCII case-insensitive on recv and generate.
- **Verdict:** F64 matches RFC/nghttp2.

### Empty host in `:authority`
- **RFC 9110 §4.3.1:** sender MUST NOT generate an empty host identifier.
- **http::uri::Authority:** accepts `":"` / `":80"` with `host() == ""`.
- **Rust h2 (pre-F66):** accepted on recv; generatable via `https://:80/`.
- **Rust h2 (F66):** reject empty host after authority parse (recv + generate + Host-only).
- **Verdict:** F66 closes empty-host hole left by userinfo-only checks (F44/F45).

### Asterisk-form `:path`
- **RFC 9110 §7.1 / RFC 9113 §8.3.1:** `:path` of `*` is for OPTIONS only.
- **nghttp2:** for http/https, requires path-regular (`/`…) or (OPTIONS and path-asterisk).
- **Rust h2 (pre-F60):** any method with `:path: *` accepted via PathAndQuery.
- **Rust h2 (F60):** reject non-OPTIONS `*`; OPTIONS `*` still OK.
- **`:path` with `//` prefix:** nghttp2 counts as path-regular (starts with `/`); h2 matches — not fix-worthy.

### Response with request pseudo-headers
- **RFC 9113 §8.3.2:** responses MUST NOT include `:method`/`:scheme`/`:authority`/`:path`/`:protocol`.
- **Rust h2 (pre-F38):** only enforced missing `:status` (F36); request pseudos ignored if status present.
- **Rust h2 (F38):** stream PROTOCOL_ERROR before `recv_open` when any request pseudo is present.
- **Verdict:** F38 aligns with RFC; Go treats mixed request/response pseudos as malformed header blocks in similar spirit.

### Mismatched multiple Content-Length values
- **RFC 9110 §8.6:** differing multi CL values → message invalid; identical duplicates MAY be collapsed.
- **Rust h2 (pre-F39):** `HeaderMap::get` first value only → Remaining(first); body framed incorrectly.
- **Rust h2 (F39):** all CL field values must parse equal; else stream PROTOCOL_ERROR.
- **Rust h2 (pre-F53):** generate still accepted mismatched multi CL on `send_request`/`send_response`.
- **Rust h2 (F53):** `validate_outbound_content_length` rejects unparseable or differing values.
- **Verdict:** F39+F53 complete receive+generate for multi Content-Length.

### Content-Length in trailers
- **RFC 9113 §8.1:** framing fields (`Content-Length`, `Transfer-Encoding`) MUST NOT be sent as trailers.
- **Rust h2 (pre-F40):** TE rejected as connection-specific; CL accepted on recv and generatable via `send_trailers`.
- **Rust h2 (F40):** stream PROTOCOL_ERROR on recv; `MalformedHeaders` on send.
- **Verdict:** F40 aligns with RFC sender/receiver framing rules for trailers.

### GOAWAY stream identifier
- **RFC 9113 §6.8:** GOAWAY MUST be associated with stream 0; else connection PROTOCOL_ERROR.
- **Rust h2 (pre-F41):** `GoAway::load` ignored frame header stream id (SETTINGS/PING already checked stream 0).
- **Rust h2 (F41):** non-zero stream id → `InvalidStreamId` → connection PROTOCOL_ERROR.
- **Verdict:** F41 aligns with RFC / other connection-oriented frame loaders in h2.

### Host vs `:authority` on inbound requests
- **RFC 9113 §8.3.1:** server SHOULD treat mismatched Host / `:authority` as malformed; proxies MUST discard Host.
- **Go:** moving to reject conflicting Host (golang/go#80065).
- **Rust h2 (pre-F42):** both fields delivered; URI authority from `:authority` only.
- **Rust h2 (F42):** byte mismatch → stream PROTOCOL_ERROR; equal Host kept.
- **Verdict:** F42 is a recommended-server-behavior + security hardening fix (not a hard MUST, but interop/security high-signal).

### 204/205/304 without END_STREAM
- **RFC 9110:** these statuses are terminated by the header section; no content or trailers.
- **Rust h2 (pre-F43):** 204 without EOS left stream recv-streaming → DATA accepted as body.
- **Rust h2 (F43):** client rejects 204/205/304 HEADERS without END_STREAM as stream PROTOCOL_ERROR.
- **Rust h2 (pre-F47):** server `send_response(204, false)` still emitted non-EOS HEADERS (generate gap).
- **Rust h2 (F47):** `send_response` rejects 204/205/304 when `!end_of_stream`.
- **Verdict:** F43+F47 complete receive+generate for no-content statuses.

### `:authority` userinfo
- **RFC 9113 §8.3.1:** authority MUST NOT include deprecated userinfo for http/https (generate or accept).
- **Rust h2 (pre-F44/F45):** inbound Authority parse accepted userinfo; outbound `Pseudo::request` copied Uri authority with userinfo onto the wire.
- **Rust h2 (F44):** inbound `@` in `:authority` → stream PROTOCOL_ERROR.
- **Rust h2 (F45):** outbound after Host promotion, `@` in `:authority` → `MalformedHeaders` (client send + push convert).
- **Verdict:** F44+F45 complete the userinfo MUST for request convert paths.

### Final response vs interim 1xx API
- **RFC 9110/9113:** 1xx does not end the message; final response uses a non-1xx status; 1xx must precede the final status.
- **Rust h2 (pre-F46):** `send_response(1xx, eos)` emitted illegal 1xx+EOS HEADERS and closed send half.
- **Rust h2 (F46):** `send_response` rejects informational status; `send_informational` remains for interim 1xx.
- **Rust h2 (pre-F48):** `send_informational` after `send_response` still queued 1xx (docs claimed error).
- **Rust h2 (F48):** reject when local half is past AwaitingHeaders.
- **Rust h2 (pre-F54):** `poll_informational` after final headers consumed could hang on DATA at queue head.
- **Rust h2 (F54):** non-1xx queue head / post-headers state → `Ready(None)`.
- **Verdict:** F46+F48+F54 align generate + client poll for interim-before-final semantics.

### Content-Length on 1xx / 204 / 205 generate path
- **RFC 9110 §8.6:** server MUST NOT send Content-Length on 1xx or 204; 205 empty content; 304 MAY include CL.
- **RFC 9113 §8.1.1:** no-payload responses *may* carry non-zero CL with no DATA (receive resilience for 204/304).
- **Rust h2 (F49):** generate rejects CL on 1xx/204 and non-zero CL on 205; receive still allows peer CL on 204/304 (existing exception).
- **Verdict:** F49 is sender MUST NOT enforcement; asymmetric with receive by design (interop vs self-generation).

### Non-zero Content-Length with END_STREAM
- **RFC 9113 §8.1.1:** HEADERS+EOS with non-zero CL is malformed (body length cannot match).
- **Rust h2 (pre-F50):** receive rejected; generate still allowed `send_request`/`send_response` to emit illegal frames.
- **Rust h2 (F50):** generate rejects; 304 exempt (representation CL with empty body).
- **Verdict:** F50 closes generate/receive asymmetry for general messages.

### Traditional CONNECT and Content-Length
- **RFC 9110 §9.3.6 / RFC 9113 §8.5:** no Content-Length on traditional CONNECT request; client MUST ignore CL on successful CONNECT response; server MUST NOT send CL in 2xx CONNECT response.
- **Rust h2 (pre-F51):** 2xx CONNECT + CL bound `Remaining` → tunnel DATA over length = PROTOCOL_ERROR; request CL accepted both ways.
- **Rust h2 (F51):** mark traditional CONNECT streams; skip CL on 2xx responses; reject CL on traditional CONNECT generate + server convert.
- **Rust h2 (pre-F52):** server `send_response(200 + CL)` on CONNECT still succeeded.
- **Rust h2 (F52):** set `is_connect` on request accept; reject CL on 2xx `send_response`.
- **Verdict:** F51+F52 complete CONNECT CL rules for client receive, client/server request, and server 2xx generate; extended CONNECT unchanged.

### SETTINGS_ENABLE_PUSH from server
- **RFC 9113 §6.5.2:** server MUST NOT send ENABLE_PUSH = 1; only 0 or omit. Value other than 0/1 is already PROTOCOL_ERROR at frame load.
- **Rust h2 (pre-F55):** client applied ENABLE_PUSH=1 from server without error.
- **Rust h2 (F55):** client GOAWAY PROTOCOL_ERROR when applying that setting.
- **Verdict:** F55 enforces server-role prohibition on ENABLE_PUSH=1.

### Send capacity reservation max
- **RFC 9113 §6.9.1:** flow-control window max is 2^31-1.
- **Rust h2 (pre-F56):** `reserve_capacity(usize)` truncated via `as u32`; prioritize used `u32::MAX` cap.
- **Rust h2 (F56):** clamp to `MAX_WINDOW_SIZE` (2^31-1) at API and prioritize.
- **Verdict:** F56 is h2 API/FC correctness (not a Go mismatch).

## Planned comparisons
- Residual #848 API design.
