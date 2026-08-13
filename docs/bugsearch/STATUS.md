# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F79)

## Current focus
F79: DATA on pending_open (idle) is connection PROTOCOL_ERROR.

## Last actions
1. Confirmed **F79**: F23 STREAM_CLOSED applied to DATA on `pending_open` (never sent; peer-idle).
2. RST/WU/HEADERS on pending_open already GOAWAYed; DATA left the connection up (`poll_ready` Ok).
3. Fix: `Streams::recv_data` GOAWAY PROTOCOL_ERROR when `is_pending_open`.
4. Existing test `frame_on_pending_open_stream_is_conn_error` now passes; F23 after-EOS tests still pass.

## Next recommended step
1. Package PRs for F3–F79.
2. Residual #848 API ready-at-max-open.
3. `settings_decrease_wakes_poll_capacity_on_reclaim` drain loop still infinite after F29 (test hygiene).
4. Pre-existing (not this fire): `recv_too_big_headers`, `recv_invalid_push_promise_headers_is_stream_protocol_error`, `srv_window_update_on_lower_stream_id`.

## Blockers
None.
