# Bugsearch status

**Updated:** 2026-08-14  
**Branch tip:** `experimental/bugsearch` (F108)

## Current focus
No new hang/FC. Last-DATA+EOS drop vs codec reclaim (F106-class) is safe.

## Last actions
1. `transition_after` unlinks/removes on last DATA+EOS when `ref_count==0`.
2. `reclaim_frame` only `resolve`s when `last_data_frame` still has payload. Codec sets that slot only after the frame is fully written (`remaining==0`), so resolve is a no-op. Large DATA sits in `encoder.next` until then.
3. Drop-handles + 16KiB DATA+EOS + write backpressure flushed without panic.

## Next recommended step
1. Package PRs for F3–F108.
2. Residual #848 API ready-at-max-open.
3. Next search: fuzz vs Go/nghttp2 or other hang/FC (not F107/F108 HOL or this reclaim path).

## Blockers
None.
