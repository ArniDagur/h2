# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F102)

## Current focus
F102: nested PUSH_PROMISE on reserved (remote) was accepted (RFC MUST GOAWAY).

## Last actions
1. Confirmed `PP(1,2)` then `PP(2,4)` reserved stream 4; mock waited forever for GOAWAY (pre-fix hang).
2. `recv_push_promise` now GOAWAYs if parent is `pending_open` or not locally initiated.
3. Regression `recv_push_promise_on_reserved_remote_is_conn_error`; existing valid PP tests still pass.

## Next recommended step
1. Package PRs for F3–F102.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
