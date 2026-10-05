# opencode integration (skill)

carpenter integrates with an agent app by dropping a **`SKILL.md`** into the app's
global skills dir — not by generating custom-tool files. The skill is lean in *authored
prose* but **embeds the full `howto` manual at render time**, so the agent has the
complete, always-current command surface inline (DRY — the manual is generated once by
`xtask gen-howto` from `clap` + `docs/examples/`; the skill embeds that artifact, it
does not hand-duplicate it).

## What `register` writes
```
~/.config/opencode/skills/carpenter/SKILL.md        (global; only scope supported)
```
The skills dir is `xdg_root()` (= carpenter `config_dir.parent()`), so it tracks the OS
automatically: `~/.config/opencode` Linux, `%APPDATA%\opencode` Windows —
[design/17](17-cross-platform.md). Frontmatter is constrained by opencode — `name` + `description` (required) +
`license`/`compatibility`/`metadata`. Name must match the dir, regex
`^[a-z0-9]+(-[a-z0-9]+)*$` → `carpenter` is valid.

The body is **assembled from typed fields in `core/skill.rs`** — there is no template
file and no monolithic prose block ([adr/009](../adr/009-skill-assembled-from-fields.md)).
`render()` composes frontmatter + sections via `format!`:

| field | source |
|-------|--------|
| `NAME` | const `"carpenter"` |
| `DESCRIPTION` | const — the frontmatter matcher (authored once) |
| `WHAT_THIS_IS` / `INTAKE` / `WORKFLOW` / `WALKTHROUGH` / `PEDAGOGY` | const — the only authored prose (exists nowhere else) |
| `MANUAL` | `crate::manual::MANUAL` (the generated howto) — inlined verbatim under `## Command manual` at render time, H1 stripped |
| `version` | `CARGO_PKG_VERSION` |
| `bin` | `std::env::current_exe` |

`carpenter register --print-skill` emits the rendered bytes for inspection/CI. A
`#[test]` asserts render determinism (re-render byte-equal) + frontmatter validation
(`name` matches `^[a-z0-9]+(-[a-z0-9]+)*$`, required fields present).

## Permission entry
`register` always merges into the **global** config `~/.config/opencode/opencode.json`:
```json
{ "permission": { "skill": { "carpenter": "allow" } } }
```
so the skill loads without prompting. `deregister` removes only the `carpenter` key
under `skill` (leaves the rest of the file intact). Merge, never overwrite.

## Multi-app selector
`--app <name>` (default `opencode`; never an interactive prompt — an agent cannot
answer one):
| app | skills dir | anchor | status |
|-----|-----------|--------|--------|
| `opencode` | `~/.config/opencode/skills/` | XDG root | implemented |
| `claude-code` | `~/.claude/skills/` | home dir | implemented |
| `agents` | `~/.agents/skills/` | — | not yet supported |

Both implemented apps use the same rendered `SKILL.md`; they differ in anchor
(`skill_path` takes the XDG root **and** the home dir) and in permissions:
opencode needs the `opencode.json` allow entry, claude code auto-discovers
`~/.claude/skills/` — no permission merge. Adding `agents` is one more match arm.
`upgrade`/`uninstall` fan out over all apps but touch only **registered** ones
(skill file present).

## Commands
| cmd | behavior |
|-----|----------|
| `register [--app opencode] [--print-skill]` | write `SKILL.md` (idempotent) + merge the allow entry (`--print-skill` writes the rendered bytes to stdout instead, no filesystem change) |
| `deregister [--app opencode]` | remove `SKILL.md` (+ dir if empty) + remove the allow key |

See [specs/21-register-deregister.md](../specs/21-register-deregister.md) for the
envelopes. `link` (future CLI registry) is a separate concern — see
[specs/17-link.md](../specs/17-link.md). Rationale:
[adr/006](../adr/006-skill-integration.md).

## `upgrade` auto-refreshes the skill
After replacing the binary, `upgrade` updates the skill — how depends on the
mode (adr/018): **source mode** (`--source` / config `source_dir`) refreshes
only if the skill file is present (`~/.config/opencode/skills/carpenter/SKILL.md`),
warns with `reason:"not_registered"` otherwise (the exact warning string is
single-sourced in [specs/18-build-install-upgrade.md](../specs/18-build-install-upgrade.md);
this doc does not duplicate it); **release mode** (no flags — the default)
re-registers unconditionally, mirroring `scripts/install.sh` (installer parity).
Both re-render via the same `write_skill()` path `register` uses — embedding the
**new** version + bin path. Refresh is best-effort (a failure never rolls back
the binary upgrade). `--no-skill` skips it (`skill:null`).
See [specs/18-build-install-upgrade.md](../specs/18-build-install-upgrade.md),
[adr/004](../adr/004-build-install-split.md), [adr/009](../adr/009-skill-assembled-from-fields.md).
