# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F75 @ `dcaec36`; tip `1523a9c`)

## Current focus
High-signal FC/hang hunt after F75. Deadlock stress test repaired.

## Last actions
1. Re-ran `logical_deadlock_max_concurrent_streams_stress` (#853): was failing every stream with **remote RST PROTOCOL_ERROR**, not a hang.
2. Root cause: F62 rejects scheme+path-only requests; test used `uri("/")` without Host/authority.
3. Fixed test to `http://localhost/`; stress passes 3× (~0.3s). #853 still looks fixed (no capacity-on-pending_open deadlock).

## Next recommended step
1. Package PRs for F3–F75.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup hunt: mid-response NO_ERROR+window0 (by design); full IPv6 structure; scan for new hangs.

## Blockers
None.
