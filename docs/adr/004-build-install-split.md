# ADR-004: `build`, `install`, and `upgrade` are separate commands

Date: 2026-08-08 · Status: Accepted

## Context
The requirements describe three distinct, independent operations: (1) creating a
course at any location, (2) the CLI living in a `bin/` directory so it is callable
from anywhere, and (3) rebuilding the CLI from a source checkout the user updates via
`git`, then replacing the installed binary. These are per-course data scaffolding,
one-time binary placement, and version refresh — three separate concerns.

## Decision
Three commands:
- **`carpenter build <path>`** — scaffold a course at `<path>`: creates `course.json`,
  an empty `course.db` (schema applied), and the `lessons/` directory.
- **`carpenter install [--bin-dir <p>]`** — place/symlink the carpenter binary into a
  `bin/` directory (default from config, else `~/.local/bin`) so it is callable
  globally.
- **`carpenter upgrade [--source <p>] [--bin-dir <p>] [--no-skill]`** — rebuild from a source
  checkout the user has already `git clone`d/`git pull`ed, and atomically replace the
  binary at `install`'s target. Source resolves `--source` → config `source_dir` →
  error (guidance to clone). It runs `cargo xtask build --release` from source (so the
  embedded `howto` regenerates), then writes the fresh binary to `<bin_dir>/carpenter`
  via tmp + rename. Build-only — carpenter never runs git. After replacing the binary,
  it **auto-refreshes the registered skill** (if `~/.config/opencode/skills/carpenter/SKILL.md`
  exists) via the same `write_skill()` path `register` uses, embedding the new version
  + bin path; `--no-skill` skips this.

## Consequences
+ Each command does one thing; `build` is pure (no PATH mutation), `install` is the
  canonical placement, `upgrade` reuses that target.
+ `install`/`upgrade` are idempotent and re-runnable.
+ Keeping git out of `upgrade` avoids a non-build concern; the user controls the
  source tree.
+ On open, the binary runs the initial `CREATE`-based schema (`core/db.rs`) so a new
  `course.db` is ready to use. A versioned `migrate` command for incremental schema
  changes is deferred ([adr/010](010-live-check-state.md)) — until it lands, a
  schema change requires rebuilding `course.db`.
+ `upgrade` keeps a registered skill in sync automatically (best-effort: a refresh
  failure never rolls back a successful binary upgrade); warns when the skill is
  absent so the user knows nothing was refreshed.
− Three commands to discover instead of one overloaded `build`; mitigated by the
  scraped `howto`.

*(Revision 2026-10-15: the shared `write_skill()` helper named above no longer
exists — the mechanism is `core/skill.rs`'s `render`/`register`; and since
adr/018 `upgrade` refreshes only an already-registered skill in both modes.)*
