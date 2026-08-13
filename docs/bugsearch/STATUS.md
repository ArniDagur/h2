# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F84)

## Current focus
F84 — malformed header in a CONTINUATION-spanning block.

## Last actions
1. `HeaderBlock::load` dropped stream-level `malformed` on `NeedMore`; `framed_read` RST'd before END_HEADERS.
2. Persist `is_malformed`; keep decoding CONTINUATION until END_HEADERS, then RST.
3. Unit + regression: `malformed_connection_header_persists_across_need_more`, `recv_connection_header_spanning_continuation_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F84.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
