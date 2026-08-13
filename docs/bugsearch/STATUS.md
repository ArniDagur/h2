# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (also `fix/scheduled-reset-error-kind`)

## Current focus
Error classification + flow-control / concurrency hang hunting after recent upstream FC/wakeup fixes.

## Last actions
1. Bootstrapped `docs/bugsearch/*` handoff docs (repo had none).
2. Audited recent master fixes (#893, #894, #895, #896, #897, #898, #930, #931) and open issues (#853, #878, #880, #882).
3. Confirmed #880 largely fixed by #896; #878 fixed by #893; #853 may be fixed by #860 (still open upstream, no confirmation).
4. Found and fixed: `State::ensure_recv_open` reported `ScheduledLibraryReset` as **connection GOAWAY** instead of **stream RST** (`library_go_away` → `library_reset`).

## Next recommended step
1. Try to reproduce #853 (connection capacity deadlock under `max_concurrent_streams` pressure) with a stress/mock test on current master+fix; close-out or re-open as confirmed.
2. Or instrument `Prioritize` with debug asserts: `sum(stream.send_flow.available) + conn.available` conservation; panic on orphan capacity when streams leave `pending_capacity` closed.
3. Or mine Go `x/net/http2` / nghttp2 FC bugs and map to h2 code paths (see IDEAS.md).

## Blockers
None.
