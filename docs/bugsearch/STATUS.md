# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked SETTINGS IWS reclaim vs pending_open / pending_capacity (F6/F8).

## Last actions
1. SETTINGS IWS decrease: skip send-closed+unbuffered; reclaim `available > window`; `notify_capacity` if reclaimed>0 (F8); leftover goes to `assign_connection_capacity`. pending_open has `available==0` so no reclaim / I1 hold.
2. SETTINGS IWS increase: `recv_stream_window_update` → `try_assign` (skips pending_open/pending_push). Parked `poll_capacity` wakes via `assign_capacity` when usable capacity rises.
3. `reserve_capacity(0)` + `poll_capacity` stays Pending (requested=0, same as never-reserved). API footgun like `max_send_buffer_size(0)`, not a silent hang.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
