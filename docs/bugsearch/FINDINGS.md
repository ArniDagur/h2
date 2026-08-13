# Findings

## Confirmed (fixed on experimental / per-bug branches)

### F1 — Scheduled library reset surfaced as GOAWAY
- **Fix branch:** `fix/scheduled-reset-error-kind`

### F2 — Capacity-0 send path may leave stream off `pending_capacity`
- **Fix branch:** `fix/pending-capacity-requeue-on-zero`

### F3 — Connection recv `in_flight` not released on non-Reset DATA errors
- **Fix branch:** `fix/recv-data-error-releases-conn-capacity`

### F4 — Sticky `poll_data` / `data()` errors after stream reset (#882)
- **Severity:** Medium (API footgun / busy loop): after one `Some(Err(reset))`, further polls repeated the same error instead of ending the stream.
- **Evidence:** `schedule_recv` used `ensure_recv_open()?` → `Poll::Ready(Some(Err(_)))` every time while state stayed `Closed(Error)`. Issue #882; seanmonstar notes local cancel stickiness.
- **Note:** `#922` `ErrorAfterEndStream` already makes clean EOS+reset return `None` / `is_end_stream`. Sticky issue remains for errors without prior EOS.
- **Fix branch:** `fix/recv-stream-error-not-sticky`
- **Change:** `recv_err_delivered` flag; first error delivered once, then `None`. `is_end_stream()` still false for unclean end.
- **Test:** `recv_stream_reset_error_is_not_sticky`.

## Instrumentation
### I1 — Send capacity conservation (debug) — holds
### I2 — Recv in-flight conservation (debug) — holds (sum **slab**)

## Dismissed
### S1 — #853 — likely fixed by #860

## Suspects
None active (S2 promoted to F4).
