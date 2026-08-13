# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream; no experimental junk. |
| `experimental/bugsearch` | Long-lived: fixes + docs + stress tests + debug instrumentation. |
| `fix/scheduled-reset-error-kind` | F1: ScheduledLibraryReset → stream reset error. |
| `fix/pending-capacity-requeue-on-zero` | F2: re-queue to pending_capacity on capacity 0. |

## Branch contents

### experimental/bugsearch
- `docs/bugsearch/*`
- F1 + F2 fixes
- `tests/h2-tests/tests/deadlock.rs` (#853 stress)
- Debug send-capacity conservation asserts (I1)

### fix/* branches
- PR candidates without research docs when possible (history may include experimental docs commits; cherry-pick/squash for clean PRs).
