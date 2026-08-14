# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F96)

## Current focus
F96: queued PUSH_PROMISE was still written after the client disabled push.

## Last actions
1. `poll2` applies SETTINGS then `poll_complete` writes pending frames.
2. `ENABLE_PUSH=0` only cleared `Send::is_push_enabled`; already-queued PP still flushed.
3. RFC 9113 §8.4: that PP is a connection PROTOCOL_ERROR at the client.
4. Fix: drop PP at pop and locally cancel the never-sent child.
5. Regression: `queued_push_promise_not_sent_after_enable_push_zero`.

## Next recommended step
1. Package PRs for F3–F96.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
