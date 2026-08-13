# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F72 @ `ec82534`)

## Current focus
F72: multiple Host header fields.

## Last actions
1. Confirmed **F72**: RFC 9110 §7.2 / nghttp2 reject multiple Host; pre-fix accepted multiples and only compared the first to `:authority`.
2. Fix: reject multi-Host on server recv, client/push generate (before promote), and `Send::check_headers`.
3. Regressions: `reject_multiple_host_headers`, `send_request_rejects_multiple_host_headers`.

## Next recommended step
1. Package PRs for F3–F72.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` residual).

## Blockers
None.
