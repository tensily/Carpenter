# Envelope & conventions

Every command prints exactly **one** JSON envelope on stdout and exits 0 (ok) or 1
(error). The `data` shapes are what an agent parses.

```
ok:     {"status":"ok",    "message":"…", "data":{ … }}
error:  {"status":"error", "message":"…", "code":"NotFound", "details":{ … }}   // exit 1
```

## Error codes
`code` = the `CarpenterError` variant:
`NotFound | AlreadyExists | ValidationError | StoreError | ExecuteError | Conflict`.

- `NotFound` — requested id/slug does not exist.
- `AlreadyExists` — create would collide (e.g. duplicate slug, `.venv` present).
- `ValidationError` — bad `--spec` YAML, unknown enum value, failed cross-field
  validation, or an unsupported flag value (e.g. `upgrade --channel edge`).
- `StoreError` — SQLite failure, missing file (e.g. no course venv for `quiz run`),
  or `uv`/`jupyter` not on PATH.
- `ExecuteError` — a managed/scaffolding cell errored during execution (see
  [09-lesson.md](09-lesson.md) `execute`). `details:{index,ename,evalue}`. Raised
  when scaffolding (carpenter-generated code) is broken; the agent must rewrite the
  section. Learner errors are **not** `ExecuteError` — they are scored as fails.
- `Conflict` — a destructive op was attempted without `--force` (see below), or a
  sync found unresolvable stub conflicts.

`QuizRunError` was removed: `quiz run` failures surface as inline per-case fails in
`data` (not as an error envelope) or as `StoreError` (no venv).

## Reused shapes
- **`errors[]`** (corrupt-row list, on `list`/`show` commands): each item is
  `{id?, reason}` where `reason ∈ {corrupt_json, missing_fk, …}`. The row is
  surfaced, never silently dropped.
- **`updated:{…}`** (echoed on `update` commands): the **full new row** as stored
  after the update — unambiguous before/after diff is the caller's job.
- **`conflicts[]`** (on `lesson sync`): `{id, reason}` where `id` is the practice
  or quiz id and `reason` is currently always `db_changed` (see
  [design/05](../design/05-notebook-sync.md)).

## Policies
- **`--spec`** accepts a file path or `-` for stdin; YAML is validated up front
  inside the envelope path, so a bad spec returns a `ValidationError` envelope
  (not a crash).
- **Corrupt rows** in aggregate `list` commands return `ok` with an `errors[]`
  field rather than dropping them silently (`show <one id>` does not emit `errors[]`).
- **`--force`** on destructive ops (delete, overwrite-update, sync-overwrite):
  omitting `--force` returns a `Conflict` envelope — **never** a TTY prompt (an
  agent cannot answer an interactive prompt). Pass `--force` to proceed.

## Global flags
`--version`, `--root <path>`, `--course/-c <slug>` (defaults to `active_course` in
`~/.config/carpenter/config.json`).
