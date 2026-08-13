# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F78)

## Current focus
F78: SendResponse after send_response no longer pins reserved send capacity.

## Last actions
1. Confirmed **F78**: `send_response` clones `StreamRef` for `SendStream` but `SendResponse` kept `send_ref_count` > 0.
2. Dropping only `SendStream` left `reserve_capacity` assigned until `SendResponse` dropped; other streams' DATA starved.
3. Fix: `owns_send` + `release_send_ownership` after transferring send to `SendStream`.
4. Regression: `drop_send_stream_reclaims_reserved_capacity_despite_send_response`.

## Next recommended step
1. Package PRs for F3–F78.
2. Residual #848 API ready-at-max-open.
3. `settings_decrease_wakes_poll_capacity_on_reclaim` drain loop still infinite after F29 (test hygiene).
4. Pre-existing (not this fire): `recv_too_big_headers`, `frame_on_pending_open_stream_is_conn_error` (F23 stream vs conn), `recv_invalid_push_promise_headers_is_stream_protocol_error`.

## Blockers
None.
