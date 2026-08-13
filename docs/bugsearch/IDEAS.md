# Ideas backlog

## Tried
- Review recent master FC/wakeup commits (#893–#898, #930–#931).
- Map open GitHub issues #853, #878, #880, #882 to current code.
- Code read of `prioritize.rs` pop_frame capacity-0 / reclaim paths.
- Fix error classification for `ScheduledLibraryReset`.

## High priority next
1. **Stress-repro #853** — concurrent POSTs, low `max_concurrent_streams`, large bodies (PR #852 test).
2. **Capacity conservation instrumentation** — debug_assert total assigned stream send capacity + conn available equals theoretical window accounting; panic on orphan.
3. **pending_capacity eviction without reclaim** — when closed streams are `continue`d in `assign_connection_capacity`, ensure any leftover `send_flow.available` is reclaimed.
4. **Differential fuzz** vs Go http2 frame sequences (WINDOW_UPDATE / SETTINGS races).
5. **Port Go/nghttp2 regression tests** for SETTINGS_INITIAL_WINDOW_SIZE decrease with in-flight streams.

## Lower priority
- Shared `send_task` for `poll_capacity` / `poll_reset` / `poll_pending_open` (usually same task waker; multi-task misuse rare).
- Sticky `poll_data` errors after reset (API design).
- `unclaimed_capacity` threshold edge cases with negative windows after SETTINGS.
