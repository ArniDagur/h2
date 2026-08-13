# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
Post-F17 hunt: no new conclusive bug this fire.

## Last actions
1. Reviewed cancellation/ref paths after F17, GOAWAY vs `pending_open` (reset + empty queue aborts via F16 scan; `ensure_no_conn_error` unblocks `poll_ready`).
2. Tracker differential: Go #80035 (SETTINGS window overflow → FLOW_CONTROL_ERROR) — h2 already rejects via `inc_window` + existing overflow tests; not a bug.
3. #882 residual `is_end_stream()==false` after reset error delivered — intentional with #810 / F4 test asserts `!is_end_stream()` after error; sticky poll fixed by F4 only.

## Next recommended step
1. Package PRs for F3–F17 (highest leverage).
2. Or residual #848 API design.
3. Or new hunt: server `pending_accept` + remote reset (#30), reserved-stream concurrency cap TODO.

## Blockers
None.
