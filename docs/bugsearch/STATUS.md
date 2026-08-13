# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F79)

## Current focus
No new library bug this fire. Three failing tests are stale after F32/F36/F74.

## Last actions
1. Investigated pre-existing failures for hang/FC/connection-kill.
2. `recv_too_big_headers`: mock expected only RST(3); library now also RST(1) (oversize after request+response EOS) — F74-style emit-RST-before-close. Test hygiene.
3. `srv_window_update_on_lower_stream_id`: `headers(7).eos()` has no `:status` → F36 PROTOCOL_ERROR instead of RST(2) CANCEL. Test hygiene.
4. `recv_invalid_push_promise_headers_is_stream_protocol_error`: `validate_request` still rejects POST and CL≠0; `ps.len()==2` is parent second 404-as-trailers (F32) surfacing on `push_promises` collect. Test hygiene.
5. No new hang/FC/wakeup bug found.

## Next recommended step
1. Package PRs for F3–F79.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: the three stale tests + F29 drain loop.

## Blockers
None.
