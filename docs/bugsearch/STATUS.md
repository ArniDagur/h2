# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F35: uncapped 1xx informational HEADERS (memory DoS).

## Last actions
1. Confirmed **F35**: peer could flood 1xx into `pending_recv` without bound (Go caps at 5).
2. Fix: per-stream count; 6th 1xx → `RST_STREAM(ENHANCE_YOUR_CALM)`.
3. Regression: `too_many_informational_responses_is_stream_error`.

## Next recommended step
1. Package PRs for F3–F35.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup hunt.

## Blockers
None.
