# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F99)

## Current focus
F99: 1xx on reserved-remote push recounted `num_recv_streams`.

## Last actions
1. `recv_open` kept `ReservedRemote` for 1xx, so each later HEADERS looked `initial` and `inc_num_recv_streams` again (debug panic / slot leak).
2. First 1xx now opens `HalfClosedLocal(AwaitingHeaders)` (RFC §5.1).
3. Regression `recv_informational_on_reserved_push_then_final`.

## Next recommended step
1. Package PRs for F3–F99.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC. Oversize-after-EOS RST still optional F74-class.

## Blockers
None.
