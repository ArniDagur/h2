# Comparisons (h2 vs nghttp2 / Go net/http2)

Cases where h2 matches reference implementations → **spec interpretation, not a fix-worthy bug**.

## Logged

### SETTINGS_INITIAL_WINDOW_SIZE decrease (send windows)
- **Go (`net/http2`):** on SETTINGS, adjusts **all** streams' flow windows by `new-old` (`processSettingInitialWindowSize` / client transport loop). Negative windows allowed; increase past 2^31-1 → connection FLOW_CONTROL_ERROR.
- **Rust h2:** adjusts open send streams; skips `is_send_closed() && buffered_send_data == 0` (no further DATA). On decrease, reclaims connection-assigned capacity when `available > window_size` (as_size floors negatives) and reassigns via `pending_capacity`.
- **Verdict:** difference is h2's internal connection-capacity assignment model, not a clear RFC violation. Multi-stream reclaim of connection assignment on decrease works; F6 was an h2-only waiter bug on top of that path (`poll_capacity` / `send_capacity_inc`), not a Go/nghttp2 mismatch. Underflow of `i32` window on extreme SETTINGS still maps to FLOW_CONTROL_ERROR (TODO in `send.rs`).

## Planned comparisons
- Connection window recovery thresholds (h2 uses 1/2 unclaimed ratio) vs nghttp2/Go auto window update strategies.
- Behavior when receiver drops interest mid-stream (RST_STREAM NO_ERROR / CANCEL) vs stream WINDOW_UPDATE suppression.
