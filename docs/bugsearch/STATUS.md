# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F69 @ `59b0efd`)

## Current focus
F69: non path-absolute `:path` / query-only URI mis-encoding.

## Last actions
1. Confirmed **F69**: RFC 9113 / nghttp2 require path-absolute `:path`; `PathAndQuery` accepts `?q=1`; `Pseudo::request` emitted that for `https://example.com?q=1`.
2. Fix: normalize query-only to `/`+query on generate; reject non-`/` / non-OPTIONS-`*` paths on recv/send/push.
3. Regressions: `reject_request_path_without_leading_slash`, `query_only_uri_sends_slash_query_path`.

## Next recommended step
1. Package PRs for F3–F69.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` authority residual).

## Blockers
None.
