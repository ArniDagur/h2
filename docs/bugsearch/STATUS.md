# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
RecvStream cancellation / error delivery (sticky poll after reset).

## Last actions
1. Investigated **S2 / #882**: after remote (or local) stream error, `schedule_recv` used `ensure_recv_open()?` so every `poll_data` re-yielded `Some(Err(reset))` forever; `is_end_stream()` correctly stays false for unclean ends (#810).
2. Fixed with `stream.recv_err_delivered`: first error poll returns `Some(Err)`, later polls return `None`. Same for trailers / push / informational empty paths.
3. Regression test `recv_stream_reset_error_is_not_sticky` (stream_states). Related suites green.

## Next recommended step
1. SETTINGS_INITIAL_WINDOW_SIZE decrease + multi-stream differential vs Go/nghttp2.
2. Or shared `send_task` multi-waiter missed wakeup (capacity vs reset).
3. Or full `cargo test` / prepare PR notes for F3+F4.

## Blockers
None.
