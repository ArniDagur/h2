# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F98)

## Current focus
F98: DATA on reserved `pending_open` push was idle GOAWAY (F79 residual / F92 gap).

## Last actions
1. F79 GOAWAYs DATA on all `pending_open`. Server push waiting for a send slot is reserved at the peer (PP already sent), not idle — same split F92 used for WU/RST.
2. `recv_data` now GOAWAYs only `is_pending_open && !peer.is_server()`. Server path uses F23 `STREAM_CLOSED` + `ignore_data`.
3. Regression `data_on_pending_open_push_is_stream_closed_not_goaway`; client idle DATA still GOAWAYs.

## Next recommended step
1. Package PRs for F3–F98.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
