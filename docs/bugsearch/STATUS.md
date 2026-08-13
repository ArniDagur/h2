# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F62 pending merge)

## Current focus
F62: non-CONNECT requests require `:authority` or Host.

## Last actions
1. Confirmed **F62**: nghttp2 requires `:authority` or Host; h2 accepted scheme+path-only (not routable).
2. Fix: require one of them; Host alone fills URI authority (origin-form).
3. Regressions: `reject_request_without_authority_or_host`, `request_with_host_without_authority_pseudo`.

## Next recommended step
1. Package PRs for F3–F62.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
