# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F101)

## Current focus
F101: `poll_trailers` re-parked on leftover DATA after RST.

## Last actions
1. `poll_trailers` treated any non-trailer queue head as Pending. RST woke the waiter, then the next poll parked again with no further notify.
2. Stream error is now delivered instead of re-parking. DATA+EOS without RST still requires drain-then-trailers (existing API).
3. Regression `poll_trailers_after_reset_with_buffered_data_does_not_hang`.

## Next recommended step
1. Package PRs for F3–F101.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
