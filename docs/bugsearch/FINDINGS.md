# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Severity:** Low–medium (API correctness / misclassification).
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Severity:** Medium if hit (stream hang); latent/defensive.
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`
- **Test:** `connection_window_update_resumes_starved_buffered_stream`.

## Instrumentation (no bug found yet)

### I1 — Send capacity conservation (debug asserts)
- **Invariant:** `Σ stream.send available + conn.available == conn.window` (signed i32 math).
- **Also:** no `pending_open` stream may hold `send_flow.available != 0`.
- **Result:** holds under existing integration suites (debug profile).
- **Location:** `Prioritize::debug_assert_send_capacity_conservation`, called from `buffer_pending`, conn WU, SETTINGS path.

## Dismissed / not reproduced

### S1 — #853 connection capacity logical deadlock → **likely fixed by #860**
- Stress test `logical_deadlock_max_concurrent_streams_stress` passes repeatedly.
- See prior notes on #852 harness missing `release_capacity` / `ready()`.

## Suspects

### S2 — Sticky `poll_data` errors after reset (#882)
- After non-EOS reset, `ensure_recv_open` keeps returning `Err`; `is_end_stream()` false. EOS+reset improved by #922.

## Not bugs / intentional
- `RecvStream` drop releases connection FC (#930) but not stream window.
- Skipping closed streams in `assign_connection_capacity`: capacity remains on conn `available`.
