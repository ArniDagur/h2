# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`

### F3 — Connection recv `in_flight` not released on non-Reset DATA errors
- **Severity:** Medium — connection flow-control capacity leak after failed DATA processing post-window consume (e.g. library GoAway paths). Stream-window Reset was released by Streams; GoAway was not.
- **Evidence:** `Streams::recv_data` only called `release_connection_capacity` on `Err(Reset)`; `recv_data` could return GoAway after `consume_connection_window`. Unit test `stream_window_error_releases_connection_in_flight`.
- **Fix branch:** `fix/recv-data-error-releases-conn-capacity`
- **Change:** release inside `recv_data` on any post-consume error; drop Streams-side release.

## Instrumentation

### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds when summing **slab**
- Unlinked streams (closed, still referenced) keep `in_flight_recv_data`; do not sum only `ids`.

## Dismissed
### S1 — #853 — likely fixed by #860

## Suspects
### S2 — Sticky `poll_data` after reset (#882)
