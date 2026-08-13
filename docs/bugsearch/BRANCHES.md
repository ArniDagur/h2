# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream; no experimental junk. |
| `experimental/bugsearch` | Long-lived: all bugsearch fixes + docs. |
| `fix/scheduled-reset-error-kind` | F1: ScheduledLibraryReset → stream reset error. |
| `fix/pending-capacity-requeue-on-zero` | F2: re-queue to pending_capacity on capacity 0. |

## Branch contents

### experimental/bugsearch
- `docs/bugsearch/*`
- F1 + F2 fixes

### fix/scheduled-reset-error-kind
- F1 only (may share docs commit history with experimental; squash/cherry-pick for PR)

### fix/pending-capacity-requeue-on-zero
- F2: `prioritize.rs` requeue + `flow_control.rs` regression test
