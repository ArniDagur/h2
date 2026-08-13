# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F75 @ `dcaec36`)

## Current focus
F75: empty IPv6 literal authority `[]` rejected (F66 residual).

## Last actions
1. Confirmed **F75**: `http::uri::Authority` accepts `"[]"` / `"[]:80"` with `host() == "[]"`; F66 only checked `host().is_empty()`.
2. Fix: `is_empty_or_empty_ip_literal_host` on server recv, Host-only, client send, push convert.
3. Regressions: `reject_request_empty_ipv6_literal_authority`, `request_with_empty_ipv6_literal_authority_is_user_error`.

## Next recommended step
1. Package PRs for F3–F75.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (full IPv6 structural validation not done — only empty `[]`).

## Blockers
None.
