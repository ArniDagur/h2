# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F36: response HEADERS without `:status` accepted as 200 OK.

## Last actions
1. Confirmed **F36**: missing `:status` defaulted to 200 via `http::Response::builder`.
2. Fix: reject before `recv_open` (so RST is sent after request EOS); defensive check in `convert_poll_message`.
3. Regression: `response_headers_missing_status_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F36.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup hunt.

## Blockers
None.
