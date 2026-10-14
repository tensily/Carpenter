# `self list` / `self remove` — output contracts

Discover and remove carpenter installs on this machine, under the `self` umbrella
(the tool itself: install · upgrade · uninstall · register · deregister · list ·
remove · link). An install is
identified by its **tag** — the binary's file stem
([adr/027](../adr/027-install-identity-from-binary-stem.md)): `carpenter` is
the stock install; a branch install `carpenter-<branch>` carries its branch
name in every surface (binary, data dir, skill). `self list` is the generic
lens over all of them; `self remove` uninstalls any of them by tag — including from a different install than the target.

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `self list` | --bin-dir (default config `bin_dir`) | `{"installs":[{"tag":"carpenter","bin":"~/.local/bin/carpenter","skill_apps":["opencode","claude-code"],"data_dir":"~/.config/carpenter"},{"tag":"carpenter-ivan-branch-installs","bin":"~/.local/bin/carpenter-ivan-branch-installs","skill_apps":["opencode"],"data_dir":"~/.config/carpenter-ivan-branch-installs"}]}` — union of `carpenter*` binaries, registered skills, and data dirs; sorted by tag |
| `self remove carpenter-ivan-topic` | tag (positional) · [--bin-dir] [--purge-config] [--yes] | `{"removed":true,"tag":"carpenter-ivan-topic","bin":"~/.local/bin/carpenter-ivan-topic","skill":[{"app":"opencode","path":"~/.config/opencode/skills/carpenter-ivan-topic/SKILL.md","removed":true}],"config_purged":false}` — same removal as `uninstall`, aimed at any install by tag; prints a removal plan and confirms on stdin unless `--yes`; `NotFound` when no skill, binary, or data dir exists |
<!-- END GENERATED -->

Discovery surfaces (union, keyed by tag): `carpenter*` binaries in `bin_dir`,
`SKILL.md` skill dirs under either agent app (`~/.config/opencode/skills/`,
`~/.claude/skills/`), and data dirs under the config base (`~/.config/`). A
row's `data_dir` is the path the tag derives to, whether or not it exists yet.

`self remove <tag>` runs the exact [adr/019](../adr/019-uninstall-semantics.md)
semantics as `uninstall` — confirmation plan + `--yes` (decline/EOF ⇒
`Conflict`, nothing removed), best-effort skill removal, binary unlink, purge
of the **removed tag's** config only with `--purge-config`; course data is
never touched — but aims at `<tag>` instead of the running install. The tag
must be kebab-case (`^[a-z0-9]+(-[a-z0-9]+)*$`, max 60) or `ValidationError`.
Removing the running install's own tag is safe (unlink keeps the inode alive).
`NotFound` only when the tag has no leg at all — no skill, no binary, and no
data dir (parity with `list`, which surfaces data-dir-only installs; removing
one reports `removed:true` with `bin:null` and `skill`
`{removed:false,reason:"not_registered"}`, and the data dir is kept).
