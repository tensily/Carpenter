# ADR-027: install identity from the binary stem

Date: 2026-10-12 · Status: Accepted

## Context

Developers work on short-lived `ivan/<topic>` branches (adr/021) and need to
run a branch build against real material — but the only install surfaces were
stock-named: binary `~/.local/bin/carpenter`, data dir `~/.config/carpenter/`,
skill `…/skills/carpenter/SKILL.md` (+ the `skill.carpenter` permission key).
Installing a branch meant clobbering the stable install; a branch with schema
changes would migrate the shared DB under the user's real courses. Multiple
installs could not coexist, and nothing about an install said *which* branch it
came from.

## Decision

The running binary's file stem is the **install identity**, resolved by
`core::store::install_name` (`CARPENTER_INSTALL_NAME` env override → sanitized
exe stem → `carpenter`), and every install surface derives from it:

| Surface | Stable (`main`) | Nightly | Branch (`ivan/foo`) |
|---|---|---|---|
| binary (command) | `carpenter` | `carpenter_nightly` | `carpenter_ivan_topic` |
| identity (tag) | `carpenter` | `carpenter-nightly` | `carpenter-ivan-topic` |
| data dir | `~/.config/carpenter/` | `~/.config/carpenter-nightly/` | `~/.config/carpenter-ivan-foo/` |
| skill dir + frontmatter | `…/skills/carpenter/` | `…/skills/carpenter-nightly/` | `…/skills/carpenter-ivan-foo/` |
| opencode permission key | `skill.carpenter` | `skill.carpenter-nightly` | `skill.carpenter-ivan-foo` |

The command name swaps `-` for `_` (`store::binary_file_name`) so the three
channel/command names are visually distinct at the shell; the identity stays
kebab because agent-app skill names forbid `_` (`slugify` maps one onto the
other bijectively for sanitized names).

- **One rule, three surfaces** — no flags, no wrapper scripts, no env wiring at
  run time. The skill's embedded binary path (`current_exe` in `render`) points
  at the tagged binary by construction, so an agent invoking the branch skill
  gets the branch CLI and its isolated data dir with no configuration.
- **Sanitization reuses `slugify`** — kebab-case, 60-char cap; the ` (deleted)`
  kernel artifact (upgrade's in-place replace) is stripped first; a stem with
  no alphanumerics falls back to `carpenter`.
- **`main` is the exception by construction, not by special case** — its stem
  *is* `carpenter`, so stock installs resolve byte-identical pre-adr/027
  paths. `scripts/install-branch.sh` installs untagged only from `main`, and
  `scripts/install.sh` ships `carpenter_nightly` for the nightly channel.
- **Branch names are a contract** — the name becomes the tag, so a branch must
  be lowercase kebab segments joined by `/`, ≤ 60 chars (else `slugify`
  truncates/collides). The `.githooks/pre-commit` hook enforces it at commit
  time; the convention is documented in `docs/README.md`.
- **Name flows as a parameter** through `skill::register/deregister/render`,
  `skill::skill_path`, and the `register`/`deregister`/`uninstall` commands
  (resolved at the `app.rs` dispatch); unit tests pin `"carpenter"` so they
  stay independent of the test binary's own stem. `uninstall` removes
  `<bin_dir>/<install_name>`, so cleanup is per-install too.

## Consequences

+ Branch installs are isolated in every way from stable — binary, DB, courses,
  bug/feature files, config, skill — and both skills can coexist in the same
  agent app, each labeled with its branch.
+ Identifying an install is trivial: the tag is visible in `ps`, `$PATH`,
  skills lists, and directory names.
+ No new CLI surface — zero howto/spec regeneration; the change is invisible
  to end users of the stock install.
− `carpenter-<branch> upgrade` must never run on a tagged install: it fetches
  stable bits that then run under a branch tag. `install-branch.sh` prints the
  caveat; the failure mode is confined to the tagged install itself.
− The stem is trust-on-first-use: renaming the binary file silently re-targets
  the data dir. Only the dev script renames binaries, so this is a
  developer-facing sharp edge, not an end-user one.

## Rejected

- **`register --as <tag>` flag** — a second source for the tag that the binary
  name already encodes; the two could disagree, and every caller had to keep
  them in sync (the "two things in sync = a bug" rule).
- **Env-only isolation (`XDG_CONFIG_HOME` per branch)** — not honored by
  `dirs` on macOS, and any invocation that skipped the env (an agent calling
  the binary directly, via the skill) silently fell back to the shared dir.
- **Wrapper script at `carpenter-<branch>` exporting env** — same hole: the
  skill embeds `current_exe`, which would resolve to the real binary, not the
  wrapper, bypassing isolation for agent-driven calls.
