# Execution environment

How carpenter runs Python: a uv-managed course venv, notebook execution, and
quiz-run execution — all scoped to the course.

## Course venv (`uv`)
`venv create` sets up a **uv project** in the course dir so `uv run` resolves
deps for every execution path:

```
carpenter venv create [--python 3.12]
  → uv venv --python <ver>
  → write pyproject.toml (base deps — canonical list in specs/22: jupyterlab, nbconvert, nbclient, ipykernel)
  → uv sync            # creates .venv + uv.lock
```
- Produces `courses/<slug>/{pyproject.toml, uv.lock, .venv/}`.
- `StoreError` if `uv` is not on PATH; `AlreadyExists` if `.venv` is already present
  (re-run `venv sync` to update).
- `venv sync` → `uv sync`; `venv add <pkg>` → `uv add`; `venv list` → `uv pip list`.
- Contracts: [specs/22-venv.md](../specs/22-venv.md).

## `lesson execute` (force-run the notebook)
Re-runs a lesson's notebook end-to-end to capture any missed cell outputs:
```
uv run jupyter nbconvert --execute --inplace \
  [--allow-errors] --ExecutePreprocessor.timeout=<N> lessons/<NN>/lesson.ipynb
```
- **Strict by default** — aborts on the first errored cell → `ExecuteError`
  `{details:{index, ename, evalue}}`.
- `--allow-errors` — runs every cell, returns
  `{cells:{total, ran, errored}, errors:[…]}`.
- `--timeout <s>` per cell (default 30). The live timeout is this clap flag —
  `config`'s `timeout_secs` key is stored/reserved but not consulted by the
  execution paths.
- Persists outputs in place; nbconvert preserves cell metadata/tags → the next
  `lesson sync` still reconciles correctly.
- Requires the venv (`StoreError` "run `carpenter venv create` first" if absent).
- Contract: [specs/09-lesson.md](../specs/09-lesson.md).

## Quiz run uses the venv
`quiz run` executes the lesson notebook via `uv run jupyter nbconvert --execute` in
the course venv (with `--allow_errors`). nbconvert's kernel is the isolation
boundary — there is no separate custom subprocess child. The in-notebook helper
cells do the scoring and write `pass_or_fail`/`last_check`. Full model in
[08-quiz-run.md](08-quiz-run.md). `helper.py` runs in the same kernel (stdlib only,
but it does write back to `course.db` — adr/010).

## Why uv
One tool for venv + deps + lock; `uv run` gives every execution path a consistent,
reproducible environment without carpenter reinventing activation. carpenter never
reimplements package management (ponytail: reuse the crate/tool).
