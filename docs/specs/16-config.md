# `config` — output contracts

App-level config (`~/.config/carpenter/config.json`). Valid keys (unknown key ⇒
`ValidationError`):

| key | type | default | rule |
|-----|------|---------|------|
| `bin_dir` | string | `~/.local/bin` | where `install` places the binary |
| `python` | string | uv's default | passed to `venv create` when `--python` omitted |
| `timeout_secs` | int | `30` | stored/reserved key — round-trips through `get`/`set`; live per-command execution timeouts come from the clap `--timeout` flag (default 30s), not this value |
| `active_course` | string? | — | current course slug (set by `course switch`) |
| `source_dir` | string? | — | carpenter source checkout (used by `upgrade` to resolve `--source`) |

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `get` | — | `{"bin_dir":"~/.local/bin","python":null,"timeout_secs":30,"active_course":null,"source_dir":null}` — all keys with defaults applied; optionals `null` when unset |
| `get <key>` | key | `{"key":"timeout_secs","value":30}` — unknown key ⇒ `ValidationError` |
| `set <key> <value>` | key + value | `{"key":"timeout_secs","value":45}` — value coerced to the key's type (`timeout_secs`⇒int); unknown key ⇒ `ValidationError` |
<!-- END GENERATED -->
