# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F64 @ `ab2367a`)

## Current focus
F64: TE: trailers case-insensitive comparison.

## Last actions
1. Confirmed **F64**: RFC 9110 / nghttp2 treat TE transfer-coding as case-insensitive; h2 required exact `"trailers"`.
2. Fix: ASCII case-insensitive compare on recv (load_hpack) and generate (check_headers, push).
3. Regressions: `request_te_trailers_case_insensitive`, trailers test update.

## Next recommended step
1. Package PRs for F3–F64.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (try_assign saturating_sub).

## Blockers
None.
