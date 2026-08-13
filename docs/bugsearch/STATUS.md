# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F42: inbound request `Host` differing from `:authority` accepted.

## Last actions
1. Confirmed **F42**: GET with `:authority: example.com` + `Host: evil.example` delivered to `poll_accept`.
2. Fix: `server::Peer::convert_poll_message` rejects Host ≠ `:authority` (PROTOCOL_ERROR); matching Host still OK.
3. Regressions: `reject_host_header_differing_from_authority`, `matching_host_with_authority_is_accepted`.

## Next recommended step
1. Package PRs for F3–F42.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
