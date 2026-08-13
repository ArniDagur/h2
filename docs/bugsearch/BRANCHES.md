# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream; no experimental junk. |
| `experimental/bugsearch` | Long-lived: all bugsearch fixes + docs + stress tests. |
| `fix/scheduled-reset-error-kind` | F1: ScheduledLibraryReset → stream reset error. |
| `fix/pending-capacity-requeue-on-zero` | F2: re-queue to pending_capacity on capacity 0. |

## Branch contents

### experimental/bugsearch
- `docs/bugsearch/*`
- F1 + F2 fixes
- `tests/h2-tests/tests/deadlock.rs` (#853 stress; not a code fix)

### fix/scheduled-reset-error-kind
- F1 only

### fix/pending-capacity-requeue-on-zero
- F2: `prioritize.rs` requeue + flow_control regression test
