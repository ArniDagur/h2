# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F14: RecvStream drop / `!is_recv` ignored DATA only restored connection FC.

## Last actions
1. Confirmed **F14**: on `RecvStream` drop and post-drop ignored DATA, only connection capacity was re-credited — stream window was not consumed/released, so no stream WINDOW_UPDATE and over-window DATA could be accepted.
2. Fix: `clear_recv_buffer` and `!is_recv` DATA path use `release_capacity` (conn + stream).
3. Tests: unit `ignored_data_when_not_recv_consumes_stream_window`; integration `drop_recv_stream_releases_stream_window_update`; updated drop capacity test.

## Next recommended step
1. Package PRs for F3–F14.
2. Or residual #848 API design.
3. Or connection window recovery threshold vs Go/nghttp2.

## Blockers
None.
