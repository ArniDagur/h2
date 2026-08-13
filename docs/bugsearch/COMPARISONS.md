# Comparisons (h2 vs nghttp2 / Go net/http2)

Cases where h2 matches reference implementations → **spec interpretation, not a fix-worthy bug**.

## Logged this fire
- None yet (no differential runs completed).

## Planned comparisons
- Connection window recovery thresholds (h2 uses 1/2 unclaimed ratio) vs nghttp2/Go auto window update strategies.
- Behavior when receiver drops interest mid-stream (RST_STREAM NO_ERROR / CANCEL) vs stream WINDOW_UPDATE suppression.
