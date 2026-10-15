# ADR-011: skip is DB-authored, rendered (not notebook-parsed)

Date: 2026-08-09 · Status: Accepted

## Context
We need a way to mark a lesson (whole) or individual practice/quiz as *skipped* so
status derivation excludes it (e.g. a quiz the learner will not attempt). The obvious
places to author skip are (a) the DB, (b) the notebook. The notebook-first instinct
("a function at the top of the notebook that checks for skip") collides with
[adr/002](002-db-source-of-truth.md): carpenter does not parse notebooks back into
rows. Status derivation is Rust-side and DB-driven; reading skip out of a notebook
would require executing or parsing notebook code at derivation time.

## Decision
Skip is **DB-authored**. Three INTEGER columns: `lessons.skip`, `practice.skip`,
`quizzes.skip` (0/1). A single top-level command sets them:
```
skip --scope lesson|quiz|practice <id> [--off]
```
Top-level (not `lesson skip`/`quiz skip`) because the operation spans scopes and a
single command with `--scope` is fewer surfaces than three subcommands.

The notebook renders the skip set into a `managed=skip-config` cell at the top — a
read-only `_skip_config()` that reads the columns from `course.db`, so learner/agent
code can call `is_skipped("q3")`. It is **regenerated from the DB on sync** (DB is
authoritative); learner edits to it are not preserved (it is managed).

Status derivation: `lessons.skip=1` ⇒ lesson status `skipped`; non-skipped items gate
`complete`; skipped practice/quiz are excluded. See
[data-model/04](../data-model/04-status-derivation.md).

## Consequences
+ Pure adr/002 — skip flows one direction (DB → notebook render), never parsed back.
+ Derivation is a pure DB read; no subprocess/parse at status-compute time.
+ One command (with `--scope`) instead of three subcommands.
− The "function at the top of the notebook" the learner sees is a *mirror*, not the
  source — editing it directly does nothing (overwritten on sync). Mitigated by it
  being a read-only helper over the DB; the learner edits skip via the CLI.

*(Revision 2026-10-15: the rendered helper fn is `is_skipped(target)` (delegating
to the helper); the `_skip_config()` name in the decision text is historical.)*
