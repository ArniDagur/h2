# Ideas backlog

## Tried
- Review recent master FC/wakeup commits (#893–#898, #930–#931).
- Map open GitHub issues #853, #878, #880, #882 to current code.
- Code read of `prioritize.rs` pop_frame capacity-0 / reclaim paths.
- Fix error classification for `ScheduledLibraryReset` (F1).
- Analyze `assign_connection_capacity` closed-stream skip (not a leak).
- Harden capacity-0 requeue into `pending_capacity` (F2) + regression test.
- Stress-repro #853 (PR #852 style) → does not hang post-#860; dismiss S1 with caveats.

## High priority next
1. **Capacity conservation instrumentation** — debug_assert sum of stream send `available` + conn `available`.
2. **Differential fuzz** vs Go http2 frame sequences (WINDOW_UPDATE / SETTINGS races).
3. **Port Go/nghttp2 regression tests** for SETTINGS_INITIAL_WINDOW_SIZE decrease with in-flight streams.
4. **S2 sticky poll_data** after reset — decide if API change is worth it.

## Lower priority
- Shared `send_task` for `poll_capacity` / `poll_reset` / `poll_pending_open`.
- `unclaimed_capacity` threshold edge cases with negative windows after SETTINGS.
- `dec_send_window` TODO about underflow (partially covered by `window_size_does_not_underflow`).
- Comment on upstream #853 that stress no longer reproduces (optional community note).
