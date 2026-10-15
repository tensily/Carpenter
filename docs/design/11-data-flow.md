# Data flow

1. `course create --spec -` → `course.json` + empty `course.db`.
2. `plan create` (course) → draft; user approves → `plan confirm` → goals rows +
   `covered_by` links resolved.
3. `lesson create --spec -` → db inserts; render notebook (skip-config cell +
   helper) + `helper.py`.
4. Learner fills practice stub, runs check cell → helper scores + writes
   `practice.pass_or_fail`/`last_check` (instant feedback).
5. Learner fills quiz stub; `quiz run` → nbconvert executes the notebook → helper
   cells write `quizzes.pass_or_fail`/`last_check`; scaffolding errors escalate via
   `scaffold_hash`.
6. `progress summary`; `notes add` (`related_open` hint); goal/lesson status derives
   from `pass_or_fail` + `skip`.
7. `skip --scope … <id>` flips a `skip` column (excludes the item from derivation;
   rendered into the notebook's `managed=skip-config` cell on next sync).
8. `lesson sync` after a DB edit → re-render, learner-filled stubs preserved (3-way
   via `scaffold_hash`).
