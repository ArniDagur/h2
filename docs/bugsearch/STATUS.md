# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F66 pending merge)

## Current focus
F66: empty host in `:authority` (e.g. `:80`).

## Last actions
1. Confirmed **F66**: RFC 9110 forbids empty host; `http::Authority` accepts `":80"`/`":"` with empty host.
2. Fix: reject empty host after parse on server recv, client send, push; Host-only path too.
3. Regressions: `reject_request_empty_host_in_authority`, `request_with_empty_host_authority_is_user_error`.

## Next recommended step
1. Package PRs for F3–F66.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
