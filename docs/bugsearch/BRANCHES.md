# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream; do not pile experimental junk here. |
| `experimental/bugsearch` | Long-lived: all bugsearch fixes + notes/docs. |
| `fix/scheduled-reset-error-kind` | PR candidate: ScheduledLibraryReset → stream reset error (not GOAWAY). |

## Branch contents

### experimental/bugsearch
- docs under `docs/bugsearch/`
- F1 fix (`ensure_recv_open` error kind)

### fix/scheduled-reset-error-kind
- Same F1 code fix (for eventual PR); keep free of unrelated docs if upstream prefers minimal PR (docs can stay experimental-only).
