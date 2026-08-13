# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F86)

## Current focus
F86 — uppercase / invalid header names GOAWAY.

## Last actions
1. `Header::new` mapped `from_lowercase` / invalid value / unknown pseudo to HPACK `DecoderError` → GOAWAY.
2. `Header::Malformed` continues HPACK; `HeaderBlock` marks stream-malformed → RST (F84/F85 path).
3. Unit + regression: `uppercase_header_name_is_malformed_not_hpack_error`, `recv_uppercase_header_name_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F86.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
