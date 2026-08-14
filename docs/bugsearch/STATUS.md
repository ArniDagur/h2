# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F106)

## Current focus
No new hang/FC. Rechecked I1 send-capacity conservation and EOS reclaim.

## Last actions
1. I1 sums `ids` only (I2 walks slab). Unlinked-but-live streams with leftover `available` would hide from I1; `is_closed` requires empty send queue, and `send_data`/`send_trailers` EOS plus F77 reclaim reserved capacity. `send_headers` EOS does not `reserve_capacity(0)`, but that path is first HEADERS on a new stream (`available == 0`).
2. Codec `shutdown` flushes before `poll_shutdown`. `poll_complete` CodecFull parks on the write waker.

## Next recommended step
1. Package PRs for F3–F106.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
