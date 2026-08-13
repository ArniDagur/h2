# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F83)

## Current focus
F83 — mid-connection ENABLE_CONNECT_PROTOCOL applied only on SETTINGS_ACK.

## Last actions
1. `Connection::enable_connect_protocol` queued SETTINGS; Recv rejected `:protocol` until ACK.
2. Apply enable when SETTINGS is written (ACK remains idempotent). Builder handshake path already set the flag at `Recv::new`.
3. Regression: `enable_connect_protocol_before_settings_ack`.

## Next recommended step
1. Package PRs for F3–F83.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
