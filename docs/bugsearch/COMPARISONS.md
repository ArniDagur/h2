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

### Response with request pseudo-headers
- **RFC 9113 §8.3.2:** responses MUST NOT include `:method`/`:scheme`/`:authority`/`:path`/`:protocol`.
- **Rust h2 (pre-F38):** only enforced missing `:status` (F36); request pseudos ignored if status present.
- **Rust h2 (F38):** stream PROTOCOL_ERROR before `recv_open` when any request pseudo is present.
- **Verdict:** F38 aligns with RFC; Go treats mixed request/response pseudos as malformed header blocks in similar spirit.

## Planned comparisons
- Residual #848 API design.
