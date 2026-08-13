# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F44: `:authority` with userinfo (`user:pass@host`) accepted.

## Last actions
1. Confirmed **F44**: `user:pass@example.com` as `:authority` delivered to `poll_accept`.
2. Fix: reject `@` in `:authority` in `server::Peer::convert_poll_message` before URI parse.
3. Regression: `reject_authority_with_userinfo`. Note: HEAD non-empty DATA already PROTOCOL_ERROR via `ContentLength::Head`.

## Next recommended step
1. Package PRs for F3–F44.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (outbound userinfo in URI → `:authority` optional follow-up).

## Blockers
None.
