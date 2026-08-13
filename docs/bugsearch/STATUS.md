# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F17: `SendRequest::pending` OpaqueStreamRef blocked cancel of pending_open.

## Last actions
1. Confirmed **F17**: when occupancy was full, `send_request` stored `pending: Some(OpaqueStreamRef)`, keeping `ref_count > 0` after the user dropped `ResponseFuture` + `SendStream`. Implicit CANCEL never ran; HEADERS were still sent when a slot opened.
2. Fix: store `pending: Option<StreamId>` (no ref); `poll_pending_open` / Rejected look up by id (missing stream ⇒ ready).
3. Regression: `drop_stream_handles_cancels_despite_sendrequest_pending`.

## Next recommended step
1. Package PRs for F3–F17.
2. Or residual #848 API design.
3. Or connection window recovery threshold vs Go/nghttp2 (COMPARISONS).

## Blockers
None.
