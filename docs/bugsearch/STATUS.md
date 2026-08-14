# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F103)

## Current focus
F103: parent reset did not RST `pending_open` push after `send_response`.

## Last actions
1. F97 only matched `ReservedLocal`. `send_response` already `send_open`s, so a child waiting on a send slot was skipped.
2. With max concurrent 1, PP(4) advertised + HEADERS queued: parent CANCEL left the client push future hanging while stream 2 held the slot.
3. Also RST `is_pending_open` children (F93 abort emits RST without a slot).

## Next recommended step
1. Package PRs for F3–F103.
2. Residual #848 API ready-at-max-open.
3. Optional: reject `push_request` after remote GOAWAY / cap promised id.
4. Next search: fuzz vs Go/nghttp2 or other hang/FC.

## Blockers
None.
