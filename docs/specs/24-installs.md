# `installs` — output contracts

Discover and remove carpenter installs on this machine. An install is
identified by its **tag** — the binary's file stem
([adr/027](../adr/027-install-identity-from-binary-stem.md)): `carpenter` is
the stock install; a branch install `carpenter-<branch>` carries its branch
name in every surface (binary, data dir, skill). `installs` is the generic
lens over all of them: `list` finds what exists, `remove` uninstalls any of
them by tag — including from a different install than the target.

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `installs list` | --bin-dir (default config `bin_dir`) | `{"installs":[{"tag":"carpenter","bin":"~/.local/bin/carpenter","skill_apps":["opencode","claude-code"],"data_dir":"~/.config/carpenter"},{"tag":"carpenter-ivan-branch-installs","bin":"~/.local/bin/carpenter-ivan-branch-installs","skill_apps":["opencode"],"data_dir":"~/.config/carpenter-ivan-branch-installs"}]}` — union of `carpenter*` binaries, registered skills, and data dirs; sorted by tag |
| `installs remove carpenter-ivan-topic` | tag (positional) | `{"removed":true,"tag":"carpenter-ivan-topic","bin":"~/.local/bin/carpenter-ivan-topic","skill":[{"app":"opencode","path":"~/.config/opencode/skills/carpenter-ivan-topic/SKILL.md","removed":true}],"config_purged":false}` — same removal as `uninstall`, aimed at any install by tag; `NotFound` when absent |
<!-- END GENERATED -->

Discovery surfaces (union, keyed by tag): `carpenter*` binaries in `bin_dir`,
`SKILL.md` skill dirs under either agent app (`~/.config/opencode/skills/`,
`~/.claude/skills/`), and data dirs under the config base (`~/.config/`). A
row's `data_dir` is the path the tag derives to, whether or not it exists yet.

`remove <tag>` runs the exact [adr/019](../adr/019-uninstall-semantics.md)
semantics as `uninstall` — best-effort skill removal, binary unlink, config
purge only with `--purge-config`; course data is never touched — but aims at
`<tag>` instead of the running install. The tag must be kebab-case
(`^[a-z0-9]+(-[a-z0-9]+)*$`, max 60) or `ValidationError`. Removing the
running install's own tag is safe (unlink keeps the inode alive). A tag with
neither skill nor binary ⇒ `NotFound`.
