# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Severity:** Low–medium (API correctness / misclassification).
- **Evidence:** `ensure_recv_open` used `library_go_away` for `ScheduledLibraryReset`.
- **Fix branch:** `fix/scheduled-reset-error-kind`
- **Change:** `ensure_recv_open(stream_id)` → `library_reset(stream_id, reason)`.

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Severity:** Medium if hit (stream hang until stream-level WU or never); latent/defensive.
- **Evidence:** `pop_frame` on `stream_capacity == 0` and `len > window_size`, and `push_back_frame` when `available == 0`, only buffered the frame and dropped the stream from `pending_send` without ensuring `pending_capacity`. Comment had TODO `debug_assert!(is_pending_send_capacity)`.
- **Normal path:** `try_assign_capacity` usually already queued `pending_capacity` when `has_unavailable`; hang requires stream to lose that membership.
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`
- **Change:** if `has_unavailable()`, `pending_capacity.push` after capacity-0 deferral / partial-frame reclaim.
- **Test:** `connection_window_update_resumes_starved_buffered_stream`.

## Suspects

### S1 — #853 connection capacity logical deadlock
- Open upstream. #860 stops assigning capacity to `pending_open` streams — may have fixed. Still needs stress confirmation.

### S2 — Sticky `poll_data` errors after reset (#882)
- After non-EOS reset, `ensure_recv_open` keeps returning `Err`; `is_end_stream()` false. EOS+reset improved by #922. API ergonomics / Stream contract gray area.

### S3 — (promoted to F2) capacity-0 requeue
- Was suspect; now hardened as F2.

## Not bugs / intentional
- `RecvStream` drop releases connection FC (#930) but not stream window: peer blocks on that stream only.
- Skipping closed streams in `assign_connection_capacity`: capacity remains on connection `flow.available`, not orphaned.
