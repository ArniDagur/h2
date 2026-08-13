# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F74 @ `ca27aea`)

## Current focus
F74: END_STREAM + non-zero Content-Length RST after request EOS.

## Last actions
1. Confirmed **F74**: non-zero CL + END_STREAM validated after `recv_open`; with request EOS the stream fully closed and `send_reset` no-op'd (peer never saw RST). F68 only fixed 204.
2. Fix: validate parse/mismatch/non-zero CL on END_STREAM headers before `recv_open` (except 304; skip HEAD / CONNECT success).
3. Regression: `reject_none_zero_content_length_header_with_end_stream` now expects RST.

## Next recommended step
1. Package PRs for F3–F74.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt (empty IPv6 `[]` residual).

## Blockers
None.
