# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F59 pending merge)

## Current focus
F59: empty `:scheme` accepted on receive and generate.

## Last actions
1. Confirmed **F59**: empty scheme string is not a valid RFC 3986 scheme; `http::uri::Scheme` still parses `""`, so present-but-empty `:scheme` bypassed missing-scheme checks.
2. Fix: reject empty on server recv convert, client send convert, and push convert.
3. Regressions: `reject_request_empty_scheme_pseudo`, `request_with_empty_scheme_is_user_error`.

## Next recommended step
1. Package PRs for F3–F59.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (e.g. non-OPTIONS `:path` `*`, `//` path-absolute).

## Blockers
None.
