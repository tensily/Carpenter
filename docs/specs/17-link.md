# `link` — output contracts (conceptual; future CLI registry)

Distinct from `register`/`deregister` (agent-app skill integration — see
[21-register-deregister.md](21-register-deregister.md)). `link` targets a future
external CLI registry (a single search point for the agent's tools); contract TBD.

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `register` | — | `{"name":"carpenter","version":"0.10.1","bin":"~/.local/bin/carpenter","summary":"Agent-driven CLI that builds Python/Jupyter learning material.","howto_excerpt":"Run `carpenter howto` for the full command manual.","commands":["course","lesson","plan","quiz","howto"]}` — emits a manifest for a future CLI registry; no filesystem effect |
<!-- END GENERATED -->
