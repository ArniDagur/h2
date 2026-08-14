# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F97)

## Current focus
No new hang/FC this fire. Checked F97 residuals + client parent-RST cancel.

## Last actions
1. Client auto-error of `ReservedRemote` children on parent RST is **wrong**: HEADERS may still be in flight after parent RST (queue order: parent RST then child HEADERS).
2. F97 `Send::send_reset` skips `enqueue_reset_expiration`; forgotten-stream path still ignores late WU/RST and STREAM_CLOSED for late HEADERS/DATA.
3. `schedule_implicit_reset` on F97 children still relies on RST pop `set_reset` to wake `poll_reset` (spawned connection). Same class as other implicit RST.

## Next recommended step
1. Package PRs for F3–F97.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` / drop queued PP when `promised_id > send.max_stream_id`.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
