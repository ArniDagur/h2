# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F82)

## Current focus
No new library bug this fire.

## Last actions
1. Local `MAX_CONCURRENT_STREAMS` is applied at `Connection::new` (not ACK-gated). No F10-class race.
2. F81 `set_scheduled_reset` does not notify `recv_task`; waiters wake when RST is popped / `abort_closed_pending_open` calls `set_reset`.
3. Checked auto-release DATA burst after RecvStream drop: 3×16KiB stays inside the 64KiB window; a 4th 16KiB frame without WU is a real peer FC violation. `poll_complete` still sends WU when read goes idle.
4. Control-frame flood delaying WU (poll2 reads until Pending before `poll_complete`) is fairness/DoS under a saturated socket, not a spec-compliant hang.

## Next recommended step
1. Package PRs for F3–F82.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
