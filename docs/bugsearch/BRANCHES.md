# Branches

| Branch | Purpose |
|--------|---------|
| `master` | Clean upstream |
| `experimental/bugsearch` | All fixes + docs + instrumentation + stress tests |
| `fix/scheduled-reset-error-kind` | F1 |
| `fix/pending-capacity-requeue-on-zero` | F2 |
| `fix/recv-data-error-releases-conn-capacity` | F3 |

## experimental/bugsearch contains
- F1, F2, F3
- I1 send conservation, I2 recv in-flight conservation
- `tests/h2-tests/tests/deadlock.rs`
- `docs/bugsearch/*`
