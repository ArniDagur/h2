# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` @ `2274e2b`

## Current focus
Send-path flow-control re-queue / capacity hang hardening after FC audit.

## Last actions
1. Investigated S3 (`push_back_frame` / `pop_frame` when `available == 0`) and #853 capacity-starvation theories.
2. Clarified `assign_connection_capacity` skip of closed streams does **not** leak: capacity already returned to `flow.available`.
3. Hardened `pop_frame` + `push_back_frame` to re-queue into `pending_capacity` when `has_unavailable()` after capacity-0 deferral (addresses latent hang if stream falls off both queues).
4. Added regression test `connection_window_update_resumes_starved_buffered_stream` (passes with fix; documents intended recovery).

## Next recommended step
1. Stress-repro or dismiss **S1/#853** with concurrent `max_concurrent_streams` + large bodies (PR #852 style); note #860 may already fix.
2. Or add debug capacity-conservation asserts (sum of stream `available` + conn `available`).
3. Or mine Go/nghttp2 SETTINGS+WINDOW_UPDATE races and port a differential test.

## Blockers
None.
