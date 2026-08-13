# Bugsearch status

**Updated:** 2026-08-13  
**Branch tip:** `experimental/bugsearch` (F60 @ `efc9ff5`)

## Current focus
F60: non-OPTIONS `:path` = `*` (asterisk-form).

## Last actions
1. Confirmed **F60**: asterisk-form is OPTIONS-only (RFC 9110 §7.1); nghttp2 enforces same for http/https. h2 accepted GET `*` via PathAndQuery.
2. Fix: reject `*` path unless OPTIONS on server recv, client send, and push convert.
3. Regressions: `reject_asterisk_path_for_non_options`, `request_asterisk_path_non_options_is_user_error`.
4. Note: `:path` starting with `//` accepted by nghttp2 (starts with `/`) → not fix-worthy vs reference.

## Next recommended step
1. Package PRs for F3–F60.
2. Residual #848 API ready-at-max-open.
3. Further FC/wakeup / protocol hunt.

## Blockers
None.
