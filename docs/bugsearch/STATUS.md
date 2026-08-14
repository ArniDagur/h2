# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F104)

## Current focus
F104: `push_request` after remote GOAWAY still reserved a promised stream.

## Last actions
1. Client GOAWAY(last=1) leaves `send.max_stream_id=1`; `push_request` still queued PP(1,2).
2. Peer ignores stream 2 (no WU/RST) → server `send_data` / client push future stall.
3. Reject when `next_promised_id > send.max_stream_id` (before reserve). GOAWAY(MAX) still allows push.

## Next recommended step
1. Package PRs for F3–F104.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
