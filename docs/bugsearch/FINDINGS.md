# Findings

## Confirmed (fixed on experimental / per-bug branch)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Severity:** Low–medium (API correctness / misclassification). Callers using `Error::is_go_away()` could treat a stream-local cancel as connection death.
- **Evidence:** `State::ensure_recv_open` matched `Closed(Cause::ScheduledLibraryReset(reason))` → `Error::library_go_away(reason)`. Unit test `scheduled_library_reset_is_stream_reset_not_goaway`.
- **When it matters:** Implicit/library scheduled resets (oversize HEADERS path scheduling `PROTOCOL_ERROR`, etc.) while recv polls still observe state before RST is flushed as `Cause::Error(Reset)`.
- **Fix branch:** `fix/scheduled-reset-error-kind`
- **Change:** `ensure_recv_open(stream_id)` returns `library_reset(stream_id, reason)`.

## Suspects (not confirmed this fire)

### S1 — #853 connection capacity logical deadlock
- Open upstream. Theory: capacity assigned to `pending_open` streams starvation. #860 stops assigning capacity to pending-open streams — may have fixed it. Needs stress repro.

### S2 — Sticky `poll_data` errors after reset (#882)
- After non-EOS reset, `ensure_recv_open` keeps returning `Err` forever; `is_end_stream()` stays false. Partly by design; EOS+reset improved by #922 (`ErrorAfterEndStream`). Sticky error vs `None` after first error is API ergonomics / Stream contract gray area.

### S3 — `push_back_frame` when `available == 0` does not ensure `pending_capacity`
- Remainder of a large DATA frame reclaimed after partial send only re-queues to `pending_send` if `available > 0`. Relies on prior `pending_capacity` membership or a later stream WINDOW_UPDATE. No hang repro yet; worth invariant instrumentation.

## Not bugs / intentional
- Dropping `RecvStream` without releasing stream-level window (only connection was fixed in #930): stops further useful receipt on that stream; peer blocks on stream FC. Connection capacity returned so other streams progress.
