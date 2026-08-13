# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F85)

## Current focus
F85 — malformed PUSH_PROMISE HPACK RSTs the parent.

## Last actions
1. Codec `MalformedMessage` RST used the PP parent stream id.
2. RST the promised id (0 → GOAWAY); parent request continues.
3. Regressions: `recv_push_promise_connection_header_resets_promised_not_parent`, `recv_push_promise_connection_header_spanning_continuation`.

## Next recommended step
1. Package PRs for F3–F85.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.

## Blockers
None.
