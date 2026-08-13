# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F37: server accepts request HEADERS missing `:path` (non-CONNECT) or CONNECT without `:authority`.

## Last actions
1. Confirmed **F37**: scheme-only GET (no `:path`) and CONNECT without `:authority` were delivered to `poll_accept`.
2. Fix in `server::Peer::convert_poll_message`: require `:path` for non-CONNECT / extended CONNECT; require `:authority` for all CONNECT.
3. Regressions: `reject_request_missing_path_pseudo`, `reject_connect_missing_authority_pseudo`.

## Next recommended step
1. Package PRs for F3–F37.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (e.g. trailers, PRIORITY).

## Blockers
None.
