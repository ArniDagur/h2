# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F28: client connection-headers check after `open()` burned stream ids.

## Last actions
1. Confirmed **F28** (F21 residual): `Send::check_headers` ran only inside `send_headers` after `open`/insert, so `Connection`/`Transfer-Encoding`/etc. still advanced `next_stream_id`.
2. Fix: `Send::check_headers` after convert, before `open()`.
3. Regression: `connection_header_does_not_burn_stream_id` (bad request then good → stream 1).

## Next recommended step
1. Package PRs for F3–F28.
2. Or residual #848 API ready-at-max-open.
3. Or hunt new FC/wakeup/cancel bugs (id-burn validation paths closed for client+push).

## Blockers
None.
