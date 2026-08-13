# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest commit)

## Current focus
Confirm or dismiss open issue #853 (connection capacity deadlock under concurrency).

## Last actions
1. Ported PR #852–style stress test → `tests/h2-tests/tests/deadlock.rs` (`logical_deadlock_max_concurrent_streams_stress`).
2. Ran test 5× successfully (50 concurrent tasks × 40 reqs, server `max_concurrent_streams=10`).
3. Conclusion: **S1/#853 does not reproduce** on current code with correct FC release + `SendRequest::ready()`. Pre-#860 theory (capacity to `pending_open`) is blocked by #860; #930 also helps by releasing conn capacity on `RecvStream` drop.
4. Original #852 test omitted `release_capacity` / `ready()` — may have been partly a test-harness FC stall.

## Next recommended step
1. Add debug capacity-conservation asserts (sum stream send `available` + conn `available`) on experimental.
2. Or investigate S2 sticky `poll_data` after reset / shared `send_task` wakeups.
3. Or SETTINGS_INITIAL_WINDOW_SIZE decrease + in-flight streams differential vs Go/nghttp2.

## Blockers
None.
