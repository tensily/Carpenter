# `skip` — output contracts

Top-level command (spans scopes via `--scope`, not nested under lesson/quiz —
adr/011). Sets the `skip` column read by status derivation
([data-model/04](../data-model/04-status-derivation.md)) and rendered into the
notebook's `managed=skip-config` cell (`_skip_config()`, read-only, reads
`course.db`).

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `skip --scope lesson\|quiz\|practice <id>` | id + scope | `{"scope":"quiz","id":"q1","skip":true}` — `NotFound` if the id does not exist under the scope |
| `skip --scope lesson\|quiz\|practice <id> --off` | id + scope | `{"scope":"quiz","id":"q1","skip":false}` — clears the flag |
<!-- END GENERATED -->

- `NotFound` if the id does not exist under the given scope.
- Effect on status: a skipped item is excluded from its lesson's `complete`
  derivation; `lessons.skip=1` forces lesson status `skipped`.
- A skip change does **not** re-execute the notebook; the rendered `_skip_config`
  cell reflects the new state on the next `lesson sync`. `quiz run`/`results`
  continue to score skipped quizzes (their `last_check` is still written) but their
  `pass_or_fail` is ignored for derivation.
