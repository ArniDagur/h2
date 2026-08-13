# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F88)

## Current focus
F88: `poll_reset` missed wakeup on clean recv EOS (F31 residual).

## Last actions
1. Recv EOS (HEADERS/DATA/trailers) and local send EOS that fully close the stream now `notify_send`.
2. Regression `poll_reset_woken_when_recv_eos_closes_stream` (park first, then peer EOS).

## Next recommended step
1. Package PRs for F3–F88.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or I1/I2, not more header-name nits.

## Blockers
None.
