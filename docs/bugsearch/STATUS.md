# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F73 @ `c764a91`)

## Current focus
F73: userinfo in Host header (Host-only path).

## Last actions
1. Confirmed **F73**: F44 rejected userinfo in `:authority`; Host-only origin-form still accepted `Host: user@host` via `http::Authority`.
2. Fix: reject `@` in Host before Authority parse on Host-only server path.
3. Regression: `reject_host_header_with_userinfo`.

## Next recommended step
1. Package PRs for F3–F73.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` residual).

## Blockers
None.
