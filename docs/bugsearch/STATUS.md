# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F101)

## Current focus
No new hang/FC. Rechecked F101 siblings on recv polls.

## Last actions
1. `poll_data` only parks via `schedule_recv` (empty queue) which already surfaces RST/GOAWAY. Non-data head is trailers → `Ready(None)` (body done), not a hang.
2. `poll_informational` / `poll_response` / `poll_pushed` park only on empty + `ensure_recv_open`; RST `notify_*` then `Err`.
3. `Closed(ErrorAfterEndStream)` + DATA still queued: `poll_trailers` stays Pending (`Ok(false)`). Same drain-then-trailers API as clean DATA+EOS; late RST after recv EOS is not a recv error.

## Next recommended step
1. Package PRs for F3–F101.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
