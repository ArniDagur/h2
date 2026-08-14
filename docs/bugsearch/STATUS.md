# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F97)

## Current focus
No new hang/FC this fire. Re-checked `has_streams` idle close and recv PP vs our GOAWAY.

## Last actions
1. `has_streams()` still omits `num_pending_open`, but `Connection::poll` runs `poll_complete` before the idle/`error` GOAWAY. Remaining `pending_open` implies `!can_inc` ⇒ open send streams ⇒ `has_streams` true. Reconfirmed, not a hang.
2. `recv_push_promise` checks parent id vs `recv.max_stream_id`, not `promised_id`. HEADERS on the promised id would be ignored (`id > max`). Client has no graceful-GOAWAY-while-parent-open API, so PP cannot arrive after our GOAWAY with a live parent. Unreachable hang; optional check only.
3. DATA/HEADERS/PP stream 0 already rejected at frame load.

## Next recommended step
1. Package PRs for F3–F97.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` / drop queued PP when `promised_id > send.max_stream_id`; optionally ignore PP when `promised_id > recv.max_stream_id`.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
