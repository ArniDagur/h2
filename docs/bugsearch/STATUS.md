# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F22: outbound Host conflicting with `:authority` (#876).

## Last actions
1. Confirmed **F22** (#876): user `Host` was emitted as a regular header while URI-derived `:authority` stayed set — can disagree on the wire (RFC 9113 §8.3.1).
2. Fix: `Pseudo::promote_host_header` — promote Host → `:authority`, strip Host; used by client requests and server PUSH_PROMISE. HTTP/1.x-version requests still default `:scheme` to `http` after promotion (relative+Host).
3. Regression: `host_header_promoted_to_authority_and_stripped`.

## Next recommended step
1. Package PRs for F3–F22.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
