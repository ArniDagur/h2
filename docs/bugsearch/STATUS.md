# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F45: outbound URI userinfo generated as `:authority` on the wire.

## Last actions
1. Confirmed **F45**: `send_request(https://user:pass@example.com/)` queued HEADERS with userinfo in `:authority`.
2. Fix: reject `@` in `:authority` after Host promotion in client `convert_send_message` and server `convert_push_message`.
3. Regression: `outbound_uri_userinfo_is_user_error` (F44 inbound already fixed).

## Next recommended step
1. Package PRs for F3–F45.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
