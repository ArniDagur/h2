# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F61 @ `d8b2eb0`)

## Current focus
F61: invalid `:scheme` tokens (digit-leading / non-RFC 3986).

## Last actions
1. Confirmed **F61**: RFC 3986 / nghttp2 require scheme to start with ALPHA; `http::uri::Scheme` accepts `"1http"` etc. F59 only covered empty.
2. Fix: `frame::is_valid_scheme` + enforce on server recv, client send, push convert.
3. Regression: `reject_request_digit_leading_scheme`; unit `scheme_grammar`.

## Next recommended step
1. Package PRs for F3–F61.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
