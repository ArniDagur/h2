# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F105)

## Current focus
F105: oversize trailers skipped the F100 `is_over_size` check.

## Last actions
1. Trailer HEADERS use `recv_trailers`, not `recv_headers`/`recv_open`.
2. After request EOS, `recv_close` fully closed the stream; oversize trailers were delivered and RST would no-op.
3. Reject `is_over_size` before `recv_close`. Valid trailers tests still pass.

## Next recommended step
1. Package PRs for F3–F105.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
