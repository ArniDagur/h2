# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F32: pseudo-header fields in received trailers accepted as OK.

## Last actions
1. Confirmed **F32**: trailers with `:status` were decoded into `Pseudo` then dropped by `into_fields()` with no stream error (RFC 9113 §8.1).
2. Fix: reject non-empty Pseudo (and CL mismatch) before `recv_close` so RST is emitted; validation-after-close made `send_reset` a no-op.
3. Regression: `recv_trailers_with_pseudo_header_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F32.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup hunt.

## Blockers
None.
