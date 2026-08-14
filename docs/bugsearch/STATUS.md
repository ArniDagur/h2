# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F105)

## Current focus
No new hang/FC. Rechecked F105 siblings and reserved-state WU / in-flight SETTINGS.

## Last actions
1. Oversize 1xx / request HEADERS already use `recv_headers` `is_over_size` (F100). Oversize PP already RST's promised id.
2. WINDOW_UPDATE on reserved (remote): `is_send_closed` → recv_stream_window_update no-ops (no inc). RFC §5.1 MUST conn PROTOCOL_ERROR; same leniency class as reserved DATA → STREAM_CLOSED (Go). Not hang/FC.
3. SETTINGS IWS decrease while a large DATA is in-flight: `push_back_frame` may leave the remainder off send/capacity queues when stream window is 0. Recovery is stream WU / later SETTINGS increase via `try_assign` (same wait-for-WU as F30).

## Next recommended step
1. Package PRs for F3–F105.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
