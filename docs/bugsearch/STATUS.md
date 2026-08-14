# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked poll_accept waker and SETTINGS ACK framing.

## Last actions
1. `poll_accept` drives via `poll_closed` (registers read/write/`actions.task`). `pending_accept` push happens in the same `poll2` as the HEADERS; `next_incoming` is checked after. Not a missed accept wake.
2. SETTINGS ACK with non-empty payload is `PROTOCOL_ERROR` (load maps all SETTINGS errors that way). RFC 9113 §6.5.1 is `FRAME_SIZE_ERROR`. Spec code mismatch, not hang/FC.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
