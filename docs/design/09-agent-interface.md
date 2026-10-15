# Agent interface

**Envelope** (one per command, stdout; error → exit 1):
```json
{"status":"ok","message":"...","data":{}}
{"status":"error","message":"...","code":"NotFound","details":{}}
```
Commands never raise to caller. `--spec` is read per dispatch arm via
`store::read_spec`, so a bad spec maps to a `ValidationError` envelope (not a
crash). `code` = error
variant name. List/show surface corrupt rows in `errors[]` (never silent skip).
Specs via `--spec <file>|-` (stdin). Reused shapes (`errors[]`, `updated:{…}`,
`conflicts[]`) and the `--force` policy live in
[specs/01-envelope.md](../specs/01-envelope.md).

**Plans human-in-the-loop:** `plan create` returns a draft (agent shows it in
opencode for user approval); `plan confirm <id>` persists.

**Feedback loops:** `notes add` echoes `related_open` (advisory); `progress summary`
rolls up lesson completion, live quiz state, and notes.

## Error handling
```
CarpenterError ⊂ { NotFound, AlreadyExists, ValidationError, StoreError, ExecuteError, Conflict }
```
`QuizRunError` was removed — `quiz run` failures surface as inline per-case fails in
`data` or as `StoreError` (no venv). See [specs/01-envelope.md](../specs/01-envelope.md).
