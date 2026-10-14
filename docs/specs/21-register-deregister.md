# `self register` / `self deregister` — output contracts

Integrate carpenter with an agent app by writing/removing a global `SKILL.md`
(see [design/15-opencode-integration.md](../design/15-opencode-integration.md)).
Global scope only.

<!-- BEGIN GENERATED -->
| cmd | input | `data` (ok) |
|-----|-------|-------------|
| `self register [--print-skill]` | no flags (opencode is the only agent app) | `{"app":"opencode","path":"~/.config/opencode/skills/carpenter/SKILL.md","version":"0.12.0","installed":true}` — writes `SKILL.md` + merges `"skill":{"carpenter":"allow"}` into `opencode.json` |
| `self register --print-skill` | --print-skill | `{"skill":"…"}` — prints the rendered `SKILL.md` bytes; no filesystem change |
| `self deregister [--yes]` | --yes skips the confirmation plan | `{"app":"opencode","path":"~/.config/opencode/skills/carpenter/SKILL.md","removed":true}` — removes `SKILL.md` (+ dir if empty) + the `carpenter` allow key; `NotFound` if absent |
<!-- END GENERATED -->

`self register` targets opencode — the only agent app (opencode into
`~/.config/opencode/skills/` + an allow entry in `opencode.json`). It is never
an interactive TTY prompt surface (an agent cannot answer one;
see [01-envelope.md](01-envelope.md) `--force` policy).

`self deregister` is destructive (it removes the machine-global skill an app
reads), so it prints a plan (skill path + permission key) to stderr and
confirms on stdin unless `--yes` — same contract as `self remove`/`self
uninstall` ([adr/019](../adr/019-uninstall-semantics.md) revision): decline or
EOF ⇒ `Conflict`, nothing removed.

The `SKILL.md` body is assembled from typed fields in `core/skill.rs` — no
template ([adr/009](../adr/009-skill-assembled-from-fields.md)). `self register
--print-skill` emits the rendered bytes; a determinism + frontmatter-validation
test gates it.
