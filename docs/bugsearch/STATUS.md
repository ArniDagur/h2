# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F81)

## Current focus
F81: last SendStream drop without EOS sends RST even if recv handles live.

## Last actions
1. Confirmed **F81**: docs promise RST on `SendStream` drop without closing send; cancel waited for `ref_count == 0`.
2. Holding `ResponseFuture` after drop left send half open — peer waits, client hangs.
3. Fix: last `send_ref` drop with send half open → `schedule_implicit_reset(CANCEL)`.
4. Regression: `drop_send_stream_without_eos_resets_despite_response_future`. F77/F78 still pass.

## Next recommended step
1. Package PRs for F3–F81.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
