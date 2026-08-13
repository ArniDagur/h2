# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F93)

## Current focus
F93: cancel of a reserved push sitting in `pending_open` discarded locally and never sent RST.

## Last actions
1. F92 allowed peer WU/RST on reserved `pending_open`. Local cancel still used the idle abort.
2. `abort_closed_pending_open` dropped HEADERS and set_reset with no wire RST (RST on idle is PROTOCOL_ERROR — true only for unsent client requests).
3. After PP, the peer sees reserved; RFC §5.1 allows RST without a concurrency slot.
4. Fix: server `pending_open` abort queues RST (scheduled or explicit reason).
5. Regression: `drop_pending_open_push_sends_reset` (fails pre-fix: 2s, no RST).

## Next recommended step
1. Package PRs for F3–F93.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
