# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F87)

## Current focus
F87 — empty header field name GOAWAY.

## Last actions
1. `Header::new("")` returned HPACK `NeedMore` (complete zero-length name).
2. Map empty name to `Header::Malformed` (stream RST, same as F86).
3. Unit + regression: `empty_header_name_is_malformed_not_need_more`, `recv_empty_header_name_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F87.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
