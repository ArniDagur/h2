# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Severity:** Low–medium (API correctness / misclassification).
- **Evidence:** `ensure_recv_open` used `library_go_away` for `ScheduledLibraryReset`.
- **Fix branch:** `fix/scheduled-reset-error-kind`
- **Change:** `ensure_recv_open(stream_id)` → `library_reset(stream_id, reason)`.

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Severity:** Medium if hit (stream hang); latent/defensive.
- **Evidence:** `pop_frame` / `push_back_frame` deferred without ensuring `pending_capacity`.
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`
- **Test:** `connection_window_update_resumes_starved_buffered_stream`.

## Dismissed / not reproduced

### S1 — #853 connection capacity logical deadlock → **likely fixed by #860**
- Stress test `logical_deadlock_max_concurrent_streams_stress` (50×40 POSTs, max_concurrent=10) passes repeatedly.
- #860: no capacity assign while `is_pending_open`.
- #930: `RecvStream` drop releases connection capacity (helps tests that forget explicit release).
- Caveat: original PR #852 harness lacked `release_capacity` / `ready()`; pure FC stall can look like deadlock. Reopen only with a harness that releases windows and still hangs.

## Suspects

### S2 — Sticky `poll_data` errors after reset (#882)
- After non-EOS reset, `ensure_recv_open` keeps returning `Err`; `is_end_stream()` false. EOS+reset improved by #922. API ergonomics gray area.

## Not bugs / intentional
- `RecvStream` drop releases connection FC (#930) but not stream window: peer blocks on that stream only.
- Skipping closed streams in `assign_connection_capacity`: capacity remains on connection `flow.available`.
