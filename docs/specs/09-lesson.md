# `lesson` — output contracts

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `create --spec -` | LessonSpec | `{"id":"arrays-101","slug":"arrays-101","path":"<root>/courses/<slug>/lessons/01-arrays-101","counts":{"sections":1,"practice":1,"quizzes":1,"cases":3}}` — renders notebook + helper; `AlreadyExists` on duplicate slug |
| `get <id>` | — | `{"id":"arrays-101","slug":"arrays-101","title":"Arrays 101","ord":1,"status":"not_started","skip":false,"sections":[],"quizzes":[]}` — full tree (sections → practice → cases; quizzes → cases) |
| `list` | — | `{"lessons":[{"id":"arrays-101","title":"Arrays 101","ord":1,"status":"not_started","skip":false}],"errors":[]}` |
| `show <id>` | — | `{"id":"arrays-101","title":"Arrays 101","status":"not_started","skip":false,"progress":{"sections":1,"practice":1,"quizzes":1,"passing":0,"total":2}}` — live `passing`/`total` (non-skipped) |
| `update <id> --spec - --force` | LessonSpec | `{"id":"arrays-101","updated":{"id":"arrays-101","slug":"arrays-101","title":"…","ord":1,"status":"not_started","skip":false,"created_at":"2026-08-09T12:00:00Z","updated_at":"2026-08-09T12:00:00Z"}}` — `Conflict` without `--force`; re-renders notebook |
| `delete <id> --force` | — | `{"id":"arrays-101","deleted":true}` — `Conflict` without `--force` |
| `sync <id> [--force]` | — | `{"id":"arrays-101","synced":true,"conflicts":[{"id":"p1","reason":"db_changed"}]}` — `conflicts[].reason` ∈ `learner_edited`\|`db_changed` |
| `execute <id> [--allow-errors]` | — | `{"id":"arrays-101","executed":true,"cells":{"total":3,"ran":3,"errored":0},"errors":[]}` — strict (default) ⇒ `ExecuteError` on first scaffolding error; `--allow-errors` lists `errors[]` |
| `verify (<id> \| --spec -) [--timeout <SECS>]` | LessonSpec (--spec) \| — (<id>) | `{"lesson_id":"arrays-101","checked":1,"passing":1,"failing":0,"checkables":[{"owner_type":"practice","owner_id":"p1","name":"sum_array","has_solution":true,"passed":1,"total":1,"cases":[{"case_id":"c1","passed":true}]}]}` — runs each author `solution` vs its own cases; `--spec` is the pre-create key-lock, `<id>` re-verifies stored solutions |
| `new [--out <FILE>]` | — | `{"yaml":"title: …\nsections: []\n"}` — emits a YAML lesson-spec template (block scalars + `solution`); stdout, or `--out` to write |
<!-- END GENERATED -->

In the `execute` envelope, `cells.total` counts executed code cells only
(skip-config, section code, stubs, checks); markdown cells — title, section-md,
question-md — are never executed and never counted. `errors[].index` is the
full-notebook cell index (`notebook::scan_errors` enumerates all cells, markdown
included), so `total < max(errors[].index)` is expected when markdown cells
precede the errored code cell — not a miscount.
