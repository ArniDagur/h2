# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F94)

## Current focus
F94: `send_response` then cancel before PP flush still emitted HEADERS (opened over max concurrent).

## Last actions
1. F18 RST'd a `pending_push` cancel after PP, but only when no HEADERS were queued.
2. `send_response` queues HEADERS; drop schedules RESET while still `pending_push`.
3. PP pop `pending_send.push`'d the child without clearing HEADERS and without `inc_num_send_streams`.
4. Fix: on scheduled-reset PP pop, `clear_queue` then RST only.
5. Regression: `drop_push_after_response_before_pp_flush_sends_reset_not_headers`.

## Next recommended step
1. Package PRs for F3–F94.
2. Residual #848 API ready-at-max-open.
3. Optional test hygiene: S4 stale tests + F29 drain loop.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
