# `plan` — output contracts (human-in-the-loop)

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `create --scope course\|lesson [--lesson <id>] --spec -` | PlanSpec | `{"id":"pl1","scope":"course","scope_id":"<slug>","title":"…","content":"{goals, links}","confirmed":false}` — draft; `links` indexes range-checked here; `--scope lesson` needs `--lesson <id>` |
| `show <id>` | — | `{"id":"pl1","scope":"course","scope_id":"<slug>","title":"…","content":"{goals, links}","confirmed_at":null}` |
| `list [--scope course\|lesson]` | — | `{"plans":[{"id":"pl1","scope":"course","scope_id":"<slug>","title":"…","confirmed":false}]}` |
| `confirm <id>` | — | `{"id":"pl1","confirmed":true,"confirmed_at":"2026-08-09T12:00:00Z","goals_created":["g1","g2"]}` — course scope creates `goals` rows |
| `update <id> --spec -` | PlanSpec | `{"id":"pl1","updated":{"id":"pl1","scope":"course","scope_id":"<slug>","title":"…","content":"{goals, links}","created_at":"2026-08-09T12:00:00Z","confirmed_at":null}}` — `Conflict` if already confirmed |
| `delete <id> --force` | — | `{"id":"pl1","deleted":true}` — `Conflict` if confirmed without `--force` |
<!-- END GENERATED -->
