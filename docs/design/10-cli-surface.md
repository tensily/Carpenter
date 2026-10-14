# CLI surface

Global: `--version`, `--root`, `--course/-c` (default `active_course`).

| group | commands |
|-------|----------|
| course | create list show update delete switch |
| lesson | create get list show update delete sync execute |
| plan | create show list confirm update delete |
| goal | add list update remove |
| quiz | run list show results |
| progress | show summary |
| notes | add show update resolve remove |
| bug | file list show resolve |
| feature | file list show resolve |
| config | get set |
| venv | create sync list add |
| skip | `[--scope lesson\|quiz\|practice] <id> [--off]` |
| link | register |
| build | `<path>` |
| install | `[--bin-dir <p>]` |
| upgrade | `[--source <p>] [--bin-dir <p>] [--no-skill]` |
| register | `[--print-skill]` |
| deregister | |
| howto | — |

`register`/`deregister` = agent-app skill integration (see
[15-opencode-integration.md](15-opencode-integration.md)); `link` = future CLI
registry (separate). `skip` = top-level (not under lesson/quiz) by design — it
spans scopes via `--scope` (see [specs/23-skip.md](../specs/23-skip.md), adr/011);
sets the `skip` column read by status derivation and rendered into the notebook's
`managed=skip-config` cell.

Each command's `--help` text comes from its `///` doc comment (enforced at compile
time — [adr/007](../adr/007-compile-enforced-command-docs.md)) and is the single
source scraped into `howto` and the specs input column.

Exact input/output: [specs/](../specs/) (tables will be generated from types once
`gen-specs` lands — build-order phase 2; today they are hand-maintained — see
[adr/008](../adr/008-specs-generated-from-types.md)).
