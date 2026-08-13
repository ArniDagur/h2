# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (latest)

## Current focus
F21: non-CONNECT request with authority but no `:scheme` was accepted.

## Last actions
1. Confirmed **F21**: `client::Peer::convert_send_message` had a `// TODO: Error` for authority-without-scheme non-CONNECT URIs (`example.com:8080`); HEADERS were emitted without `:scheme` (RFC 9113 §8.3.1).
2. Fix: return `MissingUriSchemeAndAuthority`; same check on server `convert_push_message`; convert before `open()` so bad requests do not burn stream ids.
3. Regression: `request_with_authority_without_scheme_is_user_error`.

## Next recommended step
1. Package PRs for F3–F21.
2. Or residual #848 / #30 pending_accept design.
3. Or reserved-stream concurrency cap TODO.

## Blockers
None.
