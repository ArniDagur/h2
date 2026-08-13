# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `8e3d221`

## Current focus
F51: traditional CONNECT + Content-Length (request reject; 2xx response ignore).

## Last actions
1. Confirmed **F51**: successful CONNECT 2xx with Content-Length bound tunnel DATA (PROTOCOL_ERROR after advertised length); outbound/inbound traditional CONNECT accepted CL.
2. Fix: `Stream::is_connect`; skip CL bookkeeping on 2xx CONNECT responses; reject CL on traditional CONNECT send_request and server convert.
3. Regressions: `connect_response_content_length_is_ignored`, `send_connect_rejects_content_length`, `reject_connect_with_content_length`.

## Next recommended step
1. Package PRs for F3–F51.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
