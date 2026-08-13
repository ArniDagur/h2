# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F38: response HEADERS with request pseudo-headers (`:method`, etc.) accepted.

## Last actions
1. Confirmed **F38**: `:status` + `:method` delivered as normal 200; RFC 9113 §8.3.2 forbids request pseudos on responses.
2. Fix: reject `has_request_pseudos()` before `recv_open` (client); defensive check in `convert_poll_message`.
3. Regression: `response_headers_with_request_pseudo_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F38.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (trailers-without-EOS already handled; PRIORITY / GOAWAY edges).

## Blockers
None.
